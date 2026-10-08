//! Разбор старой таблицы xlsx-файл.
//!
//! Парсер ищет заголовки («Категория», «Наименование», «Итого»), а не фиксированные
//! координаты, и не доверяет структуре: любое отклонение — `ImportError` с номером строки,
//! без содержимого ячеек (текст ошибки может попасть в лог).

use std::io::Cursor;

use calamine::{Reader as _, Xlsx};
use planning_budget_core::LegacyBook;

use self::cells::sheet;
use self::reference::reference;
use self::sections::{expenses, incomes};

use crate::ImportError;

mod cells;
mod reference;
mod sections;

const SHEET_REFERENCE: &str = "Справочник";
const SHEET_EXPENSES: &str = "Расходы";
const SHEET_INCOMES: &str = "Доходы";

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
    use super::cells::positive;
    use super::*;
    use planning_budget_core::Money;

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
