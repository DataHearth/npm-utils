use std::io;

use nom::{
    branch::alt,
    bytes::complete::{take_while, take_while_m_n},
    character,
    combinator::map_res,
    sequence::{terminated, tuple},
    IResult, Parser,
};
use semver::{Comparator, Op, Prerelease, VersionReq};

const OPERATOR_LIST: [char; 5] = ['<', '>', '=', '~', '^'];

/// Map string to corresponding `semver::Op` enum
fn from_operator(input: &str) -> Result<Op, io::Error> {
    match input {
        "<" => Ok(Op::Less),
        "<=" => Ok(Op::LessEq),
        ">" => Ok(Op::Greater),
        ">=" => Ok(Op::GreaterEq),
        "=" => Ok(Op::Exact),
        "~" => Ok(Op::Tilde),
        "^" => Ok(Op::Caret),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid operator",
        )),
    }
}

/// Parse a range operator included in `OPERATOR_LIST`
fn parse_range(input: &str) -> IResult<&str, Op> {
    let res: IResult<&str, Op> = map_res(
        take_while_m_n(0, 2, |c| OPERATOR_LIST.contains(&c)),
        from_operator,
    )
    .parse(input);

    match res {
        Ok(v) => Ok((v.0.trim_start(), v.1)),
        Err(e) => {
            // TODO: remove unwrap and use a proper error handling
            if input.chars().next().unwrap().is_digit(10) {
                Ok((input.trim_start(), Op::Exact))
            } else {
                Err(e)
            }
        }
    }
}

/// Parse a version string into `(major, minor, patch)`
fn parse_version(input: &str) -> IResult<&str, (u64, Option<u64>, Option<u64>)> {
    let (input, (major, minor, patch)) = map_res::<_, _, _, _, std::num::ParseIntError, _, _>(
        alt((
            tuple((
                terminated(character::complete::digit0, character::complete::char('.')),
                terminated(character::complete::digit0, character::complete::char('.')),
                character::complete::digit0,
            )),
            map_res::<_, _, _, _, nom::error::Error<&str>, _, _>(
                tuple((
                    terminated(character::complete::digit0, character::complete::char('.')),
                    character::complete::digit0,
                )),
                |(major, minor)| Ok((major, minor, "")),
            ),
            map_res::<_, _, _, _, nom::error::Error<&str>, _, _>(
                character::complete::digit0,
                |major| Ok((major, "", "")),
            ),
        )),
        |(major, minor, patch): (&str, &str, &str)| {
            let major = major.parse::<u64>()?;
            let minor = if !minor.is_empty() {
                Some(minor.parse::<u64>()?)
            } else {
                None
            };
            let patch = if !patch.is_empty() {
                Some(patch.parse::<u64>()?)
            } else {
                None
            };

            Ok((major, minor, patch))
        },
    )(input)?;

    Ok((input.trim_start(), (major, minor, patch)))
}

fn parse_pre(input: &str) -> IResult<&str, Prerelease> {
    let (input, pre) = take_while(|c: char| !c.is_whitespace())(input.trim_start_matches('-'))?;

    Ok((
        input.trim_start(),
        if !pre.is_empty() {
            Prerelease::new(pre).unwrap()
        } else {
            Prerelease::EMPTY
        },
    ))
}

/// Parse a version string into `semver::VersionReq`
pub fn parse_version_req(input: &str) -> IResult<&str, VersionReq> {
    let (input, op) = parse_range(input)?;
    let (input, (major, minor, patch)) = parse_version(input)?;
    let (input, pre) = parse_pre(input)?;

    Ok((
        input,
        VersionReq {
            comparators: vec![Comparator {
                major,
                minor,
                patch,
                op,
                pre,
            }],
        },
    ))
}
