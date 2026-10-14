//! Проверка обновлений.

use serde::Serialize;
use specta::Type;

/// Итог проверки обновлений.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum UpdateState {
    /// Автопроверка выключена в «Настройках».
    Disabled,
    /// Автопроверка уже была меньше суток назад.
    Skipped,
    /// Установлена последняя версия.
    UpToDate,
    /// Есть новая версия.
    Available,
    /// Не удалось проверить (нет сети, манифест недоступен или подпись не сошлась).
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatusDto {
    pub state: UpdateState,
    pub current_version: String,
    /// Версия обновления при `state = available`.
    pub version: Option<String>,
    /// Заметки релиза из манифеста: обычный текст, не больше 2 000 символов.
    pub notes: Option<String>,
}
