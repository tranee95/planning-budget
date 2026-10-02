//! DTO на границе IPC. Типы для фронтенда генерирует tauri-specta.

use std::fmt;

use budget_core::calc::{
    BudgetBalance, CategoryYearRow, Corridor, KindAmounts, LimitLevel, LimitRow, MonthSummary,
    StatusAmounts,
};
use budget_core::query::{QueryErrorKind, TokenKind};
use budget_core::{Category, CategoryKind, IncomeStatus, Money, Tag, TxStatus};
use budget_storage::{IncomeRecord, TransactionRecord};
use secrecy::SecretString;
use serde::{Deserialize, Deserializer, Serialize};
use specta::Type;

/// Пароль или recovery-код с фронтенда. В `Debug` и логи не попадает.
#[derive(Type)]
#[specta(transparent)]
pub struct Secret(#[specta(type = String)] SecretString);

impl Secret {
    #[must_use]
    pub fn secret(&self) -> &SecretString {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d).map(|s| Self(SecretString::from(s)))
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatusDto {
    pub exists: bool,
    pub locked: bool,
    /// Сколько мс ещё ждать до следующей попытки входа; `0` — можно вводить.
    pub retry_after_ms: u32,
}

/// Recovery-код показывается один раз сразу после создания.
#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryCodeDto {
    pub recovery_code: String,
}

impl fmt::Debug for RecoveryCodeDto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RecoveryCodeDto(<redacted>)")
    }
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SavedDto {
    pub saved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ReducedMotion {
    System,
    On,
}

/// Содержимое `ui-prefs.json`: только оболочка, никаких данных бюджета.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Prefs {
    pub theme: Theme,
    pub locale: String,
    /// Масштаб интерфейса в процентах, 90–125.
    pub ui_scale: u8,
    pub reduced_motion: ReducedMotion,
    /// Копия `security.autolock_minutes` для экрана входа: настройка лежит в зашифрованной БД и
    /// до входа недоступна. Пишет бэкенд при входе и при смене настройки; не секрет. `0` — никогда,
    /// `None` — ещё не известна (файл создан старой версией): экран входа тогда ничего не пишет.
    pub autolock_minutes: Option<u16>,
    /// Подсказки для новичков (онбординг): показываются, пока пользователь не пройдёт их или не отключит.
    pub show_tips: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            locale: "ru-RU".to_owned(),
            ui_scale: 100,
            reduced_motion: ReducedMotion::System,
            autolock_minutes: None,
            show_tips: true,
        }
    }
}

#[derive(Debug, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct PrefsPatch {
    pub theme: Option<Theme>,
    pub locale: Option<String>,
    pub ui_scale: Option<u8>,
    pub reduced_motion: Option<ReducedMotion>,
    pub show_tips: Option<bool>,
}

/// Целое из JS `number` (id, копейки < 2^53) в аргументах команд: tauri-specta не принимает
/// `i64` как есть, а в TS нужен `number`, а не `bigint`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(transparent)]
#[specta(transparent)]
pub struct Int53(#[specta(type = specta_typescript::Number)] pub i64);

// --- Данные бюджета ---------------------------------------------------------------
// Деньги — копейки в `number` (< 2^53), месяц — строка `YYYY-MM`, дата — `YYYY-MM-DD`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum CategoryKindDto {
    Mandatory,
    Wants,
    Savings,
    Loans,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum TxStatusDto {
    Paid,
    Debt,
    Unplanned,
    Planned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum IncomeStatusDto {
    Received,
    Expected,
}

impl From<CategoryKind> for CategoryKindDto {
    fn from(kind: CategoryKind) -> Self {
        match kind {
            CategoryKind::Mandatory => Self::Mandatory,
            CategoryKind::Wants => Self::Wants,
            CategoryKind::Savings => Self::Savings,
            CategoryKind::Loans => Self::Loans,
        }
    }
}

impl From<CategoryKindDto> for CategoryKind {
    fn from(kind: CategoryKindDto) -> Self {
        match kind {
            CategoryKindDto::Mandatory => Self::Mandatory,
            CategoryKindDto::Wants => Self::Wants,
            CategoryKindDto::Savings => Self::Savings,
            CategoryKindDto::Loans => Self::Loans,
        }
    }
}

impl From<TxStatus> for TxStatusDto {
    fn from(status: TxStatus) -> Self {
        match status {
            TxStatus::Paid => Self::Paid,
            TxStatus::Debt => Self::Debt,
            TxStatus::Unplanned => Self::Unplanned,
            TxStatus::Planned => Self::Planned,
        }
    }
}

impl From<TxStatusDto> for TxStatus {
    fn from(status: TxStatusDto) -> Self {
        match status {
            TxStatusDto::Paid => Self::Paid,
            TxStatusDto::Debt => Self::Debt,
            TxStatusDto::Unplanned => Self::Unplanned,
            TxStatusDto::Planned => Self::Planned,
        }
    }
}

impl From<IncomeStatus> for IncomeStatusDto {
    fn from(status: IncomeStatus) -> Self {
        match status {
            IncomeStatus::Received => Self::Received,
            IncomeStatus::Expected => Self::Expected,
        }
    }
}

impl From<IncomeStatusDto> for IncomeStatus {
    fn from(status: IncomeStatusDto) -> Self {
        match status {
            IncomeStatusDto::Received => Self::Received,
            IncomeStatusDto::Expected => Self::Expected,
        }
    }
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub name: String,
    pub kind: CategoryKindDto,
    pub color: String,
    pub sort_order: i32,
    pub note: Option<String>,
    pub archived: bool,
}

impl From<&Category> for CategoryDto {
    fn from(c: &Category) -> Self {
        Self {
            id: c.id.0,
            name: c.name.clone(),
            kind: c.kind.into(),
            color: c.color.clone(),
            sort_order: c.sort_order,
            note: c.note.clone(),
            archived: c.archived,
        }
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CategoryInput {
    pub name: String,
    pub kind: CategoryKindDto,
    pub color: String,
    pub note: Option<String>,
}

/// Правка категории: отсутствующее поле не меняется; `note: null` очищает заметку.
#[derive(Debug, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct CategoryPatchDto {
    pub name: Option<String>,
    pub color: Option<String>,
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>)]
    pub note: Option<Option<String>>,
}

/// Различает «поле не передано» и «передан null».
fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LimitEntryDto {
    pub valid_from: String,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
}

/// Строка истории процента плана сбережений.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SavingsRateDto {
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    pub valid_from: String,
    pub rate_bp: i32,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TagDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub name: String,
}

impl From<&Tag> for TagDto {
    fn from(t: &Tag) -> Self {
        Self {
            id: t.id.0,
            name: t.name.clone(),
        }
    }
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub month: String,
    pub date: Option<String>,
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    pub title: String,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
    pub status: TxStatusDto,
    pub comment: Option<String>,
    #[specta(type = Vec<specta_typescript::Number>)]
    pub tag_ids: Vec<i64>,
}

impl From<&TransactionRecord> for TransactionDto {
    fn from(t: &TransactionRecord) -> Self {
        Self {
            id: t.id.0,
            month: t.month.to_string(),
            date: t.date.map(|d| d.to_string()),
            category_id: t.category_id.0,
            title: t.title.clone(),
            amount: t.amount.kopecks(),
            status: t.status.into(),
            comment: t.comment.clone(),
            tag_ids: t.tags.iter().map(|id| id.0).collect(),
        }
    }
}

/// Новая трата. Если задана `date`, её месяц должен совпадать с `month`.
#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TransactionInput {
    pub month: String,
    pub date: Option<String>,
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    pub title: String,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
    pub status: TxStatusDto,
    pub comment: Option<String>,
}

/// Правка траты: отсутствующее поле не меняется; `date`/`comment: null` очищают значение.
#[derive(Debug, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct TransactionPatchDto {
    pub month: Option<String>,
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>)]
    pub date: Option<Option<String>>,
    #[specta(type = Option<specta_typescript::Number>)]
    pub category_id: Option<i64>,
    pub title: Option<String>,
    #[specta(type = Option<specta_typescript::Number>)]
    pub amount: Option<i64>,
    pub status: Option<TxStatusDto>,
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>)]
    pub comment: Option<Option<String>>,
}

/// Состояние до и после правки: по `previous` UI делает отмену.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TxUpdateResultDto {
    pub current: TransactionDto,
    pub previous: TransactionDto,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IncomeDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub month: String,
    pub date: Option<String>,
    pub source_name: String,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
    pub status: IncomeStatusDto,
    pub comment: Option<String>,
}

impl From<&IncomeRecord> for IncomeDto {
    fn from(i: &IncomeRecord) -> Self {
        Self {
            id: i.id.0,
            month: i.month.to_string(),
            date: i.date.map(|d| d.to_string()),
            source_name: i.source_name.clone(),
            amount: i.amount.kopecks(),
            status: i.status.into(),
            comment: i.comment.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IncomeInput {
    pub month: String,
    pub date: Option<String>,
    pub source_name: String,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
    pub status: IncomeStatusDto,
    pub comment: Option<String>,
}

#[derive(Debug, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct IncomePatchDto {
    pub month: Option<String>,
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>)]
    pub date: Option<Option<String>>,
    pub source_name: Option<String>,
    #[specta(type = Option<specta_typescript::Number>)]
    pub amount: Option<i64>,
    pub status: Option<IncomeStatusDto>,
    #[serde(deserialize_with = "double_option")]
    #[specta(type = Option<String>)]
    pub comment: Option<Option<String>>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IncomeUpdateResultDto {
    pub current: IncomeDto,
    pub previous: IncomeDto,
}

/// Настройки, которые редактирует пользователь.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub savings_min_bp: i32,
    pub savings_norm_bp: i32,
    pub savings_max_bp: i32,
    pub bonds_rate_bp: i32,
    pub bonds_coupon_tax_bp: i32,
    #[specta(type = specta_typescript::Number)]
    pub bonds_initial_balance: i64,
    pub bonds_initial_month: String,
    pub weeks_per_month: u8,
    pub autolock_minutes: u16,
    pub lock_on_minimize: bool,
    pub backup_auto_daily: bool,
}

#[derive(Debug, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatchDto {
    pub savings_min_bp: Option<i32>,
    pub savings_norm_bp: Option<i32>,
    pub savings_max_bp: Option<i32>,
    pub bonds_rate_bp: Option<i32>,
    pub bonds_coupon_tax_bp: Option<i32>,
    #[specta(type = Option<specta_typescript::Number>)]
    pub bonds_initial_balance: Option<i64>,
    pub bonds_initial_month: Option<String>,
    pub weeks_per_month: Option<u8>,
    pub autolock_minutes: Option<u16>,
    pub lock_on_minimize: Option<bool>,
    pub backup_auto_daily: Option<bool>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StatusAmountsDto {
    #[specta(type = specta_typescript::Number)]
    pub paid: i64,
    #[specta(type = specta_typescript::Number)]
    pub debt: i64,
    #[specta(type = specta_typescript::Number)]
    pub unplanned: i64,
    #[specta(type = specta_typescript::Number)]
    pub planned: i64,
}

impl From<StatusAmounts> for StatusAmountsDto {
    fn from(a: StatusAmounts) -> Self {
        Self {
            paid: a.paid.kopecks(),
            debt: a.debt.kopecks(),
            unplanned: a.unplanned.kopecks(),
            planned: a.planned.kopecks(),
        }
    }
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct KindAmountsDto {
    #[specta(type = specta_typescript::Number)]
    pub mandatory: i64,
    #[specta(type = specta_typescript::Number)]
    pub wants: i64,
    #[specta(type = specta_typescript::Number)]
    pub savings: i64,
    #[specta(type = specta_typescript::Number)]
    pub loans: i64,
}

impl From<KindAmounts> for KindAmountsDto {
    fn from(a: KindAmounts) -> Self {
        Self {
            mandatory: a.mandatory.kopecks(),
            wants: a.wants.kopecks(),
            savings: a.savings.kopecks(),
            loans: a.loans.kopecks(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CorridorDto {
    NoIncome,
    Below,
    Within,
    Above,
}

impl From<Corridor> for CorridorDto {
    fn from(c: Corridor) -> Self {
        match c {
            Corridor::NoIncome => Self::NoIncome,
            Corridor::Below => Self::Below,
            Corridor::Within => Self::Within,
            Corridor::Above => Self::Above,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum LimitLevelDto {
    Ok,
    Warn,
    Over,
}

impl From<LimitLevel> for LimitLevelDto {
    fn from(l: LimitLevel) -> Self {
        match l {
            LimitLevel::Ok => Self::Ok,
            LimitLevel::Warn => Self::Warn,
            LimitLevel::Over => Self::Over,
        }
    }
}

/// Строка листа «Сводка» за месяц.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MonthSummaryDto {
    pub month: String,
    #[specta(type = specta_typescript::Number)]
    pub income: i64,
    #[specta(type = specta_typescript::Number)]
    pub income_received: i64,
    #[specta(type = specta_typescript::Number)]
    pub income_expected: i64,
    #[specta(type = specta_typescript::Number)]
    pub expenses: i64,
    #[specta(type = specta_typescript::Number)]
    pub savings: i64,
    #[specta(type = specta_typescript::Number)]
    pub free: i64,
    #[specta(type = specta_typescript::Number)]
    pub free_cum: i64,
    #[specta(type = specta_typescript::Number)]
    pub savings_cum: i64,
    pub savings_rate: Option<f64>,
    pub unspent_rate: Option<f64>,
    pub savings_plan_rate_bp: i32,
    #[specta(type = specta_typescript::Number)]
    pub savings_plan: i64,
    #[specta(type = specta_typescript::Number)]
    pub savings_gap: i64,
    #[specta(type = specta_typescript::Number)]
    pub per_week: i64,
    pub corridor: CorridorDto,
    #[specta(type = specta_typescript::Number)]
    pub top_up_to_min: i64,
    #[specta(type = specta_typescript::Number)]
    pub top_up_to_norm: i64,
    pub by_status: StatusAmountsDto,
    pub by_kind: KindAmountsDto,
}

impl From<&MonthSummary> for MonthSummaryDto {
    fn from(s: &MonthSummary) -> Self {
        Self {
            month: s.month.to_string(),
            income: s.income.kopecks(),
            income_received: s.income_received.kopecks(),
            income_expected: s.income_expected.kopecks(),
            expenses: s.expenses.kopecks(),
            savings: s.savings.kopecks(),
            free: s.free.kopecks(),
            free_cum: s.free_cum.kopecks(),
            savings_cum: s.savings_cum.kopecks(),
            savings_rate: s.savings_rate,
            unspent_rate: s.unspent_rate,
            savings_plan_rate_bp: s.savings_plan_rate.0,
            savings_plan: s.savings_plan.kopecks(),
            savings_gap: s.savings_gap.kopecks(),
            per_week: s.per_week.kopecks(),
            corridor: s.corridor.into(),
            top_up_to_min: s.top_up_to_min.kopecks(),
            top_up_to_norm: s.top_up_to_norm.kopecks(),
            by_status: s.by_status.into(),
            by_kind: s.by_kind.into(),
        }
    }
}

/// Категория в месяце: факт против лимита.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LimitRowDto {
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    #[specta(type = specta_typescript::Number)]
    pub fact: i64,
    #[specta(type = Option<specta_typescript::Number>)]
    pub limit: Option<i64>,
    #[specta(type = Option<specta_typescript::Number>)]
    pub remaining: Option<i64>,
    pub usage: Option<f64>,
    pub level: Option<LimitLevelDto>,
    pub by_status: StatusAmountsDto,
}

impl From<&LimitRow> for LimitRowDto {
    fn from(r: &LimitRow) -> Self {
        Self {
            category_id: r.category_id.0,
            fact: r.fact.kopecks(),
            limit: r.limit.map(Money::kopecks),
            remaining: r.remaining.map(Money::kopecks),
            usage: r.usage,
            level: r.level.map(Into::into),
            by_status: r.by_status.into(),
        }
    }
}

/// Данные экрана «Обзор» за месяц.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MonthOverviewDto {
    pub summary: MonthSummaryDto,
    pub limits: Vec<LimitRowDto>,
    #[specta(type = specta_typescript::Number)]
    pub limits_total: i64,
    #[specta(type = specta_typescript::Number)]
    pub limits_remaining: i64,
    pub spent_vs_limits: Option<f64>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CategoryYearRowDto {
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    #[specta(type = specta_typescript::Number)]
    pub total: i64,
    #[specta(type = specta_typescript::Number)]
    pub avg: i64,
    #[specta(type = Option<specta_typescript::Number>)]
    pub limit_now: Option<i64>,
    #[specta(type = Option<specta_typescript::Number>)]
    pub avg_minus_limit: Option<i64>,
    pub share_in_expenses: Option<f64>,
}

impl From<&CategoryYearRow> for CategoryYearRowDto {
    fn from(r: &CategoryYearRow) -> Self {
        Self {
            category_id: r.category_id.0,
            total: r.total.kopecks(),
            avg: r.avg.kopecks(),
            limit_now: r.limit_now.map(Money::kopecks),
            avg_minus_limit: r.avg_minus_limit.map(Money::kopecks),
            share_in_expenses: r.share_in_expenses,
        }
    }
}

/// «Баланс бюджета».
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BudgetBalanceDto {
    #[specta(type = specta_typescript::Number)]
    pub avg_income: i64,
    #[specta(type = specta_typescript::Number)]
    pub savings_target: i64,
    #[specta(type = specta_typescript::Number)]
    pub limits_sum: i64,
    #[specta(type = specta_typescript::Number)]
    pub buffer: i64,
    pub buffer_rate: Option<f64>,
    #[specta(type = specta_typescript::Number)]
    pub economy: i64,
}

impl From<&BudgetBalance> for BudgetBalanceDto {
    fn from(b: &BudgetBalance) -> Self {
        Self {
            avg_income: b.avg_income.kopecks(),
            savings_target: b.savings_target.kopecks(),
            limits_sum: b.limits_sum.kopecks(),
            buffer: b.buffer.kopecks(),
            buffer_rate: b.buffer_rate,
            economy: b.economy.kopecks(),
        }
    }
}

/// Все строки листа «Сводка» за год: месяцы, итоги, категории и баланс бюджета.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct YearSummaryDto {
    pub year: u16,
    pub months: Vec<MonthSummaryDto>,
    #[specta(type = specta_typescript::Number)]
    pub income: i64,
    #[specta(type = specta_typescript::Number)]
    pub expenses: i64,
    #[specta(type = specta_typescript::Number)]
    pub savings: i64,
    #[specta(type = specta_typescript::Number)]
    pub free: i64,
    pub months_with_data: u32,
    #[specta(type = specta_typescript::Number)]
    pub avg_income: i64,
    #[specta(type = specta_typescript::Number)]
    pub avg_expenses: i64,
    #[specta(type = specta_typescript::Number)]
    pub avg_savings: i64,
    #[specta(type = specta_typescript::Number)]
    pub avg_free: i64,
    pub savings_rate: Option<f64>,
    pub by_status: StatusAmountsDto,
    pub by_kind: KindAmountsDto,
    pub categories: Vec<CategoryYearRowDto>,
    pub balance: BudgetBalanceDto,
}

/// Блок категории, у которого «Итого» в таблице не равно сумме позиций.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BlockMismatchDto {
    pub month: String,
    pub category: String,
    #[specta(type = specta_typescript::Number)]
    pub declared: i64,
    #[specta(type = specta_typescript::Number)]
    pub parsed: i64,
}

/// Итоги года по данным базы после переноса: их пользователь сверяет со своей таблицей.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LegacyTotalsDto {
    pub year: u16,
    #[specta(type = specta_typescript::Number)]
    pub income: i64,
    #[specta(type = specta_typescript::Number)]
    pub expenses: i64,
    #[specta(type = specta_typescript::Number)]
    pub savings: i64,
    #[specta(type = specta_typescript::Number)]
    pub free: i64,
    pub months_with_data: u32,
}

/// Отчёт переноса из старой таблицы.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LegacyReportDto {
    pub categories_created: u32,
    pub categories_matched: u32,
    pub kind_conflicts: Vec<String>,
    pub limits_written: u32,
    pub transactions: u32,
    pub incomes: u32,
    pub settings_applied: bool,
    pub block_mismatches: Vec<BlockMismatchDto>,
    /// `null`, если итоги посчитать не удалось: данные при этом записаны.
    pub totals: Option<LegacyTotalsDto>,
}
// --- Поиск ---------------------------------------------------------------

/// Чем распознан токен запроса; UI красит токены по этому полю.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum TokenKindDto {
    Text,
    Amount,
    Status,
    Category,
    Kind,
    Month,
    Tag,
    Source,
    Record,
}

impl From<TokenKind> for TokenKindDto {
    fn from(kind: TokenKind) -> Self {
        match kind {
            TokenKind::Text => Self::Text,
            TokenKind::Amount => Self::Amount,
            TokenKind::Status => Self::Status,
            TokenKind::Category => Self::Category,
            TokenKind::Kind => Self::Kind,
            TokenKind::Month => Self::Month,
            TokenKind::Tag => Self::Tag,
            TokenKind::Source => Self::Source,
            TokenKind::Record => Self::Record,
        }
    }
}

/// Причина мягкой подсказки: токен с известным ключом не распознан и ушёл в текст.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum QueryHintDto {
    EmptyValue,
    UnknownValue,
    BadAmount,
    BadMonth,
    NegationUnsupported,
}

impl From<QueryErrorKind> for QueryHintDto {
    fn from(kind: QueryErrorKind) -> Self {
        match kind {
            QueryErrorKind::EmptyValue => Self::EmptyValue,
            QueryErrorKind::UnknownValue => Self::UnknownValue,
            QueryErrorKind::BadAmount => Self::BadAmount,
            QueryErrorKind::BadMonth => Self::BadMonth,
            QueryErrorKind::NegationUnsupported => Self::NegationUnsupported,
        }
    }
}

/// Токен запроса: `start..end` — позиции в символах (не в UTF-16 и не в байтах).
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TokenSpanDto {
    pub start: u32,
    pub end: u32,
    pub kind: TokenKindDto,
    pub negated: bool,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QueryHintSpanDto {
    pub start: u32,
    pub end: u32,
    pub hint: QueryHintDto,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TransactionHitDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub title: String,
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    pub category: String,
    pub month: String,
    pub date: Option<String>,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
    pub status: TxStatusDto,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IncomeHitDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub source_name: String,
    pub month: String,
    pub date: Option<String>,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
    pub status: IncomeStatusDto,
}

/// Группа трат: `total` и `sum` — по всем совпадениям, `items` — первые строки.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TransactionGroupDto {
    #[specta(type = specta_typescript::Number)]
    pub total: u64,
    #[specta(type = specta_typescript::Number)]
    pub sum: i64,
    pub items: Vec<TransactionHitDto>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IncomeGroupDto {
    #[specta(type = specta_typescript::Number)]
    pub total: u64,
    #[specta(type = specta_typescript::Number)]
    pub sum: i64,
    pub items: Vec<IncomeHitDto>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CategoryHitDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MonthHitDto {
    pub month: String,
    #[specta(type = specta_typescript::Number)]
    pub records: u64,
}

/// Ответ палитры. `request_id` возвращается как есть: UI отбрасывает устаревшие ответы.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultDto {
    pub request_id: u32,
    pub spans: Vec<TokenSpanDto>,
    pub hints: Vec<QueryHintSpanDto>,
    pub transactions: TransactionGroupDto,
    pub incomes: IncomeGroupDto,
    pub categories: Vec<CategoryHitDto>,
    pub months: Vec<MonthHitDto>,
}

/// Полный список трат по запросу для таблицы «Расходы»; `total` и `sum` — по всем
/// совпадениям, `truncated` — строк больше, чем отдано.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TransactionSearchDto {
    pub request_id: u32,
    pub spans: Vec<TokenSpanDto>,
    pub hints: Vec<QueryHintSpanDto>,
    #[specta(type = specta_typescript::Number)]
    pub total: u64,
    #[specta(type = specta_typescript::Number)]
    pub sum: i64,
    pub truncated: bool,
    pub items: Vec<TransactionDto>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IncomeSearchDto {
    pub request_id: u32,
    pub spans: Vec<TokenSpanDto>,
    pub hints: Vec<QueryHintSpanDto>,
    #[specta(type = specta_typescript::Number)]
    pub total: u64,
    #[specta(type = specta_typescript::Number)]
    pub sum: i64,
    pub truncated: bool,
    pub items: Vec<IncomeDto>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SavedFilterDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub name: String,
    pub query: String,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TitleSuggestionDto {
    pub title: String,
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    #[specta(type = specta_typescript::Number)]
    pub uses: u64,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CategoryUsageDto {
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    #[specta(type = specta_typescript::Number)]
    pub uses: u64,
}

// --- Аналитика ---------------------------------------------------------------------
// Перечисления совпадают с `budget_core::analytics` по именам значений (snake_case).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ChartTypeDto {
    Bar,
    StackedBar,
    Hbar,
    Line,
    Area,
    Donut,
    Table,
    Kpi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ChartMetricDto {
    Spent,
    Expenses,
    Income,
    Savings,
    Free,
    FreeCum,
    SavingsCum,
    SavingsRate,
    LimitUsage,
    Count,
    AvgTicket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ChartGroupByDto {
    Month,
    Category,
    Kind,
    Status,
    Tag,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ChartSeriesByDto {
    Status,
    Kind,
    Category,
    Metric,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum PeriodPresetDto {
    Ytd,
    Last12,
    CurrentMonth,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ChartSortDto {
    Desc,
    Asc,
    Natural,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ChartUnitDto {
    Rub,
    Percent,
    Count,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(untagged)]
pub enum ChartPeriodDto {
    Preset { preset: PeriodPresetDto },
    Range { from: String, to: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChartOptionsDto {
    pub show_limit: bool,
    pub top_n: Option<u32>,
    pub sort: ChartSortDto,
    pub cumulative: bool,
    pub compare_prev_period: bool,
    pub percent: bool,
}

/// `ChartSpec` v1; в базе лежит JSON `budget_core::analytics::ChartSpec`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChartSpecDto {
    pub version: u8,
    pub title: String,
    #[serde(rename = "type")]
    pub chart_type: ChartTypeDto,
    pub metric: ChartMetricDto,
    pub group_by: ChartGroupByDto,
    pub series_by: Option<ChartSeriesByDto>,
    pub metrics: Vec<ChartMetricDto>,
    pub period: ChartPeriodDto,
    pub filter: String,
    pub options: ChartOptionsDto,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChartSeriesDto {
    pub name: String,
    pub color: String,
    pub values: Vec<Option<f64>>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChartRefLineDto {
    pub name: String,
    pub from: f64,
    pub to: Option<f64>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChartDataDto {
    pub categories: Vec<String>,
    pub category_tokens: Vec<String>,
    pub series: Vec<ChartSeriesDto>,
    pub reference_lines: Vec<ChartRefLineDto>,
    pub totals: Option<Vec<f64>>,
    pub unit: ChartUnitDto,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DashboardDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub name: String,
    pub is_default: bool,
}

/// Карточка графика на сетке из 12 колонок; высота ячейки 80 px.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChartCardDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    #[specta(type = specta_typescript::Number)]
    pub dashboard_id: i64,
    pub spec: ChartSpecDto,
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CardPlacementDto {
    pub id: Int53,
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
}

// --- Облигации ---------------------------------------------------------------------

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BondsMonthDto {
    pub month: String,
    #[specta(type = specta_typescript::Number)]
    pub savings: i64,
    #[specta(type = specta_typescript::Number)]
    pub coupon: i64,
    #[specta(type = specta_typescript::Number)]
    pub balance: i64,
    #[specta(type = specta_typescript::Number)]
    pub deposited: i64,
    #[specta(type = specta_typescript::Number)]
    pub coupon_income: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ForecastKindDto {
    A,
    B,
    C,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ForecastScenarioDto {
    pub scenario: ForecastKindDto,
    #[specta(type = specta_typescript::Number)]
    pub contribution: i64,
    #[specta(type = specta_typescript::Number)]
    pub y1: i64,
    #[specta(type = specta_typescript::Number)]
    pub y3: i64,
    #[specta(type = specta_typescript::Number)]
    pub y5: i64,
    #[specta(type = specta_typescript::Number)]
    pub coupons60: i64,
    /// Баланс на конец каждого из 60 месяцев: точки линии прогноза.
    #[specta(type = Vec<specta_typescript::Number>)]
    pub balances: Vec<i64>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BondsDto {
    pub year: u16,
    /// Ставка после налога на купон и её месячный эквивалент (доли единицы).
    pub effective_rate: f64,
    pub monthly_rate: f64,
    pub months: Vec<BondsMonthDto>,
    /// Баланс на конец декабря: старт прогноза.
    #[specta(type = specta_typescript::Number)]
    pub dec_balance: i64,
    pub scenarios: Vec<ForecastScenarioDto>,
}
