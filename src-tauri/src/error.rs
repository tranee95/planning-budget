//! Ошибка на границе IPC. Только коды и id: без текстов и PII.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use planning_budget_storage::StorageError;
use serde::Serialize;
use specta::Type;

#[derive(Debug, Serialize, Type, thiserror::Error)]
#[serde(
    tag = "code",
    rename_all = "PascalCase",
    rename_all_fields = "camelCase"
)]
pub enum AppError {
    #[error("locked")]
    Locked,
    #[error("wrong password")]
    WrongPassword { retry_after_ms: u32 },
    /// Попытка в окне задержки: пароль не проверялся.
    #[error("too many attempts")]
    TooManyAttempts { retry_after_ms: u32 },
    #[error("validation")]
    Validation {
        message_key: String,
        field: Option<String>,
    },
    #[error("not found")]
    NotFound {
        entity: String,
        // id < 2^53: потери точности в JS number нет
        #[specta(type = specta_typescript::Number)]
        id: i64,
    },
    #[error("conflict")]
    Conflict { message_key: String },
    #[error("import")]
    Import {
        message_key: String,
        line: Option<u32>,
    },
    #[error("io")]
    Io { message_key: String },
    #[error("internal")]
    Internal { correlation_id: String },
}

impl AppError {
    /// Внутренняя ошибка: причина уходит в лог под `correlation_id`, наружу — только он.
    /// `cause` должен быть без PII (ошибки крейтов проекта это гарантируют).
    #[must_use]
    pub fn internal(context: &'static str, cause: &dyn std::fmt::Display) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        let correlation_id = format!("{millis:x}-{seq:x}");
        tracing::error!(%correlation_id, context, error = %cause, "internal error");
        Self::Internal { correlation_id }
    }

    /// Внутренняя ошибка из-за аварийного завершения рабочего потока.
    ///
    /// В лог идёт только факт («паника» или «отмена»): `Display` у `JoinError` содержит
    /// текст паники, а он может нести данные бюджета.
    pub fn worker_failed(error: &tauri::Error) -> Self {
        Self::internal("session worker", &worker_failure_cause(error))
    }
}

impl From<StorageError> for AppError {
    fn from(e: StorageError) -> Self {
        match e {
            StorageError::Missing => Self::Io {
                message_key: "errors.vault.database_missing".into(),
            },
            StorageError::Invalid(key) => Self::Validation {
                message_key: format!("errors.{key}"),
                field: None,
            },
            StorageError::Conflict(key) => Self::Conflict {
                message_key: format!("errors.{key}"),
            },
            StorageError::NotFound => Self::NotFound {
                entity: "record".into(),
                id: 0,
            },
            other => Self::internal("storage", &other),
        }
    }
}

impl AppError {
    /// Ошибка хранилища с известной сущностью: `NotFound` получает имя и id записи.
    #[must_use]
    pub fn from_storage(e: StorageError, entity: &'static str, id: i64) -> Self {
        match e {
            StorageError::NotFound => Self::NotFound {
                entity: entity.into(),
                id,
            },
            other => other.into(),
        }
    }

    /// Ошибка проверки входного поля, которое пришло строкой (месяц, дата).
    #[must_use]
    pub fn invalid_field(message_key: &str, field: &str) -> Self {
        Self::Validation {
            message_key: format!("errors.{message_key}"),
            field: Some(field.to_owned()),
        }
    }
}

impl From<planning_budget_import::ImportError> for AppError {
    fn from(e: planning_budget_import::ImportError) -> Self {
        use planning_budget_import::ImportError as E;
        let (key, line) = match e {
            E::NotXlsx => ("not_xlsx", None),
            E::MissingSheet(_) => ("missing_sheet", None),
            E::Layout { row, .. } => ("layout", Some(row)),
            E::UnknownStatus { row, .. } => ("unknown_status", Some(row)),
            E::UnknownKind { row } => ("unknown_kind", Some(row)),
            E::Number { row, .. } => ("number", Some(row)),
            E::NoYear => ("no_year", None),
        };
        Self::Import {
            message_key: format!("errors.import.legacy.{key}"),
            line,
        }
    }
}

impl From<planning_budget_core::CoreError> for AppError {
    fn from(e: planning_budget_core::CoreError) -> Self {
        Self::internal("core", &e)
    }
}

/// Причина сбоя рабочего потока без текста паники.
pub(crate) fn worker_failure_cause(error: &tauri::Error) -> &'static str {
    match error {
        tauri::Error::JoinError(join) if join.is_panic() => "worker panicked",
        tauri::Error::JoinError(_) => "worker cancelled",
        _ => "worker failed",
    }
}
