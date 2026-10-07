//! Домен, деньги и расчёты Planning Budget: чистый крейт без I/O.

pub mod analytics;
pub mod calc;
mod error;
mod legacy;
mod model;
mod money;
mod period;
pub mod query;
mod seed;

pub use error::{CoreError, MoneyError};
pub use legacy::{
    BlockMismatch, LegacyBook, LegacyCategory, LegacyIncome, LegacySettings, LegacyTransaction,
};
pub use model::{
    BasisPoints, Category, CategoryId, CategoryKind, DataSet, Debt, DebtId, DebtPayment,
    DebtPaymentStatus, Income, IncomeId, IncomeStatus, LimitEntry, LockedPlan, SavingsParams,
    SavingsRateEntry, Settings, Tag, TagId, Transaction, TxId, TxStatus,
};
pub use money::Money;
pub use period::YearMonth;
pub use seed::load_seed;
