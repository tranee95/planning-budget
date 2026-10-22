//! Лимиты категорий: история с `valid_from`.

use planning_budget_core::{CategoryId, CategoryKind, LimitEntry, Money, YearMonth};
use rusqlite::params;

use crate::records::parse_month;
use crate::{Db, StorageError};

impl Db {
    pub(crate) fn require_kind(&self, id: CategoryId, savings: bool) -> Result<(), StorageError> {
        let is_savings = self.category(id)?.kind == CategoryKind::Savings;
        match (savings, is_savings) {
            (true, false) => Err(StorageError::Invalid("savings.not_a_savings_category")),
            (false, true) => Err(StorageError::Invalid("limit.savings_has_rate")),
            _ => Ok(()),
        }
    }

    /// Задаёт лимит категории с месяца `valid_from`; повторный вызов с тем же месяцем
    /// перезаписывает сумму. У категории сбережений лимита нет: у неё процент плана.
    ///
    /// # Errors
    /// `NotFound`; `Invalid` при отрицательной сумме или категории сбережений.
    pub fn limit_set(
        &mut self,
        category: CategoryId,
        valid_from: YearMonth,
        amount: Money,
    ) -> Result<(), StorageError> {
        if amount.is_negative() {
            return Err(StorageError::Invalid("limit.negative"));
        }
        self.require_kind(category, false)?;
        self.conn.execute(
            "INSERT INTO category_limits (category_id, valid_from, amount) VALUES (?1, ?2, ?3)
             ON CONFLICT (category_id, valid_from) DO UPDATE SET amount = excluded.amount",
            params![category.0, valid_from.to_string(), amount.kopecks()],
        )?;
        Ok(())
    }

    /// Лимита нет с месяца `valid_from` и до следующей строки истории (запись «без лимита»).
    ///
    /// # Errors
    /// `NotFound`; `Invalid` для категории сбережений.
    pub fn limit_unset(
        &mut self,
        category: CategoryId,
        valid_from: YearMonth,
    ) -> Result<(), StorageError> {
        self.require_kind(category, false)?;
        self.conn.execute(
            "INSERT INTO category_limits (category_id, valid_from, amount) VALUES (?1, ?2, NULL)
             ON CONFLICT (category_id, valid_from) DO UPDATE SET amount = NULL",
            params![category.0, valid_from.to_string()],
        )?;
        Ok(())
    }

    /// Убирает строку истории. Лимит на месяцы после неё вернётся к предыдущей строке.
    ///
    /// # Errors
    /// `NotFound`, если такой строки нет.
    pub fn limit_clear(
        &mut self,
        category: CategoryId,
        valid_from: YearMonth,
    ) -> Result<(), StorageError> {
        let deleted = self.conn.execute(
            "DELETE FROM category_limits WHERE category_id = ?1 AND valid_from = ?2",
            params![category.0, valid_from.to_string()],
        )?;
        if deleted == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    /// История лимитов категории по возрастанию `valid_from`.
    ///
    /// # Errors
    /// Ошибка SQLite или повреждённая строка.
    pub fn limit_history(&self, category: CategoryId) -> Result<Vec<LimitEntry>, StorageError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT valid_from, amount FROM category_limits WHERE category_id = ?1 ORDER BY valid_from",
        )?;
        let mut rows = stmt.query([category.0])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let valid_from: String = row.get(0)?;
            out.push(LimitEntry {
                category_id: category,
                valid_from: parse_month(&valid_from)?,
                amount: row.get::<_, Option<i64>>(1)?.map(Money::from_kopecks),
            });
        }
        Ok(out)
    }
}
