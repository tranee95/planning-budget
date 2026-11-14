//! Несколько хранилищ: список, добавление, переключение.
//!
//! Основное хранилище лежит в каталоге данных, как и до появления выбора; остальные — в
//! `vaults/<id>/`. Реестр — `workspace.json`. Файлов с данными сервис не открывает.

use std::fs;

use planning_budget_vault::new_vault_id;

use crate::AppError;
use crate::dto::VaultItemDto;
use crate::state::AppState;
use crate::workspace::{self, MAIN_ID, VaultEntry, Workspace, clean_label};

/// Название основного хранилища в списке.
const MAIN_LABEL: &str = "Основное";

fn exists(state: &AppState, id: &str) -> bool {
    Workspace::vault_dir(state.paths().data_dir(), id)
        .join("vault.json")
        .is_file()
}

fn item(state: &AppState, ws: &Workspace, id: &str, label: &str) -> VaultItemDto {
    VaultItemDto {
        id: id.to_owned(),
        label: label.to_owned(),
        active: ws.active == id,
        exists: exists(state, id),
    }
}

#[must_use]
pub fn list(state: &AppState) -> Vec<VaultItemDto> {
    let ws = workspace::load(&state.paths().workspace());
    let mut items = vec![item(state, &ws, MAIN_ID, MAIN_LABEL)];
    items.extend(ws.vaults.iter().map(|v| item(state, &ws, &v.id, &v.label)));
    items
}

/// Создаёт пустую папку нового хранилища и делает его активным. Ключи и база появятся, когда
/// пользователь задаст пароль на экране входа (обычное создание хранилища).
///
/// # Errors
/// `Validation` для названия, `Io` при ошибке записи.
pub fn add(state: &AppState, label: &str) -> Result<(), AppError> {
    let label = clean_label(label)?;
    let id = new_vault_id().map_err(|e| AppError::internal("vault id", &e))?;
    let dir = Workspace::vault_dir(state.paths().data_dir(), &id);
    fs::create_dir_all(&dir).map_err(|_| AppError::Io {
        message_key: "errors.io.workspace".into(),
    })?;
    let registered = workspace::update(&state.paths().workspace(), |ws| {
        ws.vaults.push(VaultEntry {
            id: id.clone(),
            label,
        });
        ws.active.clone_from(&id);
        Ok(())
    });
    if let Err(err) = registered {
        let _ = fs::remove_dir(&dir);
        return Err(err);
    }
    state.activate(dir);
    Ok(())
}

/// Делает хранилище активным и закрывает текущую сессию.
///
/// # Errors
/// `NotFound`, если хранилища нет в списке; `Io` при записи реестра.
pub fn switch(state: &AppState, id: &str) -> Result<(), AppError> {
    workspace::update(&state.paths().workspace(), |ws| {
        if !workspace::valid_id(id) || !ws.knows(id) {
            return Err(AppError::NotFound {
                entity: "vault".into(),
                id: 0,
            });
        }
        id.clone_into(&mut ws.active);
        Ok(())
    })?;
    state.activate(Workspace::vault_dir(state.paths().data_dir(), id));
    Ok(())
}

/// Переименовывает дополнительное хранилище (основное называется всегда одинаково).
///
/// # Errors
/// `Validation` для названия, `NotFound` для неизвестного или основного хранилища.
pub fn rename(state: &AppState, id: &str, label: &str) -> Result<(), AppError> {
    let label = clean_label(label)?;
    workspace::update(&state.paths().workspace(), |ws| {
        let entry = ws
            .vaults
            .iter_mut()
            .find(|v| v.id == id)
            .ok_or(AppError::NotFound {
                entity: "vault".into(),
                id: 0,
            })?;
        entry.label = label;
        Ok(())
    })
}

/// Убирает из списка хранилище, которое так и не создали (нет ключей). Хранилища с данными
/// отсюда не удаляются: это необратимо, для них есть «Удалить все данные».
///
/// # Errors
/// `Conflict` для активного, основного хранилища и хранилища с ключами.
pub fn remove_unused(state: &AppState, id: &str) -> Result<(), AppError> {
    let conflict = || AppError::Conflict {
        message_key: "errors.vault.remove_denied".into(),
    };
    let ws = workspace::load(&state.paths().workspace());
    if id == MAIN_ID || ws.active == id || exists(state, id) || !ws.knows(id) {
        return Err(conflict());
    }
    workspace::update(&state.paths().workspace(), |ws| {
        ws.vaults.retain(|v| v.id != id);
        Ok(())
    })?;
    // Папка пуста (ключей нет); если там что-то оказалось, она остаётся на месте.
    let _ = fs::remove_dir(Workspace::vault_dir(state.paths().data_dir(), id));
    Ok(())
}

/// Включает в `AppState` активное хранилище из реестра при запуске приложения.
pub fn activate_saved(state: &AppState) {
    let ws = workspace::load(&state.paths().workspace());
    if ws.active != MAIN_ID {
        state.activate(Workspace::vault_dir(state.paths().data_dir(), &ws.active));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppPaths;

    fn state(dir: &tempfile::TempDir) -> AppState {
        AppState::new(AppPaths::new(dir.path().to_path_buf()))
    }

    #[test]
    fn a_fresh_install_has_only_the_main_vault() {
        let dir = tempfile::tempdir().unwrap();
        let items = list(&state(&dir));
        assert_eq!(items.len(), 1);
        assert!(items[0].active && !items[0].exists && items[0].id == MAIN_ID);
    }

    #[test]
    fn add_switches_paths_and_survives_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let st = state(&dir);
        add(&st, " Семья ").unwrap();
        let items = list(&st);
        let added = items.iter().find(|i| i.active).unwrap();
        assert_eq!(added.label, "Семья");
        assert_ne!(added.id, MAIN_ID);
        let db = st.paths().db();
        assert!(db.starts_with(dir.path().join("vaults")), "{db:?}");

        let again = state(&dir);
        assert_eq!(again.paths().db(), dir.path().join("budget.db"));
        activate_saved(&again);
        assert_eq!(again.paths().db(), db);

        switch(&st, MAIN_ID).unwrap();
        assert_eq!(st.paths().db(), dir.path().join("budget.db"));
        assert!(matches!(
            switch(&st, "../x"),
            Err(AppError::NotFound { .. })
        ));
    }

    #[test]
    fn switching_closes_the_open_session() {
        let dir = tempfile::tempdir().unwrap();
        let st = state(&dir);
        let db = planning_budget_storage::Db::create(&st.paths().db(), &[1; 32]).unwrap();
        st.open_session(db);
        add(&st, "Копия").unwrap();
        assert!(!st.is_unlocked());
    }

    #[test]
    fn only_unused_extra_vaults_can_be_removed() {
        let dir = tempfile::tempdir().unwrap();
        let st = state(&dir);
        add(&st, "Лишнее").unwrap();
        let id = st
            .paths()
            .active_dir()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert!(remove_unused(&st, &id).is_err(), "active");
        switch(&st, MAIN_ID).unwrap();
        assert!(remove_unused(&st, MAIN_ID).is_err());
        remove_unused(&st, &id).unwrap();
        assert_eq!(list(&st).len(), 1);
        assert!(!dir.path().join("vaults").join(&id).exists());
    }

    #[test]
    fn switching_leaves_no_state_of_the_previous_vault_on_the_login_screen() {
        let dir = tempfile::tempdir().unwrap();
        let st = state(&dir);
        st.set_migration_failed(true);
        crate::prefs::set_autolock_minutes(&st.paths().prefs(), Some(15)).unwrap();
        let now = chrono::Utc::now();

        add(&st, "Другое").unwrap();
        assert!(
            !crate::services::vault::status(&st, now)
                .unwrap()
                .migration_failed
        );
        assert_eq!(
            crate::prefs::load(&st.paths().prefs()).autolock_minutes,
            None
        );

        // Перенос данных касается основного хранилища: к нему предупреждение возвращается.
        switch(&st, MAIN_ID).unwrap();
        assert!(
            crate::services::vault::status(&st, now)
                .unwrap()
                .migration_failed
        );
    }

    #[test]
    fn rename_changes_only_extra_vaults() {
        let dir = tempfile::tempdir().unwrap();
        let st = state(&dir);
        add(&st, "Старое").unwrap();
        let id = list(&st).into_iter().find(|i| i.active).unwrap().id;
        rename(&st, &id, "Новое").unwrap();
        assert!(list(&st).iter().any(|i| i.label == "Новое"));
        assert!(rename(&st, MAIN_ID, "Эх").is_err());
        assert!(rename(&st, &id, "  ").is_err());
    }
}
