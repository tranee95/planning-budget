//! Раздел «Сбережения»: накопленное за период и взносы по месяцам.

use serde::Serialize;
use specta::Type;

use super::ChartDataDto;

/// Взносы месяца и накопленное к его концу.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SavingsProgressMonthDto {
    pub month: String,
    #[specta(type = specta_typescript::Number)]
    pub deposited: i64,
    #[specta(type = specta_typescript::Number)]
    pub cumulative: i64,
}

/// Накопление в графике «Отложено по месяцам»: в том числе закрытая цель.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SavingsProgressCategoryDto {
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    pub name: String,
    /// Цвет категории (HEX): им рисуется ряд графика.
    pub color: String,
    /// Цель закрыта (категория в архиве).
    pub archived: bool,
    #[specta(type = specta_typescript::Number)]
    pub total: i64,
}

/// Накопленное за весь период.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SavingsProgressDto {
    pub months: Vec<SavingsProgressMonthDto>,
    #[specta(type = specta_typescript::Number)]
    pub total: i64,
    /// Накопления, у которых есть суммы (в том числе закрытые): ряды графика и итоги по ним.
    pub categories: Vec<SavingsProgressCategoryDto>,
    /// Столбцы «Отложено» по месяцам, по ряду на каждое накопление (с накоплением).
    pub chart: ChartDataDto,
}
