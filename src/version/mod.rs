use nom::{
    branch::alt,
    bytes::complete::tag,
    combinator::map_res,
    error::{self, context, convert_error, VerboseError},
    multi::many0,
};
use semver::{Comparator, VersionReq};

use self::parse::parse_comparator;

mod from;
mod parse;

pub(crate) fn parse(input: &str) -> Result<Vec<VersionReq>, crate::errors::CustomErrors> {
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
        crate::errors::CustomErrors::VersionParse(convert_error(
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
        return Err(crate::errors::CustomErrors::VersionParse(format!(
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
