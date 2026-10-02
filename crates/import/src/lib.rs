//! Импорт данных. Пока: разовый перенос из старой таблицы xlsx.

mod legacy;

pub use legacy::parse_legacy;

/// Ошибки разбора. Содержат лист и номер строки, но не содержимое ячеек: текст ошибки
/// может попасть в лог.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImportError {
    #[error("file is not a readable xlsx workbook")]
    NotXlsx,
    #[error("sheet is missing")]
    MissingSheet(&'static str),
    #[error("unexpected layout")]
    Layout { sheet: &'static str, row: u32 },
    #[error("unknown status")]
    UnknownStatus { sheet: &'static str, row: u32 },
    #[error("unknown category kind")]
    UnknownKind { row: u32 },
    #[error("number out of range")]
    Number { sheet: &'static str, row: u32 },
    #[error("no month section with a year found")]
    NoYear,
}
