//! План месяца: фиксация «План готов», разблокировка, копирование плана.
//!
//! Что именно фиксировать, решает `core::calc::Ledger::lock_snapshot`; здесь только запись.

use chrono::{DateTime, Utc};
use planning_budget_core::calc::Ledger;
use planning_budget_core::{BasisPoints, CategoryId, CategoryKind, Money, YearMonth};
use rusqlite::params;

use crate::records::{validate_amount, validate_title};
use crate::writes::{
    NewIncome, NewTransaction, insert_income, insert_transaction, upsert_rate_fixed,
    upsert_rate_percent,
};
use crate::{Db, StorageError, stamp};

/// План накопления в мастере: процент от дохода или фиксированная сумма.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SavingsPlanInput {
    Percent(BasisPoints),
    Fixed(Money),
}

/// Что вводит мастер первого месяца: доходы, суммы по статьям, план накоплений.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct PlanWizardInput {
    /// Ожидаемые поступления месяца: название и сумма.
    pub incomes: Vec<(String, Money)>,
    /// Плановые суммы по статьям расходов.
    pub lines: Vec<(CategoryId, Money)>,
    /// План по накоплениям с этого месяца.
    pub savings: Vec<(CategoryId, SavingsPlanInput)>,
}

impl Db {
    /// Проверяет ввод мастера и возвращает названия статей для строк плана.
    fn plan_wizard_check(&self, input: &PlanWizardInput) -> Result<Vec<String>, StorageError> {
        for (name, amount) in &input.incomes {
            validate_title(name)?;
            validate_amount(*amount)?;
        }
        let mut names = Vec::with_capacity(input.lines.len());
        for (category, amount) in &input.lines {
            validate_amount(*amount)?;
            let category = self.category(*category)?;
            if category.archived {
                return Err(StorageError::Invalid("record.category_archived"));
            }
            if category.kind == CategoryKind::Savings {
                return Err(StorageError::Invalid("plan.line_is_savings"));
            }
            names.push(category.name);
        }
        for (category, plan) in &input.savings {
            self.require_kind(*category, true)?;
            match plan {
                SavingsPlanInput::Percent(rate) if !(0..=10_000).contains(&rate.0) => {
                    return Err(StorageError::Invalid("savings.rate_out_of_range"));
                }
                SavingsPlanInput::Fixed(amount) if amount.is_negative() => {
                    return Err(StorageError::Invalid("savings.negative_amount"));
                }
                _ => {}
            }
        }
        Ok(names)
    }

    /// Мастер первого месяца: ожидаемые доходы, плановые траты и план накоплений создаются одной
    /// транзакцией, затем план фиксируется («План готов»).
    ///
    /// # Errors
    /// `Conflict("plan.locked")`, если месяц уже зафиксирован; `Conflict("plan.not_empty")`, если в нём
    /// есть плановые траты; `Invalid` при неверном вводе.
    pub fn plan_wizard_apply(
        &mut self,
        month: YearMonth,
        input: &PlanWizardInput,
        now: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        if self.plan_is_locked(month)? {
            return Err(StorageError::Conflict("plan.locked"));
        }
        let planned: i64 = self.conn.query_row(
            "SELECT count(*) FROM v_transactions WHERE month = ?1 AND status = 'planned'",
            [month.to_string()],
            |r| r.get(0),
        )?;
        if planned > 0 {
            return Err(StorageError::Conflict("plan.not_empty"));
        }
        let names = self.plan_wizard_check(input)?;

        let stamp = stamp(now);
        let month_text = month.to_string();
        let tx = self.conn.transaction()?;
        for (name, amount) in &input.incomes {
            insert_income(
                &tx,
                &NewIncome {
                    id: None,
                    month: &month_text,
                    source_name: name.trim(),
                    amount: amount.kopecks(),
                    status: "expected",
                    source: "manual",
                    stamp: &stamp,
                },
            )?;
        }
        for ((category, amount), name) in input.lines.iter().zip(&names) {
            insert_transaction(
                &tx,
                &NewTransaction {
                    id: None,
                    month: &month_text,
                    category_id: category.0,
                    title: name,
                    amount: amount.kopecks(),
                    status: "planned",
                    source: "manual",
                    stamp: &stamp,
                },
            )?;
        }
        for (category, plan) in &input.savings {
            match plan {
                SavingsPlanInput::Percent(rate) => {
                    upsert_rate_percent(&tx, category.0, &month_text, i64::from(rate.0))?;
                }
                SavingsPlanInput::Fixed(amount) => {
                    upsert_rate_fixed(&tx, category.0, &month_text, amount.kopecks())?;
                }
            }
        }
        // Ручной процент этого месяца перекрыл бы введённый в мастере план.
        for (category, _) in &input.savings {
            tx.execute(
                "DELETE FROM savings_plan_overrides WHERE month = ?1 AND category_id = ?2",
                params![month_text, category.0],
            )?;
        }
        tx.commit()?;
        self.plan_lock(month, now)
    }

    /// Месяц зафиксирован кнопкой «План готов».
    ///
    /// # Errors
    /// Ошибка SQLite.
    pub fn plan_is_locked(&self, month: YearMonth) -> Result<bool, StorageError> {
        Ok(self.conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM month_plans WHERE month = ?1)",
            [month.to_string()],
            |r| r.get(0),
        )?)
    }

    /// Фиксирует план месяца: плановые суммы трат, план накоплений и погашений на этот момент.
    ///
    /// # Errors
    /// `Conflict("plan.already_locked")`, если месяц уже зафиксирован.
    pub fn plan_lock(&mut self, month: YearMonth, now: DateTime<Utc>) -> Result<(), StorageError> {
        if self.plan_is_locked(month)? {
            return Err(StorageError::Conflict("plan.already_locked"));
        }
        let data = self.dataset(month, month)?;
        let ledger = Ledger::new(&data).map_err(|_| StorageError::Corrupt)?;
        let snapshot = ledger
            .lock_snapshot(month)
            .map_err(|_| StorageError::Corrupt)?;

        let tx = self.conn.transaction()?;
        // Повторная фиксация после разблокировки считает план заново: старые значения не остаются.
        tx.execute(
            "UPDATE transactions SET planned_amount = NULL WHERE month = ?1",
            [month.to_string()],
        )?;
        for (id, amount) in &snapshot.planned_amounts {
            tx.execute(
                "UPDATE transactions SET planned_amount = ?2 WHERE id = ?1",
                params![id.0, amount.kopecks()],
            )?;
        }
        tx.execute(
            "INSERT INTO month_plans (month, locked_at, repayments_planned) VALUES (?1, ?2, ?3)",
            params![
                month.to_string(),
                stamp(now),
                snapshot.repayments_planned.kopecks()
            ],
        )?;
        for (category, amount) in &snapshot.savings {
            tx.execute(
                "INSERT INTO month_plan_savings (month, category_id, amount) VALUES (?1, ?2, ?3)",
                params![month.to_string(), category.0, amount.kopecks()],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Снимает фиксацию: план снова читается из текущих сумм.
    ///
    /// # Errors
    /// `Conflict("plan.not_locked")`, если месяц не зафиксирован.
    pub fn plan_unlock(&mut self, month: YearMonth) -> Result<(), StorageError> {
        let deleted = self.conn.execute(
            "DELETE FROM month_plans WHERE month = ?1",
            [month.to_string()],
        )?;
        if deleted == 0 {
            return Err(StorageError::Conflict("plan.not_locked"));
        }
        Ok(())
    }

    /// «Скопировать план из прошлого месяца»: плановые строки `from` создаются в `to` со статусом
    /// «План» (архивные категории пропускаются). Возвращает число созданных строк.
    ///
    /// # Errors
    /// `Invalid("plan.same_month")`; `Conflict("plan.locked")`, если `to` зафиксирован;
    /// `Conflict("plan.not_empty")`, если в `to` уже есть плановые строки.
    pub fn plan_copy_from(
        &mut self,
        from: YearMonth,
        to: YearMonth,
        now: DateTime<Utc>,
    ) -> Result<usize, StorageError> {
        if from == to {
            return Err(StorageError::Invalid("plan.same_month"));
        }
        if self.plan_is_locked(to)? {
            return Err(StorageError::Conflict("plan.locked"));
        }
        let planned: i64 = self.conn.query_row(
            "SELECT count(*) FROM v_transactions WHERE month = ?1 AND status = 'planned'",
            [to.to_string()],
            |r| r.get(0),
        )?;
        if planned > 0 {
            return Err(StorageError::Conflict("plan.not_empty"));
        }

        let data = self.dataset(from, from)?;
        let rows = Ledger::new(&data)
            .map_err(|_| StorageError::Corrupt)?
            .plan_copy_rows(from);
        let archived: std::collections::BTreeSet<i64> = data
            .categories
            .iter()
            .filter(|c| c.archived)
            .map(|c| c.id.0)
            .collect();

        let stamp = stamp(now);
        let month = to.to_string();
        let tx = self.conn.transaction()?;
        let mut created = 0;
        for row in rows.iter().filter(|r| !archived.contains(&r.category_id.0)) {
            insert_transaction(
                &tx,
                &NewTransaction {
                    id: None,
                    month: &month,
                    category_id: row.category_id.0,
                    title: &row.title,
                    amount: row.amount.kopecks(),
                    status: "planned",
                    source: "manual",
                    stamp: &stamp,
                },
            )?;
            created += 1;
        }
        tx.commit()?;
        Ok(created)
    }
}
