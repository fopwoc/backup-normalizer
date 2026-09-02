use std::cmp::Reverse;
use std::collections::HashSet;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Datelike, Utc};
use chrono_tz::Tz;

use crate::backup_file::BackupFile;
use crate::error::ConfigError;
use crate::period::RetentionPeriod;

#[derive(Clone, Debug)]
pub(crate) struct RetentionPolicy {
    keep_all_for: RetentionPeriod,
    keep_daily_for: RetentionPeriod,
    keep_weekly_for: RetentionPeriod,
    keep_monthly_for: RetentionPeriod,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum Bucket {
    Day(i32, u32),
    Week(i32, u32),
    Month(i32, u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum KeepReason {
    Newest,
    Recent,
    Daily,
    Weekly,
    Monthly,
    TooYoung,
    Unstable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Action {
    Keep(KeepReason),
    Delete,
}

#[derive(Clone, Debug)]
pub(crate) struct Decision {
    pub(crate) backup: BackupFile,
    pub(crate) action: Action,
}

impl RetentionPolicy {
    pub(crate) fn new(
        keep_all_for: RetentionPeriod,
        keep_daily_for: RetentionPeriod,
        keep_weekly_for: RetentionPeriod,
        keep_monthly_for: RetentionPeriod,
    ) -> Result<Self, ConfigError> {
        let horizons = [
            keep_all_for.approximate_hours(),
            keep_daily_for.approximate_hours(),
            keep_weekly_for.approximate_hours(),
            keep_monthly_for.approximate_hours(),
        ];
        if !horizons.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(ConfigError::RetentionOrder);
        }
        Ok(Self {
            keep_all_for,
            keep_daily_for,
            keep_weekly_for,
            keep_monthly_for,
        })
    }

    pub(crate) fn plan(
        &self,
        now: DateTime<Utc>,
        min_age: Duration,
        mut backups: Vec<BackupFile>,
        timezone: Tz,
    ) -> Vec<Decision> {
        backups.sort_by_key(|backup| Reverse(backup.timestamp));
        let local_now = now.with_timezone(&timezone);
        let all_cutoff = self.keep_all_for.cutoff(local_now);
        let daily_cutoff = self.keep_daily_for.cutoff(local_now);
        let weekly_cutoff = self.keep_weekly_for.cutoff(local_now);
        let monthly_cutoff = self.keep_monthly_for.cutoff(local_now);
        let now_system = SystemTime::from(now);
        let mut buckets = HashSet::new();

        backups
            .into_iter()
            .enumerate()
            .map(|(index, backup)| {
                let local_timestamp = backup.timestamp.with_timezone(&timezone);
                let tier_reason = if all_cutoff.is_none_or(|cutoff| local_timestamp >= cutoff) {
                    Some(KeepReason::Recent)
                } else if daily_cutoff.is_none_or(|cutoff| local_timestamp >= cutoff) {
                    buckets
                        .insert(Bucket::Day(
                            local_timestamp.year(),
                            local_timestamp.ordinal(),
                        ))
                        .then_some(KeepReason::Daily)
                } else if weekly_cutoff.is_none_or(|cutoff| local_timestamp >= cutoff) {
                    let week = local_timestamp.iso_week();
                    buckets
                        .insert(Bucket::Week(week.year(), week.week()))
                        .then_some(KeepReason::Weekly)
                } else if monthly_cutoff.is_none_or(|cutoff| local_timestamp >= cutoff) {
                    buckets
                        .insert(Bucket::Month(
                            local_timestamp.year(),
                            local_timestamp.month(),
                        ))
                        .then_some(KeepReason::Monthly)
                } else {
                    None
                };
                let retained = if index == 0 {
                    Some(KeepReason::Newest)
                } else {
                    tier_reason
                };

                let action = retained.map_or_else(
                    || {
                        if is_too_young(now_system, backup.modified, min_age) {
                            Action::Keep(KeepReason::TooYoung)
                        } else if !backup.stable {
                            Action::Keep(KeepReason::Unstable)
                        } else {
                            Action::Delete
                        }
                    },
                    Action::Keep,
                );
                Decision { backup, action }
            })
            .collect()
    }
}

fn is_too_young(now: SystemTime, modified: SystemTime, min_age: Duration) -> bool {
    now.duration_since(modified)
        .map_or(true, |age| age < min_age)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::str::FromStr;
    use std::time::{Duration, SystemTime};

    use chrono::{DateTime, TimeDelta, TimeZone, Utc};
    use chrono_tz::UTC;

    use crate::backup_file::BackupFile;
    use crate::period::RetentionPeriod;

    use super::{Action, KeepReason, RetentionPolicy};

    #[test]
    fn keeps_progressively_coarser_history() {
        let now = Utc.with_ymd_and_hms(2026, 9, 2, 12, 0, 0).unwrap();
        let policy = policy();
        let backups = vec![
            backup(now, 1, true),
            backup(now, 11, true),
            backup(now, 13, true),
            backup(now, 14, true),
            backup(now, 24 * 3, true),
            backup(now, 24 * 3 + 1, true),
            backup(now, 24 * 20, true),
            backup(now, 24 * 21, true),
            backup(now, 24 * 100, true),
            backup(now, 24 * 101, true),
            backup(now, 24 * 400, true),
        ];

        let decisions = policy.plan(now, Duration::ZERO, backups, UTC);
        let kept = decisions
            .iter()
            .filter(|decision| matches!(decision.action, Action::Keep(_)))
            .count();
        let deleted = decisions
            .iter()
            .filter(|decision| decision.action == Action::Delete)
            .count();

        assert_eq!(kept, 6);
        assert_eq!(deleted, 5);
        assert_eq!(decisions[0].action, Action::Keep(KeepReason::Newest));
    }

    #[test]
    fn never_deletes_an_unstable_redundant_backup() {
        let now = Utc.with_ymd_and_hms(2026, 9, 2, 12, 0, 0).unwrap();
        let decisions = policy().plan(
            now,
            Duration::ZERO,
            vec![backup(now, 13, true), backup(now, 14, false)],
            UTC,
        );

        assert_eq!(decisions[1].action, Action::Keep(KeepReason::Unstable));
    }

    fn policy() -> RetentionPolicy {
        RetentionPolicy::new(
            RetentionPeriod::from_str("12h").unwrap(),
            RetentionPeriod::from_str("14d").unwrap(),
            RetentionPeriod::from_str("12w").unwrap(),
            RetentionPeriod::from_str("12mo").unwrap(),
        )
        .unwrap()
    }

    fn backup(now: DateTime<Utc>, age_hours: i64, stable: bool) -> BackupFile {
        let timestamp = now - TimeDelta::hours(age_hours);
        BackupFile {
            path: PathBuf::from(format!("{}.zip", timestamp.timestamp())),
            timestamp,
            modified: SystemTime::UNIX_EPOCH,
            stable,
        }
    }
}
