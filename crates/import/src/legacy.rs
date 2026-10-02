//! Разбор старой таблицы xlsx-файл.
//!
//! Парсер ищет заголовки («Категория», «Наименование», «Итого»), а не фиксированные
//! координаты, и не доверяет структуре: любое отклонение — `ImportError` с номером строки,
//! без содержимого ячеек (текст ошибки может попасть в лог).

use std::io::Cursor;

use budget_core::{
    BasisPoints, BlockMismatch, CategoryKind, IncomeStatus, LegacyBook, LegacyCategory,
    LegacyIncome, LegacySettings, LegacyTransaction, Money, TxStatus, YearMonth,
};
use calamine::{Data, Reader as _, Xlsx};

use crate::ImportError;

const SHEET_REFERENCE: &str = "Справочник";
const SHEET_EXPENSES: &str = "Расходы";
const SHEET_INCOMES: &str = "Доходы";

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

type Rows = Vec<Vec<Data>>;

fn sheet(book: &mut Xlsx<Cursor<&[u8]>>, name: &'static str) -> Result<Rows, ImportError> {
    let range = book
        .worksheet_range(name)
        .map_err(|_| ImportError::MissingSheet(name))?;
    Ok(range.rows().map(<[Data]>::to_vec).collect())
}

fn cell(rows: &Rows, row: usize, col: usize) -> &Data {
    rows.get(row)
        .and_then(|r| r.get(col))
        .unwrap_or(&Data::Empty)
}

fn text(rows: &Rows, row: usize, col: usize) -> Option<String> {
    match cell(rows, row, col) {
        Data::String(s) => {
            let s = s.trim();
            (!s.is_empty()).then(|| s.to_owned())
        }
        _ => None,
    }
}

/// Сумма в рублях из числовой ячейки; пусто и не число — `None`.
fn rubles(rows: &Rows, row: usize, col: usize) -> Result<Option<Money>, ImportError> {
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
fn positive(amount: Money, sheet: &'static str, row: usize) -> Result<Option<Money>, ImportError> {
    if amount.is_negative() {
        Err(ImportError::Number {
            sheet,
            row: row_number(row),
        })
    } else {
        Ok((!amount.is_zero()).then_some(amount))
    }
}

fn row_number(index: usize) -> u32 {
    u32::try_from(index).map_or(u32::MAX, |i| i.saturating_add(1))
}

fn basis_points(value: f64, row: usize) -> Result<BasisPoints, ImportError> {
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

fn tx_status(label: Option<&str>, row: usize) -> Result<TxStatus, ImportError> {
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

fn income_status(label: Option<&str>, row: usize) -> Result<IncomeStatus, ImportError> {
    match label.map(str::to_lowercase).as_deref() {
        Some("получено") => Ok(IncomeStatus::Received),
        Some("ожидается") => Ok(IncomeStatus::Expected),
        _ => Err(ImportError::UnknownStatus {
            sheet: SHEET_INCOMES,
            row: row_number(row),
        }),
    }
}

fn kind(label: &str, row: usize) -> Result<CategoryKind, ImportError> {
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

fn month_number(name: &str) -> Option<u8> {
    let name = name.to_lowercase();
    MONTHS
        .iter()
        .position(|m| *m == name)
        .and_then(|i| u8::try_from(i + 1).ok())
}

/// Заголовок секции расходов: «ЯНВАРЬ 2026».
fn section_header(value: &str) -> Option<YearMonth> {
    let (name, year) = value.trim().split_once(' ')?;
    let month = month_number(name)?;
    YearMonth::new(year.trim().parse().ok()?, month).ok()
}

fn reference(rows: &Rows) -> Result<(Vec<LegacyCategory>, Option<LegacySettings>), ImportError> {
    let header = rows
        .iter()
        .position(|r| matches!(r.first(), Some(Data::String(s)) if s.trim() == "Категория"))
        .ok_or(ImportError::Layout {
            sheet: SHEET_REFERENCE,
            row: 0,
        })?;
    let mut categories = Vec::new();
    for row in header + 1..rows.len() {
        let Some(name) = text(rows, row, 0) else {
            break;
        };
        let kind = kind(
            &text(rows, row, 1).ok_or(ImportError::UnknownKind {
                row: row_number(row),
            })?,
            row,
        )?;
        let limit = if kind == CategoryKind::Savings {
            None
        } else {
            rubles(rows, row, 2)?
        };
        categories.push(LegacyCategory {
            name,
            kind,
            limit,
            note: text(rows, row, 5),
        });
    }

    let mut values = [None::<f64>; 6];
    for row in header..rows.len() {
        let Some(label) = text(rows, row, 12) else {
            continue;
        };
        let value = match cell(rows, row, 13) {
            Data::Float(v) => *v,
            #[allow(
                clippy::cast_precision_loss,
                reason = "значения настроек много меньше 2^53"
            )]
            Data::Int(v) => *v as f64,
            _ => continue,
        };
        let label = label.to_lowercase();
        let slot = if label.contains("минимум") {
            0
        } else if label.contains("норма") && label.contains("цель") {
            1
        } else if label.contains("максимум") {
            2
        } else if label.starts_with("доходность облигаций") {
            3
        } else if label.starts_with("ндфл") {
            4
        } else if label.starts_with("уже вложено") {
            5
        } else {
            continue;
        };
        if let Some(target) = values.get_mut(slot) {
            *target = Some(value);
        }
    }
    let settings = match values {
        [
            Some(min),
            Some(norm),
            Some(max),
            Some(rate),
            Some(tax),
            Some(initial),
        ] => Some(LegacySettings {
            savings_min: basis_points(min, header)?,
            savings_norm: basis_points(norm, header)?,
            savings_max: basis_points(max, header)?,
            bonds_rate: basis_points(rate, header)?,
            bonds_coupon_tax: basis_points(tax, header)?,
            bonds_initial_balance: Money::from_kopecks_f64((initial * 100.0).round()).map_err(
                |_| ImportError::Number {
                    sheet: SHEET_REFERENCE,
                    row: row_number(header),
                },
            )?,
        }),
        _ => None,
    };
    Ok((categories, settings))
}

/// Блок категории: заголовок строкой выше «Наименование», позиции до «Итого».
fn expenses(rows: &Rows, book: &mut LegacyBook) -> Result<Option<u16>, ImportError> {
    let mut month: Option<YearMonth> = None;
    let mut year: Option<u16> = None;
    for row in 0..rows.len() {
        let width = rows.get(row).map_or(0, Vec::len);
        for col in 0..width {
            if let Some(found) = text(rows, row, col).as_deref().and_then(section_header) {
                month = Some(found);
                year.get_or_insert(found.year());
            }
        }
        for col in 0..width {
            if text(rows, row, col).as_deref() != Some("Наименование") {
                continue;
            }
            let month = month.ok_or(ImportError::Layout {
                sheet: SHEET_EXPENSES,
                row: row_number(row),
            })?;
            let category = row
                .checked_sub(1)
                .and_then(|above| text(rows, above, col))
                .ok_or(ImportError::Layout {
                    sheet: SHEET_EXPENSES,
                    row: row_number(row),
                })?;
            block(rows, book, month, category, row, col)?;
        }
    }
    Ok(year)
}

fn block(
    rows: &Rows,
    book: &mut LegacyBook,
    month: YearMonth,
    category: String,
    header_row: usize,
    col: usize,
) -> Result<(), ImportError> {
    let mut parsed = Money::ZERO;
    let mut declared = None;
    for row in header_row + 1..rows.len() {
        if text(rows, row, col).as_deref() == Some("Итого") {
            declared = rubles(rows, row, col + 1)?;
            break;
        }
        let Some(amount) = rubles(rows, row, col + 1)? else {
            continue;
        };
        let Some(amount) = positive(amount, SHEET_EXPENSES, row)? else {
            continue;
        };
        parsed = parsed
            .checked_add(amount)
            .map_err(|_| ImportError::Number {
                sheet: SHEET_EXPENSES,
                row: row_number(row),
            })?;
        book.transactions.push(LegacyTransaction {
            month,
            category: category.clone(),
            title: text(rows, row, col).unwrap_or_else(|| "Без названия".to_owned()),
            amount,
            status: tx_status(text(rows, row, col + 2).as_deref(), row)?,
        });
    }
    let declared = declared.ok_or(ImportError::Layout {
        sheet: SHEET_EXPENSES,
        row: row_number(header_row),
    })?;
    if declared != parsed {
        book.block_mismatches.push(BlockMismatch {
            month,
            category,
            declared,
            parsed,
        });
    }
    Ok(())
}

fn incomes(rows: &Rows, year: u16, book: &mut LegacyBook) -> Result<(), ImportError> {
    for row in 1..rows.len() {
        let Some(month_name) = text(rows, row, 0) else {
            continue;
        };
        let month = month_number(&month_name)
            .and_then(|m| YearMonth::new(year, m).ok())
            .ok_or(ImportError::Layout {
                sheet: SHEET_INCOMES,
                row: row_number(row),
            })?;
        let Some(amount) = rubles(rows, row, 3)? else {
            continue;
        };
        let Some(amount) = positive(amount, SHEET_INCOMES, row)? else {
            continue;
        };
        book.incomes.push(LegacyIncome {
            month,
            source_name: text(rows, row, 2).unwrap_or_else(|| "Без названия".to_owned()),
            amount,
            status: income_status(text(rows, row, 4).as_deref(), row)?,
        });
    }
    Ok(())
}

/// Разбирает книгу целиком. Ничего не пишет и не читает с диска: байты передаёт вызывающий.
///
/// # Errors
/// [`ImportError`] с листом и номером строки, если файл не xlsx или структура отличается
/// от.
pub fn parse_legacy(bytes: &[u8]) -> Result<LegacyBook, ImportError> {
    let mut workbook: Xlsx<_> = Xlsx::new(Cursor::new(bytes)).map_err(|_| ImportError::NotXlsx)?;
    let reference_rows = sheet(&mut workbook, SHEET_REFERENCE)?;
    let expense_rows = sheet(&mut workbook, SHEET_EXPENSES)?;
    let income_rows = sheet(&mut workbook, SHEET_INCOMES)?;

    let mut book = LegacyBook::default();
    (book.categories, book.settings) = reference(&reference_rows)?;
    let year = expenses(&expense_rows, &mut book)?.ok_or(ImportError::NoYear)?;
    incomes(&income_rows, year, &mut book)?;
    Ok(book)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_amount_is_skipped_and_negative_is_an_error_with_the_row() {
        assert_eq!(positive(Money::ZERO, SHEET_EXPENSES, 4), Ok(None));
        assert_eq!(
            positive(Money::from_kopecks(5), SHEET_EXPENSES, 4),
            Ok(Some(Money::from_kopecks(5)))
        );
        assert_eq!(
            positive(Money::from_kopecks(-5), SHEET_INCOMES, 4),
            Err(ImportError::Number {
                sheet: SHEET_INCOMES,
                row: 5
            })
        );
    }
}
