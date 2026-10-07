//! Календарный месяц учёта. Сериализуется строкой «YYYY-MM» (IPC, JSON, БД).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::CoreError;

const MONTH_NAMES: [&str; 12] = [
    "Январь",
    "Февраль",
    "Март",
    "Апрель",
    "Май",
    "Июнь",
    "Июль",
    "Август",
    "Сентябрь",
    "Октябрь",
    "Ноябрь",
    "Декабрь",
];

/// Месяц года; порядок сравнения — хронологический.
///
/// ```
/// use planning_budget_core::YearMonth;
/// let m = YearMonth::parse("2026-09").unwrap();
/// assert_eq!(m.to_string(), "2026-09");
/// assert_eq!(m.ru_name(), "Сентябрь 2026");
/// assert_eq!(m.succ().map(|n| n.to_string()), Some("2026-10".to_owned()));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct YearMonth {
    year: u16,
    month: u8,
}

impl YearMonth {
    pub fn new(year: u16, month: u8) -> Result<Self, CoreError> {
        if !(1970..=9999).contains(&year) || !(1..=12).contains(&month) {
            return Err(CoreError::Month);
        }
        Ok(Self { year, month })
    }

    /// Строго «YYYY-MM»: четыре цифры года, дефис, две цифры месяца.
    pub fn parse(s: &str) -> Result<Self, CoreError> {
        let (year, month) = s.split_once('-').ok_or(CoreError::Month)?;
        let digits =
            |part: &str, len: usize| part.len() == len && part.bytes().all(|b| b.is_ascii_digit());
        if !digits(year, 4) || !digits(month, 2) {
            return Err(CoreError::Month);
        }
        let year = year.parse().map_err(|_| CoreError::Month)?;
        let month = month.parse().map_err(|_| CoreError::Month)?;
        Self::new(year, month)
    }

    pub const fn year(self) -> u16 {
        self.year
    }

    pub const fn month(self) -> u8 {
        self.month
    }

    /// Январь года `year`.
    pub fn first_of_year(year: u16) -> Result<Self, CoreError> {
        Self::new(year, 1)
    }

    /// Следующий месяц; `None` после декабря 9999.
    pub fn succ(self) -> Option<Self> {
        if self.month == 12 {
            Self::new(self.year.checked_add(1)?, 1).ok()
        } else {
            Self::new(self.year, self.month.checked_add(1)?).ok()
        }
    }

    /// Предыдущий месяц; `None` до января 1970.
    pub fn pred(self) -> Option<Self> {
        if self.month == 1 {
            Self::new(self.year.checked_sub(1)?, 12).ok()
        } else {
            Self::new(self.year, self.month.checked_sub(1)?).ok()
        }
    }

    /// Месяцы от `self` до `end` включительно; пусто, если `end < self`.
    pub fn iter_to(self, end: Self) -> impl Iterator<Item = Self> {
        std::iter::successors(Some(self), |m| m.succ()).take_while(move |m| *m <= end)
    }

    /// Двенадцать месяцев года по порядку; недопустимый год — ошибка, а не пустой список.
    pub fn months_of_year(year: u16) -> Result<impl Iterator<Item = Self>, CoreError> {
        Ok(Self::first_of_year(year)?.iter_to(Self::new(year, 12)?))
    }

    /// «Сентябрь».
    pub fn month_name_ru(self) -> &'static str {
        let index = usize::from(self.month).saturating_sub(1);
        MONTH_NAMES.get(index).copied().unwrap_or_default()
    }

    /// «Сентябрь 2026».
    pub fn ru_name(self) -> String {
        format!("{} {}", self.month_name_ru(), self.year)
    }
}

impl fmt::Display for YearMonth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}", self.year, self.month)
    }
}

impl From<YearMonth> for String {
    fn from(value: YearMonth) -> Self {
        value.to_string()
    }
}

impl TryFrom<String> for YearMonth {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ym(s: &str) -> YearMonth {
        YearMonth::parse(s).unwrap()
    }

    #[test]
    fn parses_only_strict_format() {
        for bad in [
            "",
            "2026",
            "2026-9",
            "26-09",
            "2026-13",
            "2026-00",
            "2026/09",
            "2026-09-01",
            "+026-09",
            "1969-12",
        ] {
            assert!(YearMonth::parse(bad).is_err(), "input: {bad}");
        }
        assert_eq!(ym("2026-09").to_string(), "2026-09");
    }

    #[test]
    fn succ_and_pred_cross_year_boundary() {
        assert_eq!(ym("2026-12").succ(), Some(ym("2027-01")));
        assert_eq!(ym("2027-01").pred(), Some(ym("2026-12")));
        assert_eq!(ym("9999-12").succ(), None);
        assert_eq!(ym("1970-01").pred(), None);
    }

    #[test]
    fn iter_to_is_inclusive_and_empty_when_reversed() {
        let months: Vec<String> = ym("2026-11")
            .iter_to(ym("2027-02"))
            .map(|m| m.to_string())
            .collect();
        assert_eq!(months, ["2026-11", "2026-12", "2027-01", "2027-02"]);
        assert_eq!(ym("2026-05").iter_to(ym("2026-04")).count(), 0);
    }

    #[test]
    fn orders_chronologically() {
        assert!(ym("2025-12") < ym("2026-01"));
        assert!(ym("2026-02") < ym("2026-10"));
    }

    #[test]
    fn serializes_as_string() {
        let json = serde_json::to_string(&ym("2026-09")).unwrap();
        assert_eq!(json, "\"2026-09\"");
        assert_eq!(
            serde_json::from_str::<YearMonth>(&json).unwrap(),
            ym("2026-09")
        );
        assert!(serde_json::from_str::<YearMonth>("\"2026-9\"").is_err());
    }

    #[test]
    fn russian_names() {
        assert_eq!(ym("2026-01").month_name_ru(), "Январь");
        assert_eq!(ym("2026-12").ru_name(), "Декабрь 2026");
    }
}
