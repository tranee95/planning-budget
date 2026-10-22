//! Правила записей трат и доходов: месяц и дата, сумма, название.
//!
//! Решения предметной области живут здесь; хранилище только вызывает их и пишет.

use chrono::{Datelike as _, NaiveDate};

use crate::money::Money;
use crate::period::YearMonth;

/// Запись нарушает правило; ключ вида `record.*` показывается пользователю через каталог текстов.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
#[error("invalid record: {0}")]
pub struct RecordError(pub &'static str);

/// Верхняя граница суммы записи: 10^12 ₽ в копейках. Больше не бывает, а значения
/// около `i64::MAX` ломали бы сложение в расчётах и выходили бы за 2^53 в UI.
pub const MAX_AMOUNT_KOPECKS: i64 = 100_000_000_000_000;

/// Месяц календарной даты.
///
/// # Errors
/// `record.date_invalid`, если год вне диапазона `YearMonth`.
pub fn month_of(date: NaiveDate) -> Result<YearMonth, RecordError> {
    let invalid = || RecordError("record.date_invalid");
    let year = u16::try_from(date.year()).map_err(|_| invalid())?;
    let month = u8::try_from(date.month()).map_err(|_| invalid())?;
    YearMonth::new(year, month).map_err(|_| invalid())
}

/// Месяц и дата записи при создании: дата, если она есть, должна лежать в месяце записи.
///
/// # Errors
/// `record.date_month_mismatch`, если дата не из `month`.
pub fn period(
    month: YearMonth,
    date: Option<NaiveDate>,
) -> Result<(YearMonth, Option<NaiveDate>), RecordError> {
    if let Some(date) = date
        && month_of(date)? != month
    {
        return Err(RecordError("record.date_month_mismatch"));
    }
    Ok((month, date))
}

/// Месяц и дата после правки. Смена месяца без новой даты сбрасывает старую дату,
/// смена даты без месяца переносит запись в месяц новой даты.
///
/// # Errors
/// Как у [`period`].
pub fn patched_period(
    old: (YearMonth, Option<NaiveDate>),
    month: Option<YearMonth>,
    date: Option<Option<NaiveDate>>,
) -> Result<(YearMonth, Option<NaiveDate>), RecordError> {
    let new_date = match date {
        Some(date) => date,
        None if month.is_some_and(|m| m != old.0) => None,
        None => old.1,
    };
    let new_month = match (month, new_date) {
        (Some(m), _) => m,
        (None, Some(d)) if date.is_some() => month_of(d)?,
        (None, _) => old.0,
    };
    period(new_month, new_date)
}

/// Сумма записи: больше нуля и не больше [`MAX_AMOUNT_KOPECKS`].
///
/// # Errors
/// `record.amount_not_positive` или `record.amount_too_large`.
pub fn validate_amount(amount: Money) -> Result<(), RecordError> {
    if amount <= Money::ZERO {
        Err(RecordError("record.amount_not_positive"))
    } else if amount.kopecks() > MAX_AMOUNT_KOPECKS {
        Err(RecordError("record.amount_too_large"))
    } else {
        Ok(())
    }
}

/// Название записи без внешних пробелов.
///
/// # Errors
/// `record.title_empty`, если название пустое.
pub fn validate_title(title: &str) -> Result<&str, RecordError> {
    let title = title.trim();
    if title.is_empty() {
        Err(RecordError("record.title_empty"))
    } else {
        Ok(title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ym(s: &str) -> YearMonth {
        YearMonth::parse(s).unwrap()
    }

    fn day(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn date_must_belong_to_the_month() {
        assert!(period(ym("2026-10"), Some(day("2026-10-31"))).is_ok());
        assert_eq!(
            period(ym("2026-10"), Some(day("2026-11-01"))),
            Err(RecordError("record.date_month_mismatch"))
        );
    }

    #[test]
    fn a_date_outside_the_supported_years_is_invalid() {
        let before_year_zero = NaiveDate::from_ymd_opt(-1, 1, 1).unwrap();
        assert_eq!(
            month_of(before_year_zero),
            Err(RecordError("record.date_invalid"))
        );
        assert_eq!(month_of(day("2026-10-01")), Ok(ym("2026-10")));
    }

    #[test]
    fn patched_period_follows_docs_03_rules() {
        let old = (ym("2026-10"), Some(day("2026-10-05")));
        // Смена месяца без даты сбрасывает дату.
        assert_eq!(
            patched_period(old, Some(ym("2026-11")), None),
            Ok((ym("2026-11"), None))
        );
        // Смена даты без месяца переносит запись в месяц новой даты.
        assert_eq!(
            patched_period(old, None, Some(Some(day("2026-12-01")))),
            Ok((ym("2026-12"), Some(day("2026-12-01"))))
        );
        // Без правок периода ничего не меняется.
        assert_eq!(patched_period(old, None, None), Ok(old));
    }

    #[test]
    fn amount_is_positive_and_bounded() {
        assert!(validate_amount(Money::from_kopecks(1)).is_ok());
        assert_eq!(
            validate_amount(Money::ZERO),
            Err(RecordError("record.amount_not_positive"))
        );
        assert_eq!(
            validate_amount(Money::from_kopecks(MAX_AMOUNT_KOPECKS + 1)),
            Err(RecordError("record.amount_too_large"))
        );
        assert!(validate_amount(Money::from_kopecks(MAX_AMOUNT_KOPECKS)).is_ok());
    }

    #[test]
    fn title_is_trimmed_and_not_empty() {
        assert_eq!(validate_title("  Хлеб "), Ok("Хлеб"));
        assert_eq!(validate_title("  "), Err(RecordError("record.title_empty")));
    }
}
