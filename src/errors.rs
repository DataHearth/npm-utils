#[derive(thiserror::Error, Debug)]
pub(crate) enum CustomErrors {
    #[error("failed to parse version: {}", .0)]
    VersionParse(String),

    #[error("failed to parse package string: {}", .0)]
    PackageSplit(String),

    #[error("failed to parse package.json: {}", .0)]
    PackageJsonParse(String),

    #[error("failed to create HTTP client: {}", .0)]
    HttpClient(String),

    #[error("failed to parse HTTP header to create client: {}", .0)]
    HttpHeaderParse(String),

    #[error("failed to fetch package manifest: {}", .0)]
    PackageManifestFetch(String),

    #[error("failed to parse {} response: {}", .0, .1)]
    BodyParse(String, String),

    #[error("Filesystem error: {}", .0)]
    Fs(String),

    #[error("Version error: {}", .0)]
    Version(String),

    #[error("Error: {}", .0)]
    Global(String),
}
