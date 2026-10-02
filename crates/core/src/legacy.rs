//! Результат разбора старой таблицы xlsx-файл.
//! Чистые данные: парсер лежит в `crates/import`, запись в базу — в `crates/storage`.

use crate::model::{BasisPoints, CategoryKind, IncomeStatus, TxStatus};
use crate::money::Money;
use crate::period::YearMonth;

/// Строка листа «Справочник».
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LegacyCategory {
    pub name: String,
    pub kind: CategoryKind,
    /// `None` у категории сбережений: её план — процент от дохода.
    pub limit: Option<Money>,
    pub note: Option<String>,
}

/// Настройки из блока «Справочник» → колонки M:N.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LegacySettings {
    pub savings_min: BasisPoints,
    pub savings_norm: BasisPoints,
    pub savings_max: BasisPoints,
    pub bonds_rate: BasisPoints,
    pub bonds_coupon_tax: BasisPoints,
    pub bonds_initial_balance: Money,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LegacyTransaction {
    pub month: YearMonth,
    /// Имя категории как в файле: сопоставление с базой идёт при записи.
    pub category: String,
    pub title: String,
    pub amount: Money,
    pub status: TxStatus,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LegacyIncome {
    pub month: YearMonth,
    pub source_name: String,
    pub amount: Money,
    pub status: IncomeStatus,
}

/// Блок категории, у которого «Итого» в файле не равно сумме разобранных позиций.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BlockMismatch {
    pub month: YearMonth,
    pub category: String,
    pub declared: Money,
    pub parsed: Money,
}

/// Всё, что удалось прочитать из книги.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct LegacyBook {
    pub categories: Vec<LegacyCategory>,
    pub settings: Option<LegacySettings>,
    pub transactions: Vec<LegacyTransaction>,
    pub incomes: Vec<LegacyIncome>,
    /// Сверка с итогами блоков самой таблицы: пусто, если все «Итого» сошлись.
    pub block_mismatches: Vec<BlockMismatch>,
}

impl LegacyBook {
    /// Первый месяц, в котором есть траты или доходы: с него действуют лимиты.
    #[must_use]
    pub fn first_month(&self) -> Option<YearMonth> {
        self.transactions
            .iter()
            .map(|t| t.month)
            .chain(self.incomes.iter().map(|i| i.month))
            .min()
    }
}
