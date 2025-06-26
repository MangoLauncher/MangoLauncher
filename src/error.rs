use thiserror::Error;
use std::time::SystemTimeError;
use zip::result::ZipError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] ZipError),

    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("Walkdir error: {0}")]
    Walkdir(#[from] walkdir::Error),

    #[error("SystemTime error: {0}")]
    SystemTime(#[from] SystemTimeError),

    #[error("Instance error: {0}")]
    Instance(String),

    #[error("Version error: {0}")]
    Version(String),

    #[error("Profile error: {0}")]
    Profile(String),

    #[error("Launch error: {0}")]
    Launch(String),

    #[error("Java error: {0}")]
    Java(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Settings error: {0}")]
    Settings(String),

    #[error("Asset error: {0}")]
    Asset(String),

    #[error("Mod error: {0}")]
    Mod(String),

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("Unknown error: {0}")]
    Unknown(String),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::Other(err.to_string())
    }
} 