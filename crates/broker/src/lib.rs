//! Заглушка брокера: интерфейс провайдера и единственная
//! реализация MVP — `NotConfigured`. Сеть появится только вместе с настоящим провайдером
//! после отдельного решения (модель угроз, хранение токена в сейфе).

use planning_budget_core::{Money, YearMonth};

/// Включительный период в месяцах.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Period {
    pub from: YearMonth,
    pub to: YearMonth,
}

/// Облигация в портфеле.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Position {
    pub name: String,
    pub quantity: u32,
    pub value: Money,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Portfolio {
    pub positions: Vec<Position>,
}

/// Выплаченный купон.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CouponPayment {
    pub month: YearMonth,
    pub name: String,
    pub amount: Money,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BrokerError {
    #[error("broker is not implemented")]
    NotImplemented,
}

/// Источник данных о брокерском счёте. Методы возвращают `Send`-футуры, чтобы провайдера можно
/// было вызывать из `spawn_blocking`/`tokio`; динамическая диспетчеризация появится вместе
/// с настоящей реализацией.
pub trait BrokerProvider: Send + Sync {
    /// Стабильный идентификатор провайдера, например `"tinvest"`.
    fn id(&self) -> &'static str;
    /// Название для интерфейса, например «Т-Инвестиции».
    fn display_name(&self) -> &'static str;
    fn portfolio(&self) -> impl Future<Output = Result<Portfolio, BrokerError>> + Send;
    fn coupons(
        &self,
        period: Period,
    ) -> impl Future<Output = Result<Vec<CouponPayment>, BrokerError>> + Send;
}

/// Брокер не подключён: все методы отвечают `NotImplemented`.
#[derive(Clone, Copy, Debug, Default)]
pub struct NotConfigured;

impl BrokerProvider for NotConfigured {
    fn id(&self) -> &'static str {
        "none"
    }

    fn display_name(&self) -> &'static str {
        "Брокерский счёт не подключён"
    }

    async fn portfolio(&self) -> Result<Portfolio, BrokerError> {
        Err(BrokerError::NotImplemented)
    }

    async fn coupons(&self, _period: Period) -> Result<Vec<CouponPayment>, BrokerError> {
        Err(BrokerError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    use super::*;

    /// Футуры заглушки готовы сразу: исполнитель не нужен.
    fn ready<T>(future: impl Future<Output = T>) -> T {
        let mut future = pin!(future);
        let mut cx = Context::from_waker(Waker::noop());
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => value,
            Poll::Pending => unreachable!("future of NotConfigured never waits"),
        }
    }

    #[test]
    fn not_configured_refuses_every_call() {
        let broker = NotConfigured;
        assert_eq!(ready(broker.portfolio()), Err(BrokerError::NotImplemented));
        let month = YearMonth::new(2026, 9).unwrap();
        let period = Period {
            from: month,
            to: month,
        };
        assert_eq!(
            ready(broker.coupons(period)),
            Err(BrokerError::NotImplemented)
        );
    }

    #[test]
    fn identifies_itself_without_network() {
        assert_eq!(NotConfigured.id(), "none");
        assert!(!NotConfigured.display_name().is_empty());
    }
}
