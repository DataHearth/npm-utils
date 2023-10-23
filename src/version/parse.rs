use nom::{
    branch::alt,
    bytes::complete::{tag, take_while, take_while_m_n},
    character,
    combinator::map_res,
    error::{context, VerboseError},
    sequence::{terminated, tuple},
};
use semver::{Comparator, Op, Prerelease};

use super::from::{from_operator, from_version};

type IResult<R, O> = nom::IResult<R, O, VerboseError<R>>;
const OPERATOR_LIST: [char; 5] = ['<', '>', '=', '~', '^'];

/// Parse a range operator into `semver::Op`
fn parse_range(input: &str) -> IResult<&str, Op> {
    context(
        "range-operator",
        map_res(
            take_while_m_n(0, 2, |c| OPERATOR_LIST.contains(&c)),
            from_operator,
        ),
    )(input)
}

fn wildcard_or_digit(input: &str) -> IResult<&str, &str> {
    context(
        "final-version-number",
        alt((tag("*"), character::complete::alphanumeric1)),
    )(input)
}

fn terminated_number(input: &str) -> IResult<&str, &str> {
    context(
        "intermediate-version-number",
        terminated(wildcard_or_digit, character::complete::char('.')),
    )(input)
}

/// Parse a version string into `(major, minor, patch)`
fn parse_version(input: &str) -> IResult<&str, (u64, Option<i64>, Option<i64>)> {
    let (input, (major, minor, patch)) = map_res(
        alt((
            context(
                "complete",
                tuple((terminated_number, terminated_number, wildcard_or_digit)),
            ),
            context(
                "major-minor",
                map_res(
                    tuple((terminated_number, wildcard_or_digit)),
                    |(major, minor)| -> Result<(&str, &str, &str), &str> { Ok((major, minor, "")) },
                ),
            ),
            context(
                "major",
                map_res(
                    character::complete::digit1,
                    |major| -> Result<(&str, &str, &str), &str> { Ok((major, "", "")) },
                ),
            ),
        )),
        from_version,
    )(input)?;

    Ok((input, (major, minor, patch)))
}

/// Parse a prerelease string into `semver::Prerelease`
fn parse_pre(input: &str) -> IResult<&str, Prerelease> {
    let (input, pre) = if input.starts_with('-') {
        take_while(|c: char| !c.is_whitespace())(input.trim_start_matches('-'))?
    } else {
        (input, "")
    };

    Ok((
        input,
        if !pre.is_empty() {
            Prerelease::new(pre).unwrap()
        } else {
            Prerelease::EMPTY
        },
    ))
}

/// Parse a version string into `semver::VersionReq`
pub(super) fn parse_comparator(input: &str) -> IResult<&str, Comparator> {
    let (input, mut op) = context("range-operator", parse_range)(input)?;
    let (input, (major, minor, patch)) = context("version", parse_version)(input.trim_start())?;
    let (input, pre) = context("pre-release", parse_pre)(input.trim_start())?;
    let minor = if let Some(minor) = minor {
        if minor == -1 {
            op = Op::Wildcard;
            None
        } else {
            Some(minor as u64)
        }
    } else {
        None
    };
    let patch = if let Some(patch) = patch {
        if patch == -1 {
            op = Op::Wildcard;
            None
        } else {
            Some(patch as u64)
        }
    } else {
        None
    };

    Ok((
        input,
        Comparator {
            major,
            minor,
            patch,
            op,
            pre,
        },
    ))
}
