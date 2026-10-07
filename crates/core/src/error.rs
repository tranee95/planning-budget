//! Ошибки крейта. Тексты без введённых пользователем значений (PII).

/// Ошибки арифметики и разбора денег.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MoneyError {
    #[error("money overflow")]
    Overflow,
    #[error("invalid money amount")]
    Invalid,
    #[error("division by zero")]
    DivisionByZero,
}

/// Единая ошибка `planning-budget-core`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoreError {
    #[error(transparent)]
    Money(#[from] MoneyError),
    #[error("invalid month")]
    Month,
    #[error("category {0} not found")]
    CategoryNotFound(i64),
    #[error("invalid settings: {0}")]
    Settings(&'static str),
    #[error("seed is not valid json at line {line}, column {column}")]
    SeedFormat { line: usize, column: usize },
    #[error("seed refers to an unknown category (record {index})")]
    SeedUnknownCategory { index: usize },
    #[error("invalid repayment schedule: {0}")]
    Schedule(&'static str),
}
