//! Категории, лимиты, проценты плана сбережений, теги. Деньги — копейки в `number` (< 2^53).

use planning_budget_core::{Category, CategoryKind, Tag};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::double_option;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum CategoryKindDto {
    Mandatory,
    Wants,
    Savings,
    Loans,
}

impl From<CategoryKind> for CategoryKindDto {
    fn from(kind: CategoryKind) -> Self {
        match kind {
            CategoryKind::Mandatory => Self::Mandatory,
            CategoryKind::Wants => Self::Wants,
            CategoryKind::Savings => Self::Savings,
            CategoryKind::Loans => Self::Loans,
        }
    }
}

impl From<CategoryKindDto> for CategoryKind {
    fn from(kind: CategoryKindDto) -> Self {
        match kind {
            CategoryKindDto::Mandatory => Self::Mandatory,
            CategoryKindDto::Wants => Self::Wants,
            CategoryKindDto::Savings => Self::Savings,
            CategoryKindDto::Loans => Self::Loans,
        }
    }
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub name: String,
    pub kind: CategoryKindDto,
    pub color: String,
    pub sort_order: i32,
    pub note: Option<String>,
    pub archived: bool,
}

impl From<&Category> for CategoryDto {
    fn from(c: &Category) -> Self {
        Self {
            id: c.id.0,
            name: c.name.clone(),
            kind: c.kind.into(),
            color: c.color.clone(),
            sort_order: c.sort_order,
            note: c.note.clone(),
            archived: c.archived,
        }
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CategoryInput {
    pub name: String,
    pub kind: CategoryKindDto,
    pub color: String,
    pub note: Option<String>,
}

/// Правка категории: отсутствующее поле не меняется; `note: null` очищает заметку.
#[derive(Debug, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct CategoryPatchDto {
    pub name: Option<String>,
    pub color: Option<String>,
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>)]
    pub note: Option<Option<String>>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LimitEntryDto {
    pub valid_from: String,
    /// `None` — «лимита нет с этого месяца».
    #[specta(type = Option<specta_typescript::Number>)]
    pub amount: Option<i64>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TagDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub name: String,
}

impl From<&Tag> for TagDto {
    fn from(t: &Tag) -> Self {
        Self {
            id: t.id.0,
            name: t.name.clone(),
        }
    }
}
