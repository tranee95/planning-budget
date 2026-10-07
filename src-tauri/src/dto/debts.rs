//! Долги (займы) и графики погашений.

use planning_budget_core::DebtPaymentStatus;
use serde::{Deserialize, Serialize};
use specta::Type;

use super::double_option;

/// Статус строки графика.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum DebtPaymentStatusDto {
    Planned,
    Paid,
}

impl From<DebtPaymentStatus> for DebtPaymentStatusDto {
    fn from(s: DebtPaymentStatus) -> Self {
        match s {
            DebtPaymentStatus::Planned => Self::Planned,
            DebtPaymentStatus::Paid => Self::Paid,
        }
    }
}

impl From<DebtPaymentStatusDto> for DebtPaymentStatus {
    fn from(s: DebtPaymentStatusDto) -> Self {
        match s {
            DebtPaymentStatusDto::Planned => Self::Planned,
            DebtPaymentStatusDto::Paid => Self::Paid,
        }
    }
}

/// Строка графика погашения.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DebtPaymentDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub month: String,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
    pub status: DebtPaymentStatusDto,
    pub paid_date: Option<String>,
}

/// Долг со всем графиком; остаток и доля погашения посчитаны в Rust.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DebtDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub lender: String,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
    pub taken_month: String,
    pub taken_date: Option<String>,
    #[specta(type = Option<specta_typescript::Number>)]
    pub category_id: Option<i64>,
    #[specta(type = Option<specta_typescript::Number>)]
    pub transaction_id: Option<i64>,
    pub comment: Option<String>,
    /// Все платежи оплачены.
    pub closed: bool,
    #[specta(type = specta_typescript::Number)]
    pub remaining: i64,
    /// Доля погашенного в базисных пунктах (10 000 = долг закрыт).
    pub paid_bp: i32,
    pub next_payment: Option<DebtPaymentDto>,
    pub payments: Vec<DebtPaymentDto>,
}

/// Раздел «Долги» на месяц: список и итоги.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DebtsOverviewDto {
    pub debts: Vec<DebtDto>,
    pub open_count: u32,
    pub closed_count: u32,
    /// Сумма остатков открытых долгов.
    #[specta(type = specta_typescript::Number)]
    pub remaining_total: i64,
    /// К оплате в месяце (все строки графиков месяца).
    #[specta(type = specta_typescript::Number)]
    pub payments_planned: i64,
    /// Из них оплачено.
    #[specta(type = specta_typescript::Number)]
    pub payments_paid: i64,
}

/// Строка графика на входе: месяц и сумма.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SchedulePaymentDto {
    pub month: String,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
}

/// Новый долг; сумма графика должна равняться сумме долга.
#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DebtInput {
    pub lender: String,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
    pub taken_month: String,
    pub taken_date: Option<String>,
    #[specta(type = Option<specta_typescript::Number>)]
    pub category_id: Option<i64>,
    pub comment: Option<String>,
    pub schedule: Vec<SchedulePaymentDto>,
}

/// Правка долга: отсутствующее поле не меняется; `categoryId`/`comment: null` очищают значение.
/// `schedule` заменяет только неоплаченные строки.
#[derive(Debug, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct DebtPatchDto {
    pub lender: Option<String>,
    #[specta(type = Option<specta_typescript::Number>)]
    pub amount: Option<i64>,
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<Option<specta_typescript::Number>>)]
    pub category_id: Option<Option<i64>>,
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>)]
    pub comment: Option<Option<String>>,
    pub schedule: Option<Vec<SchedulePaymentDto>>,
}

/// Быстрый график погашения (считает Rust).
#[derive(Debug, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DebtScheduleKindDto {
    /// Равными частями на `months` месяцев, начиная со следующего за месяцем займа.
    EqualParts { months: u32 },
    /// Одним платежом в месяце `month`.
    Single { month: String },
}
