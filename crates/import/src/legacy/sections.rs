//! Листы «Расходы» и «Доходы»: блоки категорий по месяцам и позиции доходов.

use planning_budget_core::{
    BlockMismatch, LegacyBook, LegacyIncome, LegacyTransaction, Money, YearMonth,
};

use super::cells::*;
use super::{SHEET_EXPENSES, SHEET_INCOMES};
use crate::ImportError;

/// Блок категории: заголовок строкой выше «Наименование», позиции до «Итого».
pub(super) fn expenses(rows: &Rows, book: &mut LegacyBook) -> Result<Option<u16>, ImportError> {
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

pub(super) fn block(
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

pub(super) fn incomes(rows: &Rows, year: u16, book: &mut LegacyBook) -> Result<(), ImportError> {
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
