use anyhow::{anyhow, Result};
use nom::{bytes::complete::take_while_m_n, combinator::map_res, IResult, Parser};
use semver::{Op, VersionReq};

fn from_operator(input: &str) -> Result<Op> {
    match input {
        "<" => Ok(Op::Less),
        "<=" => Ok(Op::LessEq),
        ">" => Ok(Op::Greater),
        ">=" => Ok(Op::GreaterEq),
        "=" => Ok(Op::Exact),
        "~" => Ok(Op::Tilde),
        "^" => Ok(Op::Caret),
        _ => Err(anyhow!("invalid operator")),
    }
}

fn check_operator(c: char) -> bool {
    todo!()
}

fn parse_range(input: &str) -> IResult<&str, Op> {
    map_res(take_while_m_n(0, 2, check_operator), from_operator).parse(input)
}

pub fn parse(input: &str) -> IResult<&str, VersionReq> {
    todo!()
}
