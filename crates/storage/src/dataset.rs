//! Снимок данных для расчётов: `DataSet` одним проходом.

use budget_core::{
    CategoryId, DataSet, Income, IncomeId, LimitEntry, Money, Transaction, TxId, YearMonth,
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
        })
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
            "SELECT id, month, category_id, title, amount, status FROM v_transactions
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
