use std::num::ParseIntError;

use nom::error;
use semver::Op;

pub(super) fn from_version(
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

/// Map string to corresponding `semver::Op`
pub(super) fn from_operator(input: &str) -> Result<Op, nom::Err<error::Error<&str>>> {
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
