//! Доменные типы и `DataSet` — вход всех расчётов.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::money::Money;
use crate::period::YearMonth;

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub i64);
    };
}

id_type!(
    /// Идентификатор категории.
    CategoryId
);
id_type!(
    /// Идентификатор траты.
    TxId
);
id_type!(
    /// Идентификатор дохода.
    IncomeId
);
id_type!(
    /// Идентификатор тега.
    TagId
);

/// Доля в сотых долях процента: 1400 = 14%.
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct BasisPoints(pub i32);

impl BasisPoints {
    pub fn as_ratio(self) -> f64 {
        f64::from(self.0) / 10_000.0
    }
}

/// Тип категории.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CategoryKind {
    Mandatory,
    Wants,
    Savings,
    Loans,
}

/// Статус траты: «цвет» в таблице.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TxStatus {
    Paid,
    Debt,
    Unplanned,
    Planned,
}

/// Статус дохода.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IncomeStatus {
    Received,
    Expected,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Category {
    pub id: CategoryId,
    pub name: String,
    pub kind: CategoryKind,
    pub color: String,
    pub sort_order: i32,
    pub note: Option<String>,
    pub archived: bool,
}

/// Тег траты.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Tag {
    pub id: TagId,
    pub name: String,
}

/// Строка истории лимитов: действует с `valid_from` до следующей строки.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LimitEntry {
    pub category_id: CategoryId,
    pub valid_from: YearMonth,
    pub amount: Money,
}

/// Строка истории процента плана сбережений категории: действует с `valid_from`
/// до следующей строки. До первой строки процент категории равен нулю.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SavingsRateEntry {
    pub category_id: CategoryId,
    pub valid_from: YearMonth,
    pub rate: BasisPoints,
}

/// Неудалённая трата; `amount` положительна, направление задаёт тип записи.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Transaction {
    pub id: TxId,
    pub month: YearMonth,
    pub category_id: CategoryId,
    pub title: String,
    pub amount: Money,
    pub status: TxStatus,
}

/// Неудалённый доход.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Income {
    pub id: IncomeId,
    pub month: YearMonth,
    pub source_name: String,
    pub amount: Money,
    pub status: IncomeStatus,
}

/// Настройки, влияющие на расчёты.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Settings {
    pub savings_min: BasisPoints,
    pub savings_norm: BasisPoints,
    pub savings_max: BasisPoints,
    pub bonds_rate: BasisPoints,
    pub bonds_coupon_tax: BasisPoints,
    pub bonds_initial_balance: Money,
    pub bonds_initial_month: YearMonth,
    pub weeks_per_month: u8,
}

/// Все данные для расчётов. Репозитории кладут сюда только неудалённые записи.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DataSet {
    pub settings: Settings,
    pub categories: Vec<Category>,
    pub limits: Vec<LimitEntry>,
    /// История процентов плана сбережений по категориям типа `savings`.
    pub savings_rates: Vec<SavingsRateEntry>,
    /// Ручной процент плана сбережений категории на месяц.
    pub savings_overrides: BTreeMap<(YearMonth, CategoryId), BasisPoints>,
    pub transactions: Vec<Transaction>,
    pub incomes: Vec<Income>,
}
