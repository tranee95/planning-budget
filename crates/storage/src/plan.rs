//! Лимиты категорий и план сбережений: история с `valid_from` и ручные проценты месяца
//!.

use budget_core::{
    BasisPoints, CategoryId, CategoryKind, LimitEntry, Money, SavingsRateEntry, YearMonth,
};
use rusqlite::params;

use crate::records::parse_month;

use crate::{Db, StorageError};

fn validate_rate(rate: BasisPoints) -> Result<(), StorageError> {
    if (0..=10_000).contains(&rate.0) {
        Ok(())
    } else {
        Err(StorageError::Invalid("savings.rate_out_of_range"))
    }
}

impl Db {
    fn require_kind(&self, id: CategoryId, savings: bool) -> Result<(), StorageError> {
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
                amount: Money::from_kopecks(row.get(1)?),
            });
        }
        Ok(out)
    }

    /// Задаёт процент плана категории сбережений с месяца `valid_from`.
    ///
    /// # Errors
    /// `NotFound`; `Invalid`, если процент вне 0–100% или категория не сбережения.
    pub fn savings_rate_set(
        &mut self,
        category: CategoryId,
        valid_from: YearMonth,
        rate: BasisPoints,
    ) -> Result<(), StorageError> {
        validate_rate(rate)?;
        self.require_kind(category, true)?;
        self.conn.execute(
            "INSERT INTO savings_category_rates (category_id, valid_from, rate_bp) VALUES (?1, ?2, ?3)
             ON CONFLICT (category_id, valid_from) DO UPDATE SET rate_bp = excluded.rate_bp",
            params![category.0, valid_from.to_string(), rate.0],
        )?;
        Ok(())
    }

    /// История процентов плана всех категорий сбережений.
    ///
    /// # Errors
    /// Ошибка SQLite или повреждённая строка.
    pub fn savings_rates(&self) -> Result<Vec<SavingsRateEntry>, StorageError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT category_id, valid_from, rate_bp FROM savings_category_rates
             ORDER BY category_id, valid_from",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let valid_from: String = row.get(1)?;
            out.push(SavingsRateEntry {
                category_id: CategoryId(row.get(0)?),
                valid_from: parse_month(&valid_from)?,
                rate: BasisPoints(row.get(2)?),
            });
        }
        Ok(out)
    }

    /// Ручной процент плана категории на один месяц.
    ///
    /// # Errors
    /// `NotFound`; `Invalid`, если процент вне 0–100% или категория не сбережения.
    pub fn savings_override_set(
        &mut self,
        month: YearMonth,
        category: CategoryId,
        rate: BasisPoints,
    ) -> Result<(), StorageError> {
        validate_rate(rate)?;
        self.require_kind(category, true)?;
        self.conn.execute(
            "INSERT INTO savings_plan_overrides (month, category_id, rate_bp) VALUES (?1, ?2, ?3)
             ON CONFLICT (month, category_id) DO UPDATE SET rate_bp = excluded.rate_bp",
            params![month.to_string(), category.0, rate.0],
        )?;
        Ok(())
    }

    /// Снимает ручной процент: месяц снова считается по истории процентов.
    ///
    /// # Errors
    /// `NotFound`, если ручного процента нет.
    pub fn savings_override_clear(
        &mut self,
        month: YearMonth,
        category: CategoryId,
    ) -> Result<(), StorageError> {
        let deleted = self.conn.execute(
            "DELETE FROM savings_plan_overrides WHERE month = ?1 AND category_id = ?2",
            params![month.to_string(), category.0],
        )?;
        if deleted == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    /// Все ручные проценты: `(месяц, категория) → процент`.
    ///
    /// # Errors
    /// Ошибка SQLite или повреждённая строка.
    pub fn savings_overrides(
        &self,
    ) -> Result<std::collections::BTreeMap<(YearMonth, CategoryId), BasisPoints>, StorageError>
    {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT month, category_id, rate_bp FROM savings_plan_overrides")?;
        let mut rows = stmt.query([])?;
        let mut out = std::collections::BTreeMap::new();
        while let Some(row) = rows.next()? {
            let month: String = row.get(0)?;
            out.insert(
                (parse_month(&month)?, CategoryId(row.get(1)?)),
                BasisPoints(row.get(2)?),
            );
        }
        Ok(out)
    }
}
