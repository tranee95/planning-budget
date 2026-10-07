//! Снимок данных для расчётов: `DataSet` одним проходом.

use planning_budget_core::{
    CategoryId, DataSet, Debt, DebtId, DebtPayment, DebtPaymentStatus, Income, IncomeId,
    LimitEntry, LockedPlan, Money, Transaction, TxId, YearMonth,
};
use std::collections::BTreeMap;

use rusqlite::params;

use crate::incomes::status_from_str as income_status;
use crate::records::parse_month;
use crate::transactions::status_from_str as tx_status;
use crate::{Db, StorageError};

impl Db {
    /// Данные для расчётов за месяцы `from..=to`: траты и доходы только этого периода,
    /// категории (с архивными), история лимитов и процентов плана и настройки — целиком.
    ///
    /// Четыре запроса к таблицам данных и сборка в Rust, без запросов на каждую запись.
    ///
    /// # Errors
    /// Ошибка SQLite или повреждённая строка.
    pub fn dataset(&self, from: YearMonth, to: YearMonth) -> Result<DataSet, StorageError> {
        Ok(DataSet {
            settings: self.settings()?,
            categories: self.categories(true)?,
            limits: self.all_limits()?,
            savings_rates: self.savings_rates()?,
            savings_overrides: self.savings_overrides()?,
            transactions: self.period_transactions(from, to)?,
            incomes: self.period_incomes(from, to)?,
            debts: self.all_debts()?,
            debt_payments: self.all_debt_payments()?,
            locked_plans: self.locked_plans()?,
            savings_params: self.savings_params()?,
        })
    }

    /// Неудалённые долги.
    fn all_debts(&self) -> Result<Vec<Debt>, StorageError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, lender, amount, taken_month, category_id FROM v_debts ORDER BY id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let month: String = row.get(3)?;
            out.push(Debt {
                id: DebtId(row.get(0)?),
                lender: row.get(1)?,
                amount: Money::from_kopecks(row.get(2)?),
                taken_month: parse_month(&month)?,
                category_id: row.get::<_, Option<i64>>(4)?.map(CategoryId),
            });
        }
        Ok(out)
    }

    /// Строки графиков неудалённых долгов.
    fn all_debt_payments(&self) -> Result<Vec<DebtPayment>, StorageError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT p.debt_id, p.month, p.amount, p.status FROM debt_payments p
             JOIN v_debts d ON d.id = p.debt_id ORDER BY p.debt_id, p.month",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let month: String = row.get(1)?;
            let status: String = row.get(3)?;
            out.push(DebtPayment {
                debt_id: DebtId(row.get(0)?),
                month: parse_month(&month)?,
                amount: Money::from_kopecks(row.get(2)?),
                status: payment_status_from_str(&status)?,
            });
        }
        Ok(out)
    }

    /// Месяцы с зафиксированным планом и снимки накоплений.
    pub(crate) fn locked_plans(&self) -> Result<BTreeMap<YearMonth, LockedPlan>, StorageError> {
        let mut out = BTreeMap::new();
        let mut stmt = self
            .conn
            .prepare_cached("SELECT month, repayments_planned FROM month_plans")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let month: String = row.get(0)?;
            out.insert(
                parse_month(&month)?,
                LockedPlan {
                    repayments_planned: Money::from_kopecks(row.get(1)?),
                    savings: BTreeMap::new(),
                },
            );
        }
        let mut stmt = self
            .conn
            .prepare_cached("SELECT month, category_id, amount FROM month_plan_savings")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let month: String = row.get(0)?;
            if let Some(plan) = out.get_mut(&parse_month(&month)?) {
                plan.savings
                    .insert(CategoryId(row.get(1)?), Money::from_kopecks(row.get(2)?));
            }
        }
        Ok(out)
    }

    fn all_limits(&self) -> Result<Vec<LimitEntry>, StorageError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT category_id, valid_from, amount FROM category_limits ORDER BY category_id, valid_from",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let valid_from: String = row.get(1)?;
            out.push(LimitEntry {
                category_id: CategoryId(row.get(0)?),
                valid_from: parse_month(&valid_from)?,
                amount: Money::from_kopecks(row.get(2)?),
            });
        }
        Ok(out)
    }

    fn period_transactions(
        &self,
        from: YearMonth,
        to: YearMonth,
    ) -> Result<Vec<Transaction>, StorageError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, month, category_id, title, amount, status, planned_amount FROM v_transactions
             WHERE month BETWEEN ?1 AND ?2 ORDER BY month, category_id, sort_key, id",
        )?;
        let mut rows = stmt.query(params![from.to_string(), to.to_string()])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            // строки читаются по ссылке: на 50 000 трат это 100 000 лишних выделений
            let month = row
                .get_ref(1)?
                .as_str()
                .map_err(|_| StorageError::Corrupt)?;
            let status = row
                .get_ref(5)?
                .as_str()
                .map_err(|_| StorageError::Corrupt)?;
            out.push(Transaction {
                id: TxId(row.get(0)?),
                month: parse_month(month)?,
                category_id: CategoryId(row.get(2)?),
                title: row.get(3)?,
                amount: Money::from_kopecks(row.get(4)?),
                status: tx_status(status)?,
                planned_amount: row.get::<_, Option<i64>>(6)?.map(Money::from_kopecks),
            });
        }
        Ok(out)
    }

    fn period_incomes(&self, from: YearMonth, to: YearMonth) -> Result<Vec<Income>, StorageError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, month, source_name, amount, status FROM v_incomes
             WHERE month BETWEEN ?1 AND ?2 ORDER BY month, id",
        )?;
        let mut rows = stmt.query(params![from.to_string(), to.to_string()])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let month = row
                .get_ref(1)?
                .as_str()
                .map_err(|_| StorageError::Corrupt)?;
            let status = row
                .get_ref(4)?
                .as_str()
                .map_err(|_| StorageError::Corrupt)?;
            out.push(Income {
                id: IncomeId(row.get(0)?),
                month: parse_month(month)?,
                source_name: row.get(2)?,
                amount: Money::from_kopecks(row.get(3)?),
                status: income_status(status)?,
            });
        }
        Ok(out)
    }
}

impl Db {
    /// Имена тегов неудалённых трат месяцев `from..=to` (траты без тегов в карту не попадают).
    ///
    /// # Errors
    /// Ошибка SQLite.
    pub fn tx_tag_names(
        &self,
        from: YearMonth,
        to: YearMonth,
    ) -> Result<BTreeMap<TxId, Vec<String>>, StorageError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT tt.transaction_id, g.name FROM transaction_tags tt
             JOIN tags g ON g.id = tt.tag_id
             JOIN v_transactions t ON t.id = tt.transaction_id
             WHERE t.month BETWEEN ?1 AND ?2 ORDER BY tt.transaction_id, norm(g.name), g.id",
        )?;
        let mut rows = stmt.query(params![from.to_string(), to.to_string()])?;
        let mut out: BTreeMap<TxId, Vec<String>> = BTreeMap::new();
        while let Some(row) = rows.next()? {
            out.entry(TxId(row.get(0)?)).or_default().push(row.get(1)?);
        }
        Ok(out)
    }
}

pub(crate) fn payment_status_from_str(s: &str) -> Result<DebtPaymentStatus, StorageError> {
    match s {
        "planned" => Ok(DebtPaymentStatus::Planned),
        "paid" => Ok(DebtPaymentStatus::Paid),
        _ => Err(StorageError::Corrupt),
    }
}

pub(crate) fn payment_status_str(status: DebtPaymentStatus) -> &'static str {
    match status {
        DebtPaymentStatus::Planned => "planned",
        DebtPaymentStatus::Paid => "paid",
    }
}
