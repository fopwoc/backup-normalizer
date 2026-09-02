use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use chrono::Utc;
use chrono_tz::Tz;
use tracing::{debug, error, info, warn};

use crate::error::Error;
use crate::policy::{Action, RetentionPolicy};
use crate::scanner::Scanner;

#[derive(Clone, Debug)]
pub(crate) struct Config {
    pub(crate) path: PathBuf,
    pub(crate) filename_format: String,
    pub(crate) timezone: Tz,
    pub(crate) policy: RetentionPolicy,
    pub(crate) min_age: Duration,
    pub(crate) scan_every: Duration,
    pub(crate) apply: bool,
}

pub(crate) struct Application {
    config: Config,
    scanner: Scanner,
}

impl Application {
    pub(crate) fn new(config: Config) -> Self {
        Self {
            config,
            scanner: Scanner::default(),
        }
    }

    pub(crate) fn run(mut self) -> Result<(), Error> {
        Scanner::validate_directory(&self.config.path)?;
        info!(
            version = env!("BUILD_VERSION"),
            path = %self.config.path.display(),
            timezone = %self.config.timezone,
            apply = self.config.apply,
            "backup normalizer started"
        );

        loop {
            if let Err(error) = self.scan_once() {
                error!(%error, "normalization scan failed; no further files were removed by this scan");
            }
            thread::sleep(self.config.scan_every);
        }
    }

    fn scan_once(&mut self) -> Result<(), Error> {
        debug!("starting normalization scan");
        let backups = self.scanner.scan(
            &self.config.path,
            &self.config.filename_format,
            self.config.timezone,
        )?;
        let decisions = self.config.policy.plan(
            Utc::now(),
            self.config.min_age,
            backups,
            self.config.timezone,
        );
        let mut kept = 0usize;
        let mut redundant = 0usize;
        let mut removed = 0usize;

        for decision in decisions {
            match decision.action {
                Action::Keep(reason) => {
                    kept += 1;
                    debug!(path = %decision.backup.path.display(), ?reason, "keeping backup");
                }
                Action::Delete => {
                    redundant += 1;
                    if self.config.apply {
                        fs::remove_file(&decision.backup.path).map_err(|source| {
                            Error::RemoveBackup {
                                path: decision.backup.path.clone(),
                                source,
                            }
                        })?;
                        removed += 1;
                        info!(path = %decision.backup.path.display(), "removed redundant backup");
                    } else {
                        warn!(path = %decision.backup.path.display(), "would remove redundant backup; dry-run mode is active");
                    }
                }
            }
        }

        info!(kept, redundant, removed, "normalization scan completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::str::FromStr;
    use std::time::Duration;

    use chrono::{TimeDelta, Utc};
    use chrono_tz::UTC;
    use tempfile::tempdir;

    use crate::period::RetentionPeriod;
    use crate::policy::RetentionPolicy;

    use super::{Application, Config};

    #[test]
    fn apply_removes_only_stable_redundant_automatic_backups() {
        let directory = tempdir().unwrap();
        let now = Utc::now();
        let recent = now.format("%Y-%m-%d-%H-%M-%S.zip").to_string();
        let old_a = (now - TimeDelta::days(400))
            .format("%Y-%m-%d-%H-%M-%S.zip")
            .to_string();
        let old_b = (now - TimeDelta::days(401))
            .format("%Y-%m-%d-%H-%M-%S.zip")
            .to_string();
        let manual = "before-new-base.zip";
        for name in [&recent, &old_a, &old_b, manual] {
            fs::write(directory.path().join(name), b"backup").unwrap();
        }

        let mut application = Application::new(config(directory.path().to_owned(), true));
        application.scan_once().unwrap();
        assert!(directory.path().join(&old_a).exists());
        assert!(directory.path().join(&old_b).exists());

        application.scan_once().unwrap();
        assert!(directory.path().join(&recent).exists());
        assert!(!directory.path().join(&old_a).exists());
        assert!(!directory.path().join(&old_b).exists());
        assert!(directory.path().join(manual).exists());
    }

    #[test]
    fn dry_run_never_removes_redundant_backups() {
        let directory = tempdir().unwrap();
        let recent = Utc::now().format("%Y-%m-%d-%H-%M-%S.zip").to_string();
        let old = (Utc::now() - TimeDelta::days(400))
            .format("%Y-%m-%d-%H-%M-%S.zip")
            .to_string();
        fs::write(directory.path().join(recent), b"backup").unwrap();
        fs::write(directory.path().join(&old), b"backup").unwrap();

        let mut application = Application::new(config(directory.path().to_owned(), false));
        application.scan_once().unwrap();
        application.scan_once().unwrap();

        assert!(directory.path().join(old).exists());
    }

    fn config(path: std::path::PathBuf, apply: bool) -> Config {
        Config {
            path,
            filename_format: "%Y-%m-%d-%H-%M-%S.zip".to_owned(),
            timezone: UTC,
            policy: RetentionPolicy::new(
                RetentionPeriod::from_str("12h").unwrap(),
                RetentionPeriod::from_str("14d").unwrap(),
                RetentionPeriod::from_str("12w").unwrap(),
                RetentionPeriod::from_str("12mo").unwrap(),
            )
            .unwrap(),
            min_age: Duration::ZERO,
            scan_every: Duration::from_secs(3600),
            apply,
        }
    }
}
