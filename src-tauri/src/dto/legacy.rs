//! Отчёт переноса из старой таблицы xlsx.

use serde::Serialize;
use specta::Type;

/// Блок категории, у которого «Итого» в таблице не равно сумме позиций.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BlockMismatchDto {
    pub month: String,
    pub category: String,
    #[specta(type = specta_typescript::Number)]
    pub declared: i64,
    #[specta(type = specta_typescript::Number)]
    pub parsed: i64,
}

/// Итоги года по данным базы после переноса: их пользователь сверяет со своей таблицей.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LegacyTotalsDto {
    pub year: u16,
    #[specta(type = specta_typescript::Number)]
    pub income: i64,
    #[specta(type = specta_typescript::Number)]
    pub expenses: i64,
    #[specta(type = specta_typescript::Number)]
    pub savings: i64,
    #[specta(type = specta_typescript::Number)]
    pub free: i64,
    pub months_with_data: u32,
}

/// Отчёт переноса из старой таблицы.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LegacyReportDto {
    pub categories_created: u32,
    pub categories_matched: u32,
    pub kind_conflicts: Vec<String>,
    pub limits_written: u32,
    pub transactions: u32,
    pub incomes: u32,
    pub settings_applied: bool,
    pub block_mismatches: Vec<BlockMismatchDto>,
    /// `null`, если итоги посчитать не удалось: данные при этом записаны.
    pub totals: Option<LegacyTotalsDto>,
}
