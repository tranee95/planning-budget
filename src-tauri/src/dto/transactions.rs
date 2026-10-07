//! Траты и доходы. Месяц — строка `YYYY-MM`, дата — `YYYY-MM-DD`.

use planning_budget_core::{IncomeStatus, TxStatus};
use planning_budget_storage::{IncomeRecord, TransactionRecord};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::double_option;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum TxStatusDto {
    Paid,
    Debt,
    Unplanned,
    Planned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum IncomeStatusDto {
    Received,
    Expected,
}

impl From<TxStatus> for TxStatusDto {
    fn from(status: TxStatus) -> Self {
        match status {
            TxStatus::Paid => Self::Paid,
            TxStatus::Debt => Self::Debt,
            TxStatus::Unplanned => Self::Unplanned,
            TxStatus::Planned => Self::Planned,
        }
    }
}

impl From<TxStatusDto> for TxStatus {
    fn from(status: TxStatusDto) -> Self {
        match status {
            TxStatusDto::Paid => Self::Paid,
            TxStatusDto::Debt => Self::Debt,
            TxStatusDto::Unplanned => Self::Unplanned,
            TxStatusDto::Planned => Self::Planned,
        }
    }
}

impl From<IncomeStatus> for IncomeStatusDto {
    fn from(status: IncomeStatus) -> Self {
        match status {
            IncomeStatus::Received => Self::Received,
            IncomeStatus::Expected => Self::Expected,
        }
    }
}

impl From<IncomeStatusDto> for IncomeStatus {
    fn from(status: IncomeStatusDto) -> Self {
        match status {
            IncomeStatusDto::Received => Self::Received,
            IncomeStatusDto::Expected => Self::Expected,
        }
    }
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub month: String,
    pub date: Option<String>,
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    pub title: String,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
    pub status: TxStatusDto,
    pub comment: Option<String>,
    #[specta(type = Vec<specta_typescript::Number>)]
    pub tag_ids: Vec<i64>,
}

impl From<&TransactionRecord> for TransactionDto {
    fn from(t: &TransactionRecord) -> Self {
        Self {
            id: t.id.0,
            month: t.month.to_string(),
            date: t.date.map(|d| d.to_string()),
            category_id: t.category_id.0,
            title: t.title.clone(),
            amount: t.amount.kopecks(),
            status: t.status.into(),
            comment: t.comment.clone(),
            tag_ids: t.tags.iter().map(|id| id.0).collect(),
        }
    }
}

/// Новая трата. Если задана `date`, её месяц должен совпадать с `month`.
#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TransactionInput {
    pub month: String,
    pub date: Option<String>,
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    pub title: String,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
    pub status: TxStatusDto,
    pub comment: Option<String>,
}

/// Правка траты: отсутствующее поле не меняется; `date`/`comment: null` очищают значение.
#[derive(Debug, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct TransactionPatchDto {
    pub month: Option<String>,
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>)]
    pub date: Option<Option<String>>,
    #[specta(type = Option<specta_typescript::Number>)]
    pub category_id: Option<i64>,
    pub title: Option<String>,
    #[specta(type = Option<specta_typescript::Number>)]
    pub amount: Option<i64>,
    pub status: Option<TxStatusDto>,
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>)]
    pub comment: Option<Option<String>>,
}

/// Состояние до и после правки: по `previous` UI делает отмену.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TxUpdateResultDto {
    pub current: TransactionDto,
    pub previous: TransactionDto,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IncomeDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub month: String,
    pub date: Option<String>,
    pub source_name: String,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
    pub status: IncomeStatusDto,
    pub comment: Option<String>,
}

impl From<&IncomeRecord> for IncomeDto {
    fn from(i: &IncomeRecord) -> Self {
        Self {
            id: i.id.0,
            month: i.month.to_string(),
            date: i.date.map(|d| d.to_string()),
            source_name: i.source_name.clone(),
            amount: i.amount.kopecks(),
            status: i.status.into(),
            comment: i.comment.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IncomeInput {
    pub month: String,
    pub date: Option<String>,
    pub source_name: String,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
    pub status: IncomeStatusDto,
    pub comment: Option<String>,
}

#[derive(Debug, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct IncomePatchDto {
    pub month: Option<String>,
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>)]
    pub date: Option<Option<String>>,
    pub source_name: Option<String>,
    #[specta(type = Option<specta_typescript::Number>)]
    pub amount: Option<i64>,
    pub status: Option<IncomeStatusDto>,
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>)]
    pub comment: Option<Option<String>>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IncomeUpdateResultDto {
    pub current: IncomeDto,
    pub previous: IncomeDto,
}
