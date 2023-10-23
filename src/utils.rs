use std::collections::BTreeMap;

use semver::VersionReq;

use crate::{errors::CustomErrors, serde::Version, version::parse};

pub(crate) fn find_version(
    src: BTreeMap<String, Version>,
    pre: Option<Vec<VersionReq>>,
    lts: Option<&str>,
) -> Result<Option<Version>, CustomErrors> {
    if let Some(lts) = lts {
        if let Some(v) = src.get(lts) {
            return Ok(Some(v.to_owned()));
        } else {
            return Ok(None);
        }
    }

    let mut found = None;
    let pre = pre.expect("pre can't be optional if lts is None");

    for (tag, v) in src.iter().rev() {
        let parsed_v =
            semver::Version::parse(tag).map_err(|e| CustomErrors::VersionParse(e.to_string()))?;
        let matched = pre.iter().find(|req| req.matches(&parsed_v)).is_some();
        if !matched && found.is_some() {
            break;
        }

        if matched {
            found = Some(v.clone());
        }
    }

    Ok(found)
}

/// Split a package string into a tuple of package name and version
pub(crate) fn split_package_string(
    package: String,
) -> Result<(String, Option<Vec<VersionReq>>), CustomErrors> {
    let mut splitted = package.split('@').collect::<Vec<&str>>();
    if splitted.len() > 3 {
        return Err(CustomErrors::PackageSplit(format!(
            "package name can only contains a maximum of 2 '@'. Found {}",
            splitted.len() - 1
        )));
    } else if splitted.len() == 3 {
        splitted.remove(0);
    }

    let version = if splitted[1] == "latest" {
        None
    } else {
        Some(parse(splitted[1])?)
    };

    return Ok((splitted[0].to_string(), version));
}
