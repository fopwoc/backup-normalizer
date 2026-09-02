use std::str::FromStr;

use chrono::{DateTime, Months, TimeDelta};
use chrono_tz::Tz;

use crate::error::PeriodParseError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RetentionPeriod {
    amount: u32,
    unit: PeriodUnit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PeriodUnit {
    Hours,
    Days,
    Weeks,
    Months,
    Years,
}

impl RetentionPeriod {
    pub(crate) fn cutoff(self, now: DateTime<Tz>) -> Option<DateTime<Tz>> {
        match self.unit {
            PeriodUnit::Hours => now.checked_sub_signed(TimeDelta::hours(i64::from(self.amount))),
            PeriodUnit::Days => now.checked_sub_signed(TimeDelta::days(i64::from(self.amount))),
            PeriodUnit::Weeks => now.checked_sub_signed(TimeDelta::weeks(i64::from(self.amount))),
            PeriodUnit::Months => now.checked_sub_months(Months::new(self.amount)),
            PeriodUnit::Years => now.checked_sub_months(Months::new(self.amount.checked_mul(12)?)),
        }
    }

    pub(crate) fn approximate_hours(self) -> u64 {
        let multiplier = match self.unit {
            PeriodUnit::Hours => 1,
            PeriodUnit::Days => 24,
            PeriodUnit::Weeks => 24 * 7,
            PeriodUnit::Months => 24 * 31,
            PeriodUnit::Years => 24 * 366,
        };
        u64::from(self.amount) * multiplier
    }
}

impl FromStr for RetentionPeriod {
    type Err = PeriodParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let split_at = value
            .find(|character: char| !character.is_ascii_digit())
            .ok_or_else(|| PeriodParseError::MissingUnit(value.to_owned()))?;
        let (amount, unit) = value.split_at(split_at);
        let amount = amount
            .parse::<u32>()
            .map_err(|_| PeriodParseError::InvalidAmount(value.to_owned()))?;
        if amount == 0 {
            return Err(PeriodParseError::Zero);
        }

        let unit = match unit {
            "h" => PeriodUnit::Hours,
            "d" => PeriodUnit::Days,
            "w" => PeriodUnit::Weeks,
            "mo" => PeriodUnit::Months,
            "y" => PeriodUnit::Years,
            _ => return Err(PeriodParseError::UnsupportedUnit(unit.to_owned())),
        };

        Ok(Self { amount, unit })
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::RetentionPeriod;

    #[test]
    fn parses_supported_periods() {
        assert_eq!(
            RetentionPeriod::from_str("12h")
                .unwrap()
                .approximate_hours(),
            12
        );
        assert_eq!(
            RetentionPeriod::from_str("14d")
                .unwrap()
                .approximate_hours(),
            336
        );
        assert_eq!(
            RetentionPeriod::from_str("12w")
                .unwrap()
                .approximate_hours(),
            2016
        );
        assert_eq!(
            RetentionPeriod::from_str("12mo")
                .unwrap()
                .approximate_hours(),
            8928
        );
        assert_eq!(
            RetentionPeriod::from_str("2y").unwrap().approximate_hours(),
            17568
        );
    }

    #[test]
    fn rejects_ambiguous_or_empty_periods() {
        assert!(RetentionPeriod::from_str("12").is_err());
        assert!(RetentionPeriod::from_str("0h").is_err());
        assert!(RetentionPeriod::from_str("1m").is_err());
    }
}
