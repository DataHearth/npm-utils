use anyhow::{anyhow, Result};
use semver::VersionReq;

pub fn parser_multi_requirements(req: &str) -> Result<Vec<VersionReq>> {
    let mut splitted = req.split_whitespace();

    let mut requirements = vec![];
    let mut unfinished_requirement = String::new();

    loop {
        let v = splitted.next();
        if v.is_none() {
            if unfinished_requirement.len() > 0 {
                requirements.push(semver::VersionReq::parse(&unfinished_requirement)?);
            }

            return Ok(requirements);
        }

        match v.unwrap() {
            "||" => {
                if unfinished_requirement.len() == 0 {
                    return Err(anyhow!(
                        "requirement cannot start or chain operators with '||'"
                    ));
                }

                requirements.push(VersionReq::parse(&unfinished_requirement)?);
                unfinished_requirement = String::new();
            }
            "<" | ">" | "<=" | ">=" => {
                if unfinished_requirement.len() == 0 {
                    unfinished_requirement.push_str(v.unwrap());
                } else {
                    unfinished_requirement.push_str(&format!(", {}", v.unwrap()));
                }
            }
            "-" => {
                unfinished_requirement = format!(">={}, <=", unfinished_requirement);
            }
            _ => {
                unfinished_requirement.push_str(v.unwrap());
            }
        }
    }
}

pub fn display_multi(requirements: Vec<VersionReq>) -> String {
    let mut output = String::new();
    for req in requirements {
        output.push_str(&format!("{},", req.to_string()));
    }

    output
}
