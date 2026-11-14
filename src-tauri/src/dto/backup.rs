//! Хранилища и резервные копии.

use serde::Serialize;
use specta::Type;

/// Хранилище из списка в «Настройках» и на экране входа.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VaultItemDto {
    pub id: String,
    pub label: String,
    /// С этим хранилищем приложение работает сейчас.
    pub active: bool,
    /// Файл ключей есть. У только что добавленного хранилища его нет: пароль задают при первом входе.
    pub exists: bool,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BackupItemDto {
    pub file_name: String,
    /// Время создания по UTC, RFC 3339.
    pub created_at: String,
    #[specta(type = specta_typescript::Number)]
    pub size_bytes: i64,
}

/// Каталог и копии активного хранилища, новые первыми.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BackupStatusDto {
    pub dir: String,
    /// Каталог не выбран пользователем: используется `backups/` в каталоге данных.
    pub dir_is_default: bool,
    pub items: Vec<BackupItemDto>,
}
