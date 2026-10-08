//! Автоблокировка по бездействию.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};

use tauri::{AppHandle, Manager as _};

use crate::events::{self, LockReason};
use crate::state::AppState;

const CHECK_EVERY: Duration = Duration::from_secs(5);
const DEFAULT_MINUTES: u32 = 5;

/// Отметка времени по двум часам сразу.
///
/// Монотонные часы (`Instant`) на macOS и Linux стоят, пока компьютер спит: по ним
/// закрытая на ночь крышка выглядит как минута бездействия. Настенные часы сон считают,
/// но их можно перевести. Бездействием считается большее из двух значений.
#[derive(Debug, Clone, Copy)]
pub struct Moment {
    pub monotonic: Instant,
    pub wall: SystemTime,
}

impl Moment {
    #[must_use]
    pub fn now() -> Self {
        Self {
            monotonic: Instant::now(),
            wall: SystemTime::now(),
        }
    }

    /// Сколько прошло с `earlier`. Перевод настенных часов назад даёт по ним ноль.
    fn since(self, earlier: Self) -> Duration {
        let monotonic = self.monotonic.saturating_duration_since(earlier.monotonic);
        let wall = self.wall.duration_since(earlier.wall).unwrap_or_default();
        monotonic.max(wall)
    }
}

/// Время последней активности и порог блокировки. `0` минут — «никогда».
#[derive(Debug)]
pub struct IdleTimer {
    last_activity: Mutex<Moment>,
    minutes: AtomicU32,
}

impl IdleTimer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            last_activity: Mutex::new(Moment::now()),
            minutes: AtomicU32::new(DEFAULT_MINUTES),
        }
    }

    /// Сбрасывает отсчёт: вызывается из `activity_ping` и при входе.
    pub fn touch(&self) {
        *self
            .last_activity
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Moment::now();
    }

    /// Задаёт отметку активности явно: тесты состарят её, не дожидаясь часов.
    #[cfg(test)]
    pub fn touch_at(&self, at: Moment) {
        *self
            .last_activity
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = at;
    }

    pub fn set_minutes(&self, minutes: u32) {
        self.minutes.store(minutes, Ordering::Relaxed);
    }

    #[must_use]
    pub fn expired(&self, now: Moment) -> bool {
        let minutes = self.minutes.load(Ordering::Relaxed);
        if minutes == 0 {
            return false;
        }
        let last = *self
            .last_activity
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        now.since(last) >= Duration::from_secs(u64::from(minutes) * 60)
    }
}

impl Default for IdleTimer {
    fn default() -> Self {
        Self::new()
    }
}

/// Фоновый поток: раз в несколько секунд блокирует просроченную сессию.
///
/// # Errors
/// Если поток не удалось создать.
pub fn spawn_watcher(app: AppHandle) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("idle-watcher".into())
        .spawn(move || {
            loop {
                std::thread::sleep(CHECK_EVERY);
                let state = app.state::<AppState>();
                if state.is_unlocked() && state.idle().expired(Moment::now()) {
                    events::lock_session(&app, LockReason::Idle);
                }
            }
        })
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn after(base: Moment, monotonic_secs: u64, wall_secs: u64) -> Moment {
        Moment {
            monotonic: base.monotonic + Duration::from_secs(monotonic_secs),
            wall: base.wall + Duration::from_secs(wall_secs),
        }
    }

    #[test]
    fn expires_only_after_the_configured_minutes() {
        let timer = IdleTimer::new();
        timer.set_minutes(1);
        let start = Moment::now();
        assert!(!timer.expired(after(start, 59, 59)));
        assert!(timer.expired(after(start, 61, 61)));
    }

    #[test]
    fn touch_restarts_the_countdown() {
        let timer = IdleTimer::new();
        timer.set_minutes(1);
        assert!(timer.expired(after(Moment::now(), 61, 61)));
        timer.touch();
        assert!(!timer.expired(after(Moment::now(), 30, 30)));
    }

    #[test]
    fn zero_minutes_means_never() {
        let timer = IdleTimer::new();
        timer.set_minutes(0);
        assert!(!timer.expired(after(Moment::now(), 86_400, 86_400)));
    }

    #[test]
    fn sleep_counts_as_idle_even_when_the_monotonic_clock_stood_still() {
        let timer = IdleTimer::new();
        timer.set_minutes(5);
        // Крышку закрыли на 8 часов: монотонные часы прошли минуту, настенные — всё время сна.
        assert!(timer.expired(after(Moment::now(), 60, 8 * 3600)));
    }

    #[test]
    fn wall_clock_moved_back_does_not_hide_real_idle_time() {
        let timer = IdleTimer::new();
        timer.set_minutes(5);
        let start = Moment::now();
        let rewound = Moment {
            monotonic: start.monotonic + Duration::from_secs(600),
            wall: start.wall - Duration::from_secs(3600),
        };
        assert!(timer.expired(rewound));
    }
}
