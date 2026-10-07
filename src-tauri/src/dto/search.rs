//! Поиск и фильтры.

use planning_budget_core::query::{QueryErrorKind, TokenKind};
use serde::Serialize;
use specta::Type;

use super::{IncomeDto, IncomeStatusDto, TransactionDto, TxStatusDto};

/// Чем распознан токен запроса; UI красит токены по этому полю.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum TokenKindDto {
    Text,
    Amount,
    Status,
    Category,
    Kind,
    Month,
    Tag,
    Source,
    Record,
}

impl From<TokenKind> for TokenKindDto {
    fn from(kind: TokenKind) -> Self {
        match kind {
            TokenKind::Text => Self::Text,
            TokenKind::Amount => Self::Amount,
            TokenKind::Status => Self::Status,
            TokenKind::Category => Self::Category,
            TokenKind::Kind => Self::Kind,
            TokenKind::Month => Self::Month,
            TokenKind::Tag => Self::Tag,
            TokenKind::Source => Self::Source,
            TokenKind::Record => Self::Record,
        }
    }
}

/// Причина мягкой подсказки: токен с известным ключом не распознан и ушёл в текст.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum QueryHintDto {
    EmptyValue,
    UnknownValue,
    BadAmount,
    BadMonth,
    NegationUnsupported,
}

impl From<QueryErrorKind> for QueryHintDto {
    fn from(kind: QueryErrorKind) -> Self {
        match kind {
            QueryErrorKind::EmptyValue => Self::EmptyValue,
            QueryErrorKind::UnknownValue => Self::UnknownValue,
            QueryErrorKind::BadAmount => Self::BadAmount,
            QueryErrorKind::BadMonth => Self::BadMonth,
            QueryErrorKind::NegationUnsupported => Self::NegationUnsupported,
        }
    }
}

/// Токен запроса: `start..end` — позиции в символах (не в UTF-16 и не в байтах).
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TokenSpanDto {
    pub start: u32,
    pub end: u32,
    pub kind: TokenKindDto,
    pub negated: bool,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QueryHintSpanDto {
    pub start: u32,
    pub end: u32,
    pub hint: QueryHintDto,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TransactionHitDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub title: String,
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    pub category: String,
    pub month: String,
    pub date: Option<String>,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
    pub status: TxStatusDto,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IncomeHitDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub source_name: String,
    pub month: String,
    pub date: Option<String>,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
    pub status: IncomeStatusDto,
}

/// Группа трат: `total` и `sum` — по всем совпадениям, `items` — первые строки.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TransactionGroupDto {
    #[specta(type = specta_typescript::Number)]
    pub total: u64,
    #[specta(type = specta_typescript::Number)]
    pub sum: i64,
    pub items: Vec<TransactionHitDto>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IncomeGroupDto {
    #[specta(type = specta_typescript::Number)]
    pub total: u64,
    #[specta(type = specta_typescript::Number)]
    pub sum: i64,
    pub items: Vec<IncomeHitDto>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CategoryHitDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MonthHitDto {
    pub month: String,
    #[specta(type = specta_typescript::Number)]
    pub records: u64,
}

/// Ответ палитры. `request_id` возвращается как есть: UI отбрасывает устаревшие ответы.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultDto {
    pub request_id: u32,
    pub spans: Vec<TokenSpanDto>,
    pub hints: Vec<QueryHintSpanDto>,
    pub transactions: TransactionGroupDto,
    pub incomes: IncomeGroupDto,
    pub categories: Vec<CategoryHitDto>,
    pub months: Vec<MonthHitDto>,
}

/// Полный список трат по запросу для таблицы «Расходы»; `total` и `sum` — по всем
/// совпадениям, `truncated` — строк больше, чем отдано.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TransactionSearchDto {
    pub request_id: u32,
    pub spans: Vec<TokenSpanDto>,
    pub hints: Vec<QueryHintSpanDto>,
    #[specta(type = specta_typescript::Number)]
    pub total: u64,
    #[specta(type = specta_typescript::Number)]
    pub sum: i64,
    pub truncated: bool,
    pub items: Vec<TransactionDto>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IncomeSearchDto {
    pub request_id: u32,
    pub spans: Vec<TokenSpanDto>,
    pub hints: Vec<QueryHintSpanDto>,
    #[specta(type = specta_typescript::Number)]
    pub total: u64,
    #[specta(type = specta_typescript::Number)]
    pub sum: i64,
    pub truncated: bool,
    pub items: Vec<IncomeDto>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SavedFilterDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub name: String,
    pub query: String,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TitleSuggestionDto {
    pub title: String,
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    #[specta(type = specta_typescript::Number)]
    pub uses: u64,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CategoryUsageDto {
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    #[specta(type = specta_typescript::Number)]
    pub uses: u64,
}
