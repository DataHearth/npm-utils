#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to parse version: {}", .0)]
    VersionParse(String),
}
