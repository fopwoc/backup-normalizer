use std::path::PathBuf;
use std::time::Duration;

use chrono_tz::Tz;
use clap::Parser;

use crate::application::Config;
use crate::error::ConfigError;
use crate::period::RetentionPeriod;
use crate::policy::RetentionPolicy;

#[derive(Debug, Parser)]
#[command(version = env!("BUILD_VERSION"), about = "Thin timestamped backups into progressively coarser retention tiers")]
pub(crate) struct Cli {
    /// Directory containing backup files. Only its immediate children are inspected.
    #[arg(long, default_value = "/backups")]
    path: PathBuf,

    /// Chrono filename format. Files that do not parse exactly are never touched.
    #[arg(long, default_value = "%Y-%m-%d-%H-%M-%S.zip")]
    filename_format: String,

    /// Keep every matching backup within this age.
    #[arg(long, default_value = "12h")]
    keep_all_for: RetentionPeriod,

    /// Keep one matching backup per local calendar day within this age.
    #[arg(long, default_value = "14d")]
    keep_daily_for: RetentionPeriod,

    /// Keep one matching backup per local ISO week within this age.
    #[arg(long, default_value = "12w")]
    keep_weekly_for: RetentionPeriod,

    /// Keep one matching backup per local calendar month within this age.
    #[arg(long, default_value = "12mo")]
    keep_monthly_for: RetentionPeriod,

    /// Do not delete a file modified more recently than this duration.
    #[arg(long, default_value = "2h", value_parser = parse_duration)]
    min_age: Duration,

    /// Delay between directory scans.
    #[arg(long, default_value = "1h", value_parser = parse_duration)]
    scan_every: Duration,

    /// IANA timezone used for daily, weekly, and monthly calendar buckets.
    #[arg(long, env = "TZ", default_value = "UTC")]
    timezone: Tz,

    /// Delete redundant backups. Without this flag, only the plan is logged.
    #[arg(long)]
    apply: bool,
}

impl Cli {
    pub(crate) fn into_config(self) -> Result<Config, ConfigError> {
        if self.scan_every.is_zero() {
            return Err(ConfigError::ZeroScanInterval);
        }

        let policy = RetentionPolicy::new(
            self.keep_all_for,
            self.keep_daily_for,
            self.keep_weekly_for,
            self.keep_monthly_for,
        )?;

        Ok(Config {
            path: self.path,
            filename_format: self.filename_format,
            timezone: self.timezone,
            policy,
            min_age: self.min_age,
            scan_every: self.scan_every,
            apply: self.apply,
        })
    }
}

fn parse_duration(value: &str) -> Result<Duration, String> {
    humantime::parse_duration(value).map_err(|error| error.to_string())
}
