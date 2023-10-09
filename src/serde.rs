use std::collections::BTreeMap;
use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
    pub name: String,
    pub dependencies: HashMap<String, String>,
    pub dev_dependencies: HashMap<String, String>,
    pub peer_dependencies: HashMap<String, String>,
    pub optional_dependencies: HashMap<String, String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageRsp {
    #[serde(rename = "dist-tags")]
    pub dist_tags: HashMap<String, String>,
    pub versions: BTreeMap<String, Version>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Version {
    pub name: String,
    pub version: String,
    pub dist: Dist,
    #[serde(default = "HashMap::new")]
    pub dependencies: HashMap<String, String>,
    #[serde(rename = "devDependencies", default = "HashMap::new")]
    pub dev_dependencies: HashMap<String, String>,
    #[serde(rename = "peerDependencies", default = "HashMap::new")]
    pub peer_dependencies: HashMap<String, String>,
    #[serde(rename = "optionalDependencies", default = "HashMap::new")]
    pub optional_dependencies: HashMap<String, String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dist {
    pub tarball: String,
    pub shasum: String,
}
