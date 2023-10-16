use std::num::ParseIntError;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while, take_while_m_n},
    character,
    combinator::map_res,
    error::{self, context, convert_error, VerboseError},
    multi::many0,
    sequence::{terminated, tuple},
};
use semver::{Comparator, Op, Prerelease, VersionReq};

const OPERATOR_LIST: [char; 5] = ['<', '>', '=', '~', '^'];

type IResult<R, O> = nom::IResult<R, O, VerboseError<R>>;

/// Map string to corresponding `semver::Op`
fn from_operator(input: &str) -> Result<Op, nom::Err<error::Error<&str>>> {
    if input.is_empty() {
        return Ok(Op::Exact);
    }

    match input {
        "<" => Ok(Op::Less),
        "<=" => Ok(Op::LessEq),
        ">" => Ok(Op::Greater),
        ">=" => Ok(Op::GreaterEq),
        "=" => Ok(Op::Exact),
        "~" => Ok(Op::Tilde),
        "^" => Ok(Op::Caret),
        _ => panic!("parsing invalid operator"),
    }
}

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

fn from_version(
    version: (&str, &str, &str),
) -> Result<(u64, Option<i64>, Option<i64>), ParseIntError> {
    let (major, minor, patch) = version;

    let major = major.parse::<u64>()?;
    let minor = if !minor.is_empty() {
        Some(
            if minor == "*" || minor.chars().all(|v| v.is_alphabetic()) {
                -1
            } else {
                minor.parse::<i64>()?
            },
        )
    } else {
        None
    };
    let patch = if !patch.is_empty() {
        Some(
            if patch == "*" || patch.chars().all(|v| v.is_alphabetic()) {
                -1
            } else {
                patch.parse::<i64>()?
            },
        )
    } else {
        None
    };

    Ok((major, minor, patch))
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
fn parse_comparator(input: &str) -> IResult<&str, Comparator> {
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

pub fn parse(input: &str) -> Result<Vec<VersionReq>, crate::errors::Error> {
    let original = input;

    if input == "*" {
        return Ok(vec![VersionReq::STAR]);
    }

    let mut reqs = vec![];
    let mut req = VersionReq::default();

    let (input, result) = context(
        "semver",
        many0(alt((
            map_res(tag("||"), |_| -> Result<(bool, Option<Comparator>), &str> {
                Ok((true, None))
            }),
            map_res(
                parse_comparator,
                |v| -> Result<(bool, Option<Comparator>), &str> { Ok((false, Some(v))) },
            ),
        ))),
    )(input.trim_start())
    .map_err(|e| {
        crate::errors::Error::VersionParse(convert_error(
            input,
            match e {
                nom::Err::Incomplete(_) => VerboseError {
                    errors: vec![(input, error::VerboseErrorKind::Context("incomplete input"))],
                },
                nom::Err::Error(e) => e,
                nom::Err::Failure(e) => e,
            },
        ))
    })?;

    if !input.is_empty() {
        return Err(crate::errors::Error::VersionParse(format!(
            "trailing characters in version (\"{original}\"): {input}"
        )));
    }

    for (new, comp) in result {
        if !new {
            req.comparators.push(comp.clone().unwrap());
            continue;
        }

        reqs.push(req);
        req = VersionReq::default();
    }

    if req.comparators.len() > 0 {
        reqs.push(req);
    }

    Ok(reqs)
}
