use std::path::PathBuf;
use std::time::SystemTime;

use chrono::{DateTime, LocalResult, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;

#[derive(Clone, Debug)]
pub(crate) struct BackupFile {
    pub(crate) path: PathBuf,
    pub(crate) timestamp: DateTime<Utc>,
    pub(crate) modified: SystemTime,
    pub(crate) stable: bool,
}

impl BackupFile {
    pub(crate) fn parse_timestamp(name: &str, format: &str, timezone: Tz) -> Option<DateTime<Utc>> {
        let local = NaiveDateTime::parse_from_str(name, format).ok()?;
        match timezone.from_local_datetime(&local) {
            LocalResult::Single(timestamp) => Some(timestamp.with_timezone(&Utc)),
            LocalResult::Ambiguous(first, _) => Some(first.with_timezone(&Utc)),
            LocalResult::None => None,
        }
    }
}
