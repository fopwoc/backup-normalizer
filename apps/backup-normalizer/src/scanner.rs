use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono_tz::Tz;
use tracing::{debug, warn};

use crate::backup_file::BackupFile;
use crate::error::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Fingerprint {
    length: u64,
    modified: SystemTime,
}

#[derive(Debug, Default)]
pub(crate) struct Scanner {
    previous: HashMap<PathBuf, Fingerprint>,
}

impl Scanner {
    pub(crate) fn validate_directory(path: &Path) -> Result<(), Error> {
        let metadata = fs::symlink_metadata(path).map_err(|source| Error::InspectDirectory {
            path: path.to_owned(),
            source,
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(Error::InvalidDirectory(path.to_owned()));
        }
        Ok(())
    }

    pub(crate) fn scan(
        &mut self,
        path: &Path,
        filename_format: &str,
        timezone: Tz,
    ) -> Result<Vec<BackupFile>, Error> {
        let entries = fs::read_dir(path).map_err(|source| Error::ReadDirectory {
            path: path.to_owned(),
            source,
        })?;
        let mut current = HashMap::new();
        let mut backups = Vec::new();
        let mut seen = HashSet::new();

        for entry in entries {
            let entry = entry.map_err(|source| Error::ReadEntry {
                path: path.to_owned(),
                source,
            })?;
            let entry_path = entry.path();
            let file_type = entry.file_type().map_err(|source| Error::ReadMetadata {
                path: entry_path.clone(),
                source,
            })?;
            if !file_type.is_file() || file_type.is_symlink() {
                debug!(path = %entry_path.display(), "ignoring non-regular directory entry");
                continue;
            }

            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                warn!(path = %entry_path.display(), "ignoring non-UTF-8 filename");
                continue;
            };
            let Some(timestamp) = BackupFile::parse_timestamp(&name, filename_format, timezone)
            else {
                debug!(path = %entry_path.display(), "ignoring filename outside the configured format");
                continue;
            };

            let metadata = entry.metadata().map_err(|source| Error::ReadMetadata {
                path: entry_path.clone(),
                source,
            })?;
            let modified = metadata.modified().map_err(|source| Error::ReadMetadata {
                path: entry_path.clone(),
                source,
            })?;
            let fingerprint = Fingerprint {
                length: metadata.len(),
                modified,
            };
            let stable = self.previous.get(&entry_path) == Some(&fingerprint);
            current.insert(entry_path.clone(), fingerprint);
            seen.insert(entry_path.clone());
            backups.push(BackupFile {
                path: entry_path,
                timestamp,
                modified,
                stable,
            });
        }

        self.previous.retain(|path, _| seen.contains(path));
        self.previous.extend(current);
        Ok(backups)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono_tz::UTC;
    use tempfile::tempdir;

    use super::Scanner;

    #[test]
    fn requires_two_unchanged_observations() {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join("2026-09-02-12-00-00.zip"), b"backup").unwrap();
        fs::write(directory.path().join("manual-milestone.zip"), b"manual").unwrap();

        let mut scanner = Scanner::default();
        let first = scanner
            .scan(directory.path(), "%Y-%m-%d-%H-%M-%S.zip", UTC)
            .unwrap();
        let second = scanner
            .scan(directory.path(), "%Y-%m-%d-%H-%M-%S.zip", UTC)
            .unwrap();

        assert_eq!(first.len(), 1);
        assert!(!first[0].stable);
        assert_eq!(second.len(), 1);
        assert!(second[0].stable);
    }

    #[cfg(unix)]
    #[test]
    fn ignores_symlinks() {
        use std::os::unix::fs::symlink;

        let directory = tempdir().unwrap();
        let outside = directory.path().join("outside.zip");
        fs::write(&outside, b"outside").unwrap();
        symlink(&outside, directory.path().join("2026-09-02-12-00-00.zip")).unwrap();

        let mut scanner = Scanner::default();
        let backups = scanner
            .scan(directory.path(), "%Y-%m-%d-%H-%M-%S.zip", UTC)
            .unwrap();
        assert!(backups.is_empty());
    }
}
