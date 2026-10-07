//! Деньги: целые копейки, арифметика только через `checked_*`.
#![deny(clippy::arithmetic_side_effects)]

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::MoneyError;

const NBSP: char = '\u{a0}';
const MINUS: char = '\u{2212}';

/// Сумма в копейках. Может быть отрицательной (разности, остатки).
///
/// ```
/// use planning_budget_core::Money;
/// let m = Money::from_rub_str("2 596,50 ₽").unwrap();
/// assert_eq!(m.kopecks(), 259_650);
/// assert_eq!(m.to_string(), "2\u{a0}597\u{a0}₽");
/// assert_eq!(m.fmt_exact(), "2\u{a0}596,50\u{a0}₽");
/// ```
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Money(i64);

impl Money {
    pub const ZERO: Self = Self(0);

    pub const fn from_kopecks(kopecks: i64) -> Self {
        Self(kopecks)
    }

    pub const fn kopecks(self) -> i64 {
        self.0
    }

    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }

    /// Копейки как `f64` для долей и прогнозов.
    #[allow(
        clippy::cast_precision_loss,
        reason = "копейки < 2^53, точность f64 достаточна"
    )]
    pub fn as_f64(self) -> f64 {
        self.0 as f64
    }

    /// Округляет значение в копейках half away from zero.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "диапазон проверен строкой выше"
    )]
    pub fn from_kopecks_f64(kopecks: f64) -> Result<Self, MoneyError> {
        let rounded = kopecks.round();
        if !rounded.is_finite() || rounded.abs() >= 9.0e18 {
            return Err(MoneyError::Overflow);
        }
        Ok(Self(rounded as i64))
    }

    pub fn checked_add(self, other: Self) -> Result<Self, MoneyError> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or(MoneyError::Overflow)
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, MoneyError> {
        self.0
            .checked_sub(other.0)
            .map(Self)
            .ok_or(MoneyError::Overflow)
    }

    pub fn checked_neg(self) -> Result<Self, MoneyError> {
        self.0.checked_neg().map(Self).ok_or(MoneyError::Overflow)
    }

    /// Сумма значений; переполнение — ошибка, а не паника.
    pub fn sum(items: impl IntoIterator<Item = Self>) -> Result<Self, MoneyError> {
        items.into_iter().try_fold(Self::ZERO, Self::checked_add)
    }

    /// Умножение на долю (ставка, процент) с округлением до копейки.
    pub fn mul_ratio(self, ratio: f64) -> Result<Self, MoneyError> {
        Self::from_kopecks_f64(self.as_f64() * ratio)
    }

    /// Деление на `n > 0` с округлением half away from zero.
    pub fn div_round(self, n: i64) -> Result<Self, MoneyError> {
        if n <= 0 {
            return Err(MoneyError::DivisionByZero);
        }
        let quotient = self.0.checked_div(n).ok_or(MoneyError::Overflow)?;
        let remainder = self.0.checked_rem(n).ok_or(MoneyError::Overflow)?;
        let doubled = remainder.unsigned_abs().saturating_mul(2);
        if doubled < n.unsigned_abs() {
            return Ok(Self(quotient));
        }
        let step = if self.0 < 0 { -1 } else { 1 };
        quotient
            .checked_add(step)
            .map(Self)
            .ok_or(MoneyError::Overflow)
    }

    /// Доля `self / total`; `None`, если знаменатель равен нулю (UI показывает «—»).
    pub fn ratio(self, total: Self) -> Option<f64> {
        if total.is_zero() {
            return None;
        }
        Some(self.as_f64() / total.as_f64())
    }

    /// Доля `self / total` в базисных пунктах (10 000 = 100 %), округление half away from zero.
    /// `None`, если знаменатель равен нулю; результат ограничен диапазоном `i32`.
    pub fn ratio_bp(self, total: Self) -> Option<i32> {
        self.ratio_scaled(total, 10_000)
    }

    /// Доля `self / total` в целых процентах (то же округление, что у `ratio_bp`).
    pub fn ratio_percent(self, total: Self) -> Option<i32> {
        self.ratio_scaled(total, 100)
    }

    fn ratio_scaled(self, total: Self, scale: i128) -> Option<i32> {
        if total.is_zero() {
            return None;
        }
        let numerator = i128::from(self.0).checked_mul(scale)?;
        let denominator = i128::from(total.0);
        let negative = (numerator < 0) != (denominator < 0);
        let (n, d) = (numerator.unsigned_abs(), denominator.unsigned_abs());
        let quotient = i128::try_from(n.checked_add(d / 2)?.checked_div(d)?).ok()?;
        let signed = if negative {
            quotient.checked_neg()?
        } else {
            quotient
        };
        Some(i32::try_from(signed).unwrap_or(if signed < 0 { i32::MIN } else { i32::MAX }))
    }

    /// Разбор ввода: «2596», «2 596», «2596,5», «2 596,50 ₽», «-100».
    pub fn from_rub_str(input: &str) -> Result<Self, MoneyError> {
        let trimmed = input.trim();
        let trimmed = trimmed.strip_suffix('₽').unwrap_or(trimmed).trim_end();
        let (negative, rest) = match trimmed.strip_prefix(['-', MINUS]) {
            Some(rest) => (true, rest),
            None => (false, trimmed),
        };
        let cleaned: String = rest
            .chars()
            .filter(|c| !matches!(*c, ' ' | NBSP | '\u{202f}'))
            .collect();
        let mut parts = cleaned.split([',', '.']);
        let int_part = parts.next().ok_or(MoneyError::Invalid)?;
        let frac_part = parts.next();
        if parts.next().is_some() {
            return Err(MoneyError::Invalid);
        }
        let all_digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
        if !all_digits(int_part) || (int_part.is_empty() && frac_part.is_none()) {
            return Err(MoneyError::Invalid);
        }
        let rubles: i64 = if int_part.is_empty() {
            0
        } else {
            int_part.parse().map_err(|_| MoneyError::Overflow)?
        };
        let kopecks: i64 = match frac_part {
            None => 0,
            Some(f) if all_digits(f) && f.len() == 2 => {
                f.parse().map_err(|_| MoneyError::Invalid)?
            }
            Some(f) if all_digits(f) && f.len() == 1 => {
                let tenths: i64 = f.parse().map_err(|_| MoneyError::Invalid)?;
                tenths.checked_mul(10).ok_or(MoneyError::Overflow)?
            }
            Some(_) => return Err(MoneyError::Invalid),
        };
        let total = Self(
            rubles
                .checked_mul(100)
                .and_then(|k| k.checked_add(kopecks))
                .ok_or(MoneyError::Overflow)?,
        );
        if negative {
            total.checked_neg()
        } else {
            Ok(total)
        }
    }

    /// «1 234,50 ₽» — с копейками, для редактирования и тултипов.
    pub fn fmt_exact(self) -> String {
        let abs = self.0.unsigned_abs();
        let sign = if self.0 < 0 {
            MINUS.to_string()
        } else {
            String::new()
        };
        format!("{sign}{},{:02}{NBSP}₽", group_digits(abs / 100), abs % 100)
    }
}

/// «1 234 ₽» — до рубля, half away from zero; минус — U+2212.
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let abs = self.0.unsigned_abs();
        let whole = abs / 100;
        let rubles = if abs % 100 >= 50 {
            whole.saturating_add(1)
        } else {
            whole
        };
        let sign = if self.0 < 0 && rubles != 0 {
            MINUS.to_string()
        } else {
            String::new()
        };
        write!(f, "{sign}{}{NBSP}₽", group_digits(rubles))
    }
}

fn group_digits(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len().saturating_add(digits.len() / 3));
    for (i, ch) in digits.chars().enumerate() {
        let from_end = digits.len().saturating_sub(i);
        if i > 0 && from_end % 3 == 0 {
            out.push(NBSP);
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn ratio_in_basis_points_and_percent_rounds_half_away_from_zero() {
        let m = |kop: i64| Money(kop);
        assert_eq!(m(1).ratio_bp(m(3)), Some(3_333));
        assert_eq!(m(2).ratio_bp(m(3)), Some(6_667));
        assert_eq!(m(1).ratio_bp(m(8)), Some(1_250));
        assert_eq!(m(-1).ratio_bp(m(8)), Some(-1_250));
        assert_eq!(m(1).ratio_bp(m(-8)), Some(-1_250));
        assert_eq!(m(5).ratio_percent(m(200)), Some(3));
        assert_eq!(m(-5).ratio_percent(m(200)), Some(-3));
        assert_eq!(m(0).ratio_bp(m(100)), Some(0));
        assert_eq!(m(100).ratio_bp(Money::ZERO), None);
        assert_eq!(m(i64::MAX).ratio_bp(m(1)), Some(i32::MAX));
        assert_eq!(m(i64::MIN).ratio_bp(m(1)), Some(i32::MIN));
    }

    #[test]
    fn parses_money_input() {
        let cases = [
            ("2596", 259_600),
            ("2 596", 259_600),
            ("2596,5", 259_650),
            ("2596.05", 259_605),
            ("2 596,50 ₽", 259_650),
            ("-100", -10_000),
            ("\u{2212}100,01", -10_001),
            (",5", 50),
        ];
        for (input, kop) in cases {
            assert_eq!(
                Money::from_rub_str(input).map(Money::kopecks),
                Ok(kop),
                "input: {input}"
            );
        }
    }

    #[test]
    fn rejects_malformed_input() {
        for input in [
            "", "abc", "1,234,5", "12,345", "1,", "₽", "--5", "+5", "1e3",
        ] {
            assert!(Money::from_rub_str(input).is_err(), "input: {input}");
        }
    }

    #[test]
    fn display_rounds_to_ruble_half_away_from_zero() {
        let cases = [
            (0, "0\u{a0}₽"),
            (49, "0\u{a0}₽"),
            (-49, "0\u{a0}₽"),
            (50, "1\u{a0}₽"),
            (-50, "\u{2212}1\u{a0}₽"),
            (123_456_789, "1\u{a0}234\u{a0}568\u{a0}₽"),
        ];
        for (kop, expected) in cases {
            assert_eq!(
                Money::from_kopecks(kop).to_string(),
                expected,
                "kopecks: {kop}"
            );
        }
    }

    #[test]
    fn div_round_is_half_away_from_zero() {
        let cases = [(10, 4, 3), (-10, 4, -3), (9, 4, 2), (-9, 4, -2), (7, 1, 7)];
        for (kop, n, expected) in cases {
            assert_eq!(
                Money::from_kopecks(kop).div_round(n).map(Money::kopecks),
                Ok(expected)
            );
        }
        assert_eq!(Money::ZERO.div_round(0), Err(MoneyError::DivisionByZero));
    }

    #[test]
    fn sum_reports_overflow() {
        let big = Money::from_kopecks(i64::MAX);
        assert_eq!(
            Money::sum([big, Money::from_kopecks(1)]),
            Err(MoneyError::Overflow)
        );
    }

    #[test]
    fn ratio_is_none_for_zero_total() {
        assert_eq!(Money::from_kopecks(5).ratio(Money::ZERO), None);
        assert_eq!(
            Money::from_kopecks(5).ratio(Money::from_kopecks(10)),
            Some(0.5)
        );
    }

    const LIMIT: i64 = 1_000_000_000_000;

    proptest! {
        #[test]
        fn exact_format_parses_back(kop in -LIMIT..=LIMIT) {
            let money = Money::from_kopecks(kop);
            prop_assert_eq!(Money::from_rub_str(&money.fmt_exact()), Ok(money));
        }

        #[test]
        fn add_then_sub_is_identity(a in -LIMIT..=LIMIT, b in -LIMIT..=LIMIT) {
            let (a, b) = (Money::from_kopecks(a), Money::from_kopecks(b));
            prop_assert_eq!(a.checked_add(b).and_then(|s| s.checked_sub(b)), Ok(a));
        }

        #[test]
        fn display_never_panics_and_ends_with_ruble(kop in any::<i64>()) {
            prop_assert!(Money::from_kopecks(kop).to_string().ends_with('₽'));
        }
    }
}
