use std::io;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ConfigError {
    #[error("scan interval must be greater than zero")]
    ZeroScanInterval,
    #[error("retention horizons must increase in the order all, daily, weekly, monthly")]
    RetentionOrder,
}

#[derive(Debug, Error)]
pub(crate) enum PeriodParseError {
    #[error("retention period `{0}` is missing a unit; use h, d, w, mo, or y")]
    MissingUnit(String),
    #[error("retention period `{0}` has an invalid amount")]
    InvalidAmount(String),
    #[error("retention periods must be greater than zero")]
    Zero,
    #[error("unsupported retention unit `{0}`; use h, d, w, mo, or y")]
    UnsupportedUnit(String),
}

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("backup directory `{path}` cannot be inspected: {source}")]
    InspectDirectory { path: PathBuf, source: io::Error },
    #[error("backup path `{0}` must be a real directory, not a symlink")]
    InvalidDirectory(PathBuf),
    #[error("backup directory `{path}` cannot be read: {source}")]
    ReadDirectory { path: PathBuf, source: io::Error },
    #[error("directory entry in `{path}` cannot be read: {source}")]
    ReadEntry { path: PathBuf, source: io::Error },
    #[error("metadata for `{path}` cannot be read: {source}")]
    ReadMetadata { path: PathBuf, source: io::Error },
    #[error("redundant backup `{path}` cannot be removed: {source}")]
    RemoveBackup { path: PathBuf, source: io::Error },
}
