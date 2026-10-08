//! Загрузка `seed-2026.json` в `DataSet`. Без I/O: байты передаёт вызывающий.

use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;

use crate::error::CoreError;
use crate::model::{
    BasisPoints, Category, CategoryId, CategoryKind, DataSet, Income, IncomeId, IncomeStatus,
    LimitEntry, SavingsParams, SavingsRateEntry, Settings, Transaction, TxId, TxStatus,
};
use crate::money::Money;
use crate::period::YearMonth;

#[derive(Deserialize)]
struct SeedFile {
    settings: SeedSettings,
    categories: Vec<SeedCategory>,
    limits_valid_from: YearMonth,
    transactions: Vec<SeedTransaction>,
    incomes: Vec<SeedIncome>,
}

#[derive(Deserialize)]
struct SeedSettings {
    #[serde(rename = "savings.target_min_bp")]
    savings_min: i32,
    #[serde(rename = "savings.target_norm_bp")]
    savings_norm: i32,
    #[serde(rename = "savings.target_max_bp")]
    savings_max: i32,
    #[serde(rename = "bonds.rate_bp")]
    bonds_rate: i32,
    #[serde(rename = "bonds.coupon_tax_bp")]
    bonds_coupon_tax: i32,
    #[serde(rename = "bonds.initial_balance")]
    bonds_initial_balance: i64,
    #[serde(rename = "bonds.initial_month")]
    bonds_initial_month: YearMonth,
    #[serde(rename = "ui.weeks_per_month")]
    weeks_per_month: u8,
}

#[derive(Deserialize)]
struct SeedCategory {
    name: String,
    kind: CategoryKind,
    limit: Option<i64>,
    sort_order: i32,
    color: String,
    note: Option<String>,
}

#[derive(Deserialize)]
struct SeedTransaction {
    month: YearMonth,
    category: String,
    title: String,
    amount: i64,
    status: TxStatus,
}

#[derive(Deserialize)]
struct SeedIncome {
    month: YearMonth,
    source: String,
    amount: i64,
    status: IncomeStatus,
}

/// Разбирает сид. Идентификаторы выдаются по порядку записей, начиная с 1.
pub fn load_seed(bytes: &[u8]) -> Result<DataSet, CoreError> {
    let seed: SeedFile = serde_json::from_slice(bytes).map_err(|e| CoreError::SeedFormat {
        line: e.line(),
        column: e.column(),
    })?;

    let mut categories = Vec::with_capacity(seed.categories.len());
    let mut limits = Vec::new();
    let mut savings_rates = Vec::new();
    let mut savings_params = BTreeMap::new();
    let mut by_name = HashMap::with_capacity(seed.categories.len());
    for (index, c) in seed.categories.into_iter().enumerate() {
        let id = CategoryId(
            i64::try_from(index).map_err(|_| CoreError::Settings("too many categories"))? + 1,
        );
        if c.kind == CategoryKind::Savings {
            savings_rates.push(SavingsRateEntry {
                category_id: id,
                valid_from: seed.limits_valid_from,
                rate: BasisPoints(seed.settings.savings_norm),
                fixed_amount: None,
            });
            savings_params.insert(
                id,
                SavingsParams {
                    annual_rate: BasisPoints(seed.settings.bonds_rate),
                    tax: BasisPoints(seed.settings.bonds_coupon_tax),
                    initial_balance: Money::from_kopecks(seed.settings.bonds_initial_balance),
                    initial_month: seed.settings.bonds_initial_month,
                },
            );
        } else if let Some(amount) = c.limit {
            limits.push(LimitEntry {
                category_id: id,
                valid_from: seed.limits_valid_from,
                amount: Some(Money::from_kopecks(amount)),
            });
        }
        by_name.insert(c.name.clone(), id);
        categories.push(Category {
            id,
            name: c.name,
            kind: c.kind,
            color: c.color,
            sort_order: c.sort_order,
            note: c.note,
            archived: false,
        });
    }

    let mut transactions = Vec::with_capacity(seed.transactions.len());
    for (index, t) in seed.transactions.into_iter().enumerate() {
        let category_id = *by_name
            .get(&t.category)
            .ok_or(CoreError::SeedUnknownCategory { index })?;
        transactions.push(Transaction {
            id: TxId(
                i64::try_from(index).map_err(|_| CoreError::Settings("too many records"))? + 1,
            ),
            month: t.month,
            category_id,
            title: t.title,
            amount: Money::from_kopecks(t.amount),
            status: t.status,
            planned_amount: None,
        });
    }

    let mut incomes = Vec::with_capacity(seed.incomes.len());
    for (index, i) in seed.incomes.into_iter().enumerate() {
        incomes.push(Income {
            id: IncomeId(
                i64::try_from(index).map_err(|_| CoreError::Settings("too many records"))? + 1,
            ),
            month: i.month,
            source_name: i.source,
            amount: Money::from_kopecks(i.amount),
            status: i.status,
        });
    }

    let s = seed.settings;
    Ok(DataSet {
        settings: Settings {
            savings_min: BasisPoints(s.savings_min),
            savings_norm: BasisPoints(s.savings_norm),
            savings_max: BasisPoints(s.savings_max),
            weeks_per_month: s.weeks_per_month,
        },
        categories,
        limits,
        savings_rates,
        savings_overrides: BTreeMap::new(),
        transactions,
        incomes,
        debts: Vec::new(),
        debt_payments: Vec::new(),
        locked_plans: BTreeMap::new(),
        savings_params,
    })
}
