//! Calendar days in UTC, without a date library.

use std::time::{SystemTime, UNIX_EPOCH};

/// A day of the proleptic Gregorian calendar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: i64,
    pub month: u32,
    pub day: u32,
}

const MONTHS_EN: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const MONTHS_PT: [&str; 12] = [
    "janeiro",
    "fevereiro",
    "março",
    "abril",
    "maio",
    "junho",
    "julho",
    "agosto",
    "setembro",
    "outubro",
    "novembro",
    "dezembro",
];

impl Date {
    /// `YYYY-MM-DD`, an existing day; nothing else.
    pub fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return None;
        }
        let digits = |range: std::ops::Range<usize>| -> Option<i64> {
            let part = text.get(range)?;
            part.bytes()
                .all(|b| b.is_ascii_digit())
                .then(|| part.parse().ok())?
        };
        let date = Date {
            year: digits(0..4)?,
            month: u32::try_from(digits(5..7)?).ok()?,
            day: u32::try_from(digits(8..10)?).ok()?,
        };
        (date.year >= 1
            && (1..=12).contains(&date.month)
            && date.day >= 1
            && date.day <= days_in_month(date.year, date.month))
        .then_some(date)
    }

    /// Today in UTC.
    pub fn today() -> Self {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        Self::from_days(i64::try_from(seconds / 86_400).unwrap_or(0))
    }

    /// Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
    pub fn days(self) -> i64 {
        let year = if self.month <= 2 {
            self.year - 1
        } else {
            self.year
        };
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let month = i64::from(self.month);
        let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5
            + i64::from(self.day)
            - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    /// The day `days` after 1970-01-01 (`civil_from_days`).
    pub fn from_days(days: i64) -> Self {
        let shifted = days + 719_468;
        let era = shifted.div_euclid(146_097);
        let day_of_era = shifted - era * 146_097;
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_index = (5 * day_of_year + 2) / 153;
        let day = u32::try_from(day_of_year - (153 * month_index + 2) / 5 + 1).unwrap_or(1);
        let month = u32::try_from(if month_index < 10 {
            month_index + 3
        } else {
            month_index - 9
        })
        .unwrap_or(1);
        let year = year_of_era + era * 400 + i64::from(month <= 2);
        Date { year, month, day }
    }

    /// `YYYY-MM-DD`.
    pub fn iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// "20 November 2026" or "20 de novembro de 2026".
    pub fn long(self, portuguese: bool) -> String {
        let index = usize::try_from(self.month.saturating_sub(1)).unwrap_or(0) % 12;
        if portuguese {
            format!("{} de {} de {}", self.day, MONTHS_PT[index], self.year)
        } else {
            format!("{} {} {}", self.day, MONTHS_EN[index], self.year)
        }
    }
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

#[cfg(test)]
mod tests {
    use super::Date;

    #[test]
    fn parses_only_existing_iso_days() {
        assert_eq!(
            Date::parse("2026-11-20").map(Date::iso).as_deref(),
            Some("2026-11-20")
        );
        for bad in [
            "2026-02-29",
            "2026-13-01",
            "20261120",
            "2026-1-20",
            "2026-11-20T00:00",
            "+026-11-20",
        ] {
            assert!(Date::parse(bad).is_none(), "{bad}");
        }
        assert!(Date::parse("2024-02-29").is_some());
    }

    #[test]
    fn counts_days_both_ways() {
        let day = Date::parse("2026-11-21").unwrap();
        assert_eq!(Date::from_days(day.days()), day);
        assert_eq!(day.days() - Date::parse("2026-08-22").unwrap().days(), 91);
        assert_eq!(Date::from_days(0).iso(), "1970-01-01");
        assert_eq!(day.long(false), "21 November 2026");
        assert_eq!(day.long(true), "21 de novembro de 2026");
    }
}
