//! Чтение ячеек и подписей таблицы: числа, статусы, месяцы, листы.

use std::io::Cursor;

use calamine::{Data, Reader as _, Xlsx};
use planning_budget_core::{BasisPoints, CategoryKind, IncomeStatus, Money, TxStatus, YearMonth};

use super::{SHEET_EXPENSES, SHEET_INCOMES, SHEET_REFERENCE};
use crate::ImportError;

const MONTHS: [&str; 12] = [
    "январь",
    "февраль",
    "март",
    "апрель",
    "май",
    "июнь",
    "июль",
    "август",
    "сентябрь",
    "октябрь",
    "ноябрь",
    "декабрь",
];

pub(super) type Rows = Vec<Vec<Data>>;

pub(super) fn sheet(
    book: &mut Xlsx<Cursor<&[u8]>>,
    name: &'static str,
) -> Result<Rows, ImportError> {
    let range = book
        .worksheet_range(name)
        .map_err(|_| ImportError::MissingSheet(name))?;
    Ok(range.rows().map(<[Data]>::to_vec).collect())
}

pub(super) fn cell(rows: &Rows, row: usize, col: usize) -> &Data {
    rows.get(row)
        .and_then(|r| r.get(col))
        .unwrap_or(&Data::Empty)
}

pub(super) fn text(rows: &Rows, row: usize, col: usize) -> Option<String> {
    match cell(rows, row, col) {
        Data::String(s) => {
            let s = s.trim();
            (!s.is_empty()).then(|| s.to_owned())
        }
        _ => None,
    }
}

/// Сумма в рублях из числовой ячейки; пусто и не число — `None`.
pub(super) fn rubles(rows: &Rows, row: usize, col: usize) -> Result<Option<Money>, ImportError> {
    let value = match cell(rows, row, col) {
        Data::Float(v) => *v,
        #[allow(
            clippy::cast_precision_loss,
            reason = "рубли в ячейке много меньше 2^53"
        )]
        Data::Int(v) => *v as f64,
        _ => return Ok(None),
    };
    Money::from_kopecks_f64((value * 100.0).round())
        .map(Some)
        .map_err(|_| ImportError::Number {
            sheet: SHEET_EXPENSES,
            row: row_number(row),
        })
}

/// Сумма позиции: нулевая означает «нет суммы» (строка пропускается), отрицательная — ошибка
/// с номером строки (в базе `amount > 0`, возвраты ведутся иначе).
pub(super) fn positive(
    amount: Money,
    sheet: &'static str,
    row: usize,
) -> Result<Option<Money>, ImportError> {
    if amount.is_negative() {
        Err(ImportError::Number {
            sheet,
            row: row_number(row),
        })
    } else {
        Ok((!amount.is_zero()).then_some(amount))
    }
}

pub(super) fn row_number(index: usize) -> u32 {
    u32::try_from(index).map_or(u32::MAX, |i| i.saturating_add(1))
}

pub(super) fn basis_points(value: f64, row: usize) -> Result<BasisPoints, ImportError> {
    let bad = || ImportError::Number {
        sheet: SHEET_REFERENCE,
        row: row_number(row),
    };
    let kopecks = Money::from_kopecks_f64((value * 10_000.0).round()).map_err(|_| bad())?;
    i32::try_from(kopecks.kopecks())
        .ok()
        .filter(|bp| (0..=10_000).contains(bp))
        .map(BasisPoints)
        .ok_or_else(bad)
}

pub(super) fn tx_status(label: Option<&str>, row: usize) -> Result<TxStatus, ImportError> {
    match label.map(str::to_lowercase).as_deref() {
        Some("оплачено") => Ok(TxStatus::Paid),
        Some("долг") => Ok(TxStatus::Debt),
        Some("незапланировано") => Ok(TxStatus::Unplanned),
        // Пустой статус при заполненной сумме — «план» (так было в исходном переносе).
        Some("план") | None => Ok(TxStatus::Planned),
        Some(_) => Err(ImportError::UnknownStatus {
            sheet: SHEET_EXPENSES,
            row: row_number(row),
        }),
    }
}

pub(super) fn income_status(label: Option<&str>, row: usize) -> Result<IncomeStatus, ImportError> {
    match label.map(str::to_lowercase).as_deref() {
        Some("получено") => Ok(IncomeStatus::Received),
        Some("ожидается") => Ok(IncomeStatus::Expected),
        _ => Err(ImportError::UnknownStatus {
            sheet: SHEET_INCOMES,
            row: row_number(row),
        }),
    }
}

pub(super) fn kind(label: &str, row: usize) -> Result<CategoryKind, ImportError> {
    match label.to_lowercase().as_str() {
        "обязательные" => Ok(CategoryKind::Mandatory),
        "желания" => Ok(CategoryKind::Wants),
        "сбережения" => Ok(CategoryKind::Savings),
        "займы" => Ok(CategoryKind::Loans),
        _ => Err(ImportError::UnknownKind {
            row: row_number(row),
        }),
    }
}

pub(super) fn month_number(name: &str) -> Option<u8> {
    let name = name.to_lowercase();
    MONTHS
        .iter()
        .position(|m| *m == name)
        .and_then(|i| u8::try_from(i + 1).ok())
}

/// Заголовок секции расходов: «ЯНВАРЬ 2026».
pub(super) fn section_header(value: &str) -> Option<YearMonth> {
    let (name, year) = value.trim().split_once(' ')?;
    let month = month_number(name)?;
    YearMonth::new(year.trim().parse().ok()?, month).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(cells: Vec<Vec<Data>>) -> Rows {
        cells
    }

    fn row_err(sheet: &'static str, row: u32) -> ImportError {
        ImportError::Number { sheet, row }
    }

    #[test]
    fn missing_cells_read_as_empty() {
        let table = rows(vec![vec![
            Data::String("  текст  ".into()),
            Data::Float(1.5),
        ]]);
        assert_eq!(text(&table, 0, 0).as_deref(), Some("текст"));
        assert_eq!(text(&table, 0, 1), None, "числа не текст");
        assert_eq!(text(&table, 5, 5), None);
        assert!(matches!(cell(&table, 9, 9), Data::Empty));
        let blank = rows(vec![vec![Data::String("   ".into())]]);
        assert_eq!(text(&blank, 0, 0), None);
    }

    #[test]
    fn rubles_round_to_kopecks_and_ignore_non_numbers() {
        let table = rows(vec![vec![
            Data::Float(12.345),
            Data::Int(7),
            Data::String("12".into()),
            Data::Float(f64::NAN),
        ]]);
        assert_eq!(rubles(&table, 0, 0), Ok(Some(Money::from_kopecks(1235))));
        assert_eq!(rubles(&table, 0, 1), Ok(Some(Money::from_kopecks(700))));
        assert_eq!(rubles(&table, 0, 2), Ok(None));
        assert_eq!(rubles(&table, 0, 3), Err(row_err(SHEET_EXPENSES, 1)));
    }

    #[test]
    fn row_numbers_are_one_based_and_saturate() {
        assert_eq!(row_number(0), 1);
        assert_eq!(row_number(41), 42);
        assert_eq!(row_number(usize::MAX), u32::MAX);
    }

    #[test]
    fn basis_points_accept_zero_to_hundred_percent_only() {
        assert_eq!(basis_points(0.14, 3), Ok(BasisPoints(1400)));
        assert_eq!(basis_points(0.0, 3), Ok(BasisPoints(0)));
        assert_eq!(basis_points(1.0, 3), Ok(BasisPoints(10_000)));
        assert_eq!(basis_points(1.01, 3), Err(row_err(SHEET_REFERENCE, 4)));
        assert_eq!(basis_points(-0.01, 3), Err(row_err(SHEET_REFERENCE, 4)));
    }

    #[test]
    fn transaction_status_dictionary_and_default_plan() {
        let status = |label| tx_status(label, 0);
        assert_eq!(status(Some("Оплачено")), Ok(TxStatus::Paid));
        assert_eq!(status(Some("ДОЛГ")), Ok(TxStatus::Debt));
        assert_eq!(status(Some("Незапланировано")), Ok(TxStatus::Unplanned));
        assert_eq!(status(Some("план")), Ok(TxStatus::Planned));
        assert_eq!(status(None), Ok(TxStatus::Planned));
        assert_eq!(
            status(Some("что-то")),
            Err(ImportError::UnknownStatus {
                sheet: SHEET_EXPENSES,
                row: 1
            })
        );
    }

    #[test]
    fn income_status_requires_a_known_label() {
        assert_eq!(
            income_status(Some("Получено"), 0),
            Ok(IncomeStatus::Received)
        );
        assert_eq!(
            income_status(Some("ожидается"), 0),
            Ok(IncomeStatus::Expected)
        );
        for bad in [None, Some("план")] {
            assert_eq!(
                income_status(bad, 4),
                Err(ImportError::UnknownStatus {
                    sheet: SHEET_INCOMES,
                    row: 5
                })
            );
        }
    }

    #[test]
    fn kinds_map_to_categories_and_unknown_is_an_error() {
        assert_eq!(kind("Обязательные", 0), Ok(CategoryKind::Mandatory));
        assert_eq!(kind("желания", 0), Ok(CategoryKind::Wants));
        assert_eq!(kind("СБЕРЕЖЕНИЯ", 0), Ok(CategoryKind::Savings));
        assert_eq!(kind("Займы", 0), Ok(CategoryKind::Loans));
        assert_eq!(kind("прочее", 6), Err(ImportError::UnknownKind { row: 7 }));
    }

    #[test]
    fn month_names_and_section_headers() {
        assert_eq!(month_number("Январь"), Some(1));
        assert_eq!(month_number("декабрь"), Some(12));
        assert_eq!(month_number("smarch"), None);
        assert_eq!(
            section_header(" ЯНВАРЬ 2026 "),
            Some(YearMonth::new(2026, 1).unwrap())
        );
        assert_eq!(section_header("Январь"), None);
        assert_eq!(section_header("Январь двадцать"), None);
        assert_eq!(section_header("Нечто 2026"), None);
    }
}
