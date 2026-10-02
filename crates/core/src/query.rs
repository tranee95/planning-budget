//! Язык запроса поиска и фильтров: `парсер → Filter + текст + spans`.

use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::model::{CategoryKind, TxStatus};
use crate::money::Money;
use crate::period::YearMonth;

/// Включительный диапазон месяцев.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct MonthRange {
    pub from: YearMonth,
    pub to: YearMonth,
}

/// Имя категории или его начало, как ввёл пользователь; сопоставление — в `storage`.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CategoryRef(pub String);

/// Диапазон сумм в копейках; границы включительны, `None` — без ограничения.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct AmountRange {
    pub min: Option<Money>,
    pub max: Option<Money>,
}

impl AmountRange {
    fn intersect(self, other: Self) -> Self {
        Self {
            min: self.min.max(other.min),
            max: match (self.max, other.max) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            },
        }
    }
}

/// Происхождение записи (`transactions.source`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Manual,
    Import,
    Legacy,
    Seed,
}

/// Какие записи искать.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordKind {
    #[default]
    Any,
    Expense,
    Income,
}

/// Структурная часть запроса; общая для палитры, панели фильтров и графиков.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Filter {
    pub months: Option<MonthRange>,
    pub categories: Vec<CategoryRef>,
    pub exclude_categories: Vec<CategoryRef>,
    pub kinds: Vec<CategoryKind>,
    pub statuses: Vec<TxStatus>,
    pub amount: Option<AmountRange>,
    pub tags: Vec<String>,
    pub exclude_tags: Vec<String>,
    pub source: Option<Source>,
    pub record: RecordKind,
}

/// Свободное слово или фраза в кавычках для FTS.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Term {
    pub text: String,
    pub phrase: bool,
}

/// Чем распознан токен: UI красит его чипом.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenKind {
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

/// Токен во входной строке; `range` — в символах, `-` отрицания входит в диапазон.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TokenSpan {
    pub range: Range<usize>,
    pub kind: TokenKind,
    pub negated: bool,
}

/// Причина, по которой токен с известным ключом не распознан и ушёл в текст.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryErrorKind {
    EmptyValue,
    UnknownValue,
    BadAmount,
    BadMonth,
    NegationUnsupported,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct QueryError {
    pub range: Range<usize>,
    pub kind: QueryErrorKind,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ParsedQuery {
    pub text: Vec<Term>,
    pub filter: Filter,
    pub spans: Vec<TokenSpan>,
    pub errors: Vec<QueryError>,
}

enum Applied {
    Done(TokenKind, bool),
    Text,
    Rejected(QueryErrorKind),
}

type Aliases<T> = [(&'static [&'static str], T)];

const AMOUNT_KEYS: &[&str] = &["сумма", "amount"];
const STATUS_KEYS: &[&str] = &["статус", "status"];
const CATEGORY_KEYS: &[&str] = &["кат", "категория", "cat", "category"];
const KIND_KEYS: &[&str] = &["тип", "kind"];
const MONTH_KEYS: &[&str] = &["мес", "месяц", "период", "month"];
const SOURCE_KEYS: &[&str] = &["источник", "src", "source"];

const STATUSES: &Aliases<TxStatus> = &[
    (&["оплачено", "оплачен", "paid"], TxStatus::Paid),
    (&["долг", "debt"], TxStatus::Debt),
    (
        &["внеплан", "незапланировано", "unplanned"],
        TxStatus::Unplanned,
    ),
    (&["план", "planned"], TxStatus::Planned),
];
const KINDS: &Aliases<CategoryKind> = &[
    (
        &["обязательные", "обязательное", "mandatory"],
        CategoryKind::Mandatory,
    ),
    (&["желания", "wants"], CategoryKind::Wants),
    (&["сбережения", "savings"], CategoryKind::Savings),
    (&["займы", "loans"], CategoryKind::Loans),
];
const SOURCES: &Aliases<Source> = &[
    (&["вручную", "manual"], Source::Manual),
    (&["импорт", "import"], Source::Import),
    (&["legacy", "таблица"], Source::Legacy),
    (&["seed"], Source::Seed),
];
const RECORDS: &Aliases<RecordKind> = &[
    (&["доход", "income"], RecordKind::Income),
    (&["расход", "expense"], RecordKind::Expense),
];
const MONTH_NAMES: [&str; 12] = [
    "январь",
    "февраль",
    "март",
    "апрель",
    "май",
    "июнь",
    "июль",
    "август",
    "сентябрь",
    "октябрь",
    "ноябрь",
    "декабрь",
];

/// Разбирает запрос; не паникует и не отклоняет ввод. `current_year` подставляется в
/// месяцы без года («мес:сен»).
///
/// ```
/// use budget_core::query::{parse, RecordKind};
/// let q = parse("лента статус:оплачено сумма>10000 доход", 2026);
/// assert_eq!(q.text.len(), 1);
/// assert_eq!(q.filter.record, RecordKind::Income);
/// assert_eq!(q.spans.len(), 4);
/// ```
pub fn parse(input: &str, current_year: u16) -> ParsedQuery {
    let chars: Vec<char> = input.chars().collect();
    let mut q = ParsedQuery::default();
    for range in tokenize(&chars) {
        let Some(tok) = chars.get(range.clone()) else {
            continue;
        };
        match apply(&mut q.filter, tok, current_year) {
            Applied::Done(kind, negated) => q.spans.push(TokenSpan {
                range,
                kind,
                negated,
            }),
            Applied::Text => push_text(&mut q, tok, range),
            Applied::Rejected(kind) => {
                q.errors.push(QueryError {
                    range: range.clone(),
                    kind,
                });
                push_text(&mut q, tok, range);
            }
        }
    }
    q
}

fn push_text(q: &mut ParsedQuery, tok: &[char], range: Range<usize>) {
    let phrase = tok.first() == Some(&'"');
    let text = unquote(tok).trim().to_owned();
    if !text.is_empty() {
        q.text.push(Term { text, phrase });
    }
    q.spans.push(TokenSpan {
        range,
        kind: TokenKind::Text,
        negated: false,
    });
}

/// Токены разделены пробелами вне кавычек.
fn tokenize(chars: &[char]) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = None;
    let mut in_quotes = false;
    for (i, &c) in chars.iter().enumerate() {
        if c == '"' {
            in_quotes = !in_quotes;
        }
        if c.is_whitespace() && !in_quotes {
            if let Some(s) = start.take() {
                out.push(s..i);
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(s) = start {
        out.push(s..chars.len());
    }
    out
}

fn unquote(tok: &[char]) -> String {
    let s: String = tok.iter().collect();
    match s.strip_prefix('"') {
        Some(rest) => rest.strip_suffix('"').unwrap_or(rest).to_owned(),
        None => s,
    }
}

/// Нижний регистр и `ё = е`.
fn norm(s: &str) -> String {
    s.to_lowercase().replace('ё', "е")
}

fn apply(filter: &mut Filter, tok: &[char], year: u16) -> Applied {
    let (negated, body) = match tok {
        ['-', rest @ ..] if !rest.is_empty() => (true, rest),
        _ => (false, tok),
    };
    if let ['#', name @ ..] = body {
        let name = norm(&unquote(name));
        if name.is_empty() {
            return Applied::Rejected(QueryErrorKind::EmptyValue);
        }
        let list = if negated {
            &mut filter.exclude_tags
        } else {
            &mut filter.tags
        };
        push_unique(list, name);
        return Applied::Done(TokenKind::Tag, negated);
    }
    if body.first() == Some(&'"') {
        return Applied::Text;
    }
    let Some(op) = body.iter().position(|c| matches!(c, ':' | '>' | '<' | '=')) else {
        return apply_keyword(filter, body, negated);
    };
    let Some((key, rest)) = body.split_at_checked(op) else {
        return Applied::Text;
    };
    let key = norm(&key.iter().collect::<String>());
    let rest: String = rest.iter().collect();
    let is_key = |names: &[&str]| names.contains(&key.as_str());

    if key.is_empty() && rest.starts_with(['>', '<', '=']) || is_key(AMOUNT_KEYS) {
        if negated {
            return Applied::Rejected(QueryErrorKind::NegationUnsupported);
        }
        let expr = rest.strip_prefix(':').unwrap_or(&rest);
        return match parse_amount(expr) {
            Some(range) => {
                let merged = filter.amount.map_or(range, |old| old.intersect(range));
                filter.amount = Some(merged);
                Applied::Done(TokenKind::Amount, false)
            }
            None => Applied::Rejected(QueryErrorKind::BadAmount),
        };
    }
    let Some(value) = rest.strip_prefix(':') else {
        return Applied::Text;
    };
    let kind = if is_key(CATEGORY_KEYS) {
        TokenKind::Category
    } else if is_key(STATUS_KEYS) {
        TokenKind::Status
    } else if is_key(KIND_KEYS) {
        TokenKind::Kind
    } else if is_key(MONTH_KEYS) {
        TokenKind::Month
    } else if is_key(SOURCE_KEYS) {
        TokenKind::Source
    } else {
        return Applied::Text;
    };
    let value = norm(
        value
            .strip_prefix('"')
            .map_or(value, |v| v.strip_suffix('"').unwrap_or(v)),
    );
    if value.is_empty() {
        return Applied::Rejected(QueryErrorKind::EmptyValue);
    }
    if negated && kind != TokenKind::Category {
        return Applied::Rejected(QueryErrorKind::NegationUnsupported);
    }
    let applied = match kind {
        TokenKind::Category => {
            let list = if negated {
                &mut filter.exclude_categories
            } else {
                &mut filter.categories
            };
            for name in value.split(',').map(str::trim).filter(|n| !n.is_empty()) {
                push_unique(list, CategoryRef(name.to_owned()));
            }
            Ok(())
        }
        TokenKind::Status => lookup_list(STATUSES, &value, &mut filter.statuses),
        TokenKind::Kind => lookup_list(KINDS, &value, &mut filter.kinds),
        TokenKind::Source => lookup(SOURCES, &value)
            .map(|s| filter.source = Some(s))
            .ok_or(()),
        TokenKind::Month => parse_months(&value, year)
            .map(|m| filter.months = Some(m))
            .ok_or(()),
        TokenKind::Text | TokenKind::Amount | TokenKind::Tag | TokenKind::Record => Err(()),
    };
    match applied {
        Ok(()) => Applied::Done(kind, negated),
        Err(()) if kind == TokenKind::Month => Applied::Rejected(QueryErrorKind::BadMonth),
        Err(()) => Applied::Rejected(QueryErrorKind::UnknownValue),
    }
}

/// Слово без ключа: «доход» / «расход».
fn apply_keyword(filter: &mut Filter, body: &[char], negated: bool) -> Applied {
    if negated {
        return Applied::Text;
    }
    let word = norm(&body.iter().collect::<String>());
    let Some(record) = RECORDS
        .iter()
        .find(|(names, _)| names.contains(&word.as_str()))
        .map(|(_, r)| *r)
    else {
        return Applied::Text;
    };
    filter.record = if filter.record == RecordKind::Any || filter.record == record {
        record
    } else {
        RecordKind::Any
    };
    Applied::Done(TokenKind::Record, false)
}

fn push_unique<T: PartialEq>(list: &mut Vec<T>, item: T) {
    if !list.contains(&item) {
        list.push(item);
    }
}

/// Точное совпадение, затем префикс от трёх символов («внеп» → «внеплан»).
fn lookup<T: Copy>(table: &Aliases<T>, value: &str) -> Option<T> {
    let exact = table
        .iter()
        .find(|(names, _)| names.contains(&value))
        .map(|(_, v)| *v);
    exact.or_else(|| {
        if value.chars().count() < 3 {
            return None;
        }
        table
            .iter()
            .find(|(names, _)| names.iter().any(|n| n.starts_with(value)))
            .map(|(_, v)| *v)
    })
}

/// Список через запятую; при ошибке в любом элементе фильтр не меняется.
fn lookup_list<T: Copy + PartialEq>(
    table: &Aliases<T>,
    value: &str,
    out: &mut Vec<T>,
) -> Result<(), ()> {
    let items: Option<Vec<T>> = value
        .split(',')
        .map(str::trim)
        .map(|v| lookup(table, v))
        .collect();
    for item in items.ok_or(())? {
        push_unique(out, item);
    }
    Ok(())
}

fn parse_amount(expr: &str) -> Option<AmountRange> {
    let num = |s: &str| Money::from_rub_str(s).ok().filter(|m| !m.is_negative());
    let one = Money::from_kopecks(1);
    if let Some((a, b)) = expr.split_once("..") {
        let min = if a.is_empty() { None } else { Some(num(a)?) };
        let max = if b.is_empty() { None } else { Some(num(b)?) };
        return (min.is_some() || max.is_some()).then_some(AmountRange { min, max });
    }
    if let Some(n) = expr.strip_prefix(">=") {
        return Some(AmountRange {
            min: Some(num(n)?),
            max: None,
        });
    }
    if let Some(n) = expr.strip_prefix("<=") {
        return Some(AmountRange {
            min: None,
            max: Some(num(n)?),
        });
    }
    if let Some(n) = expr.strip_prefix('>') {
        return Some(AmountRange {
            min: Some(num(n)?.checked_add(one).ok()?),
            max: None,
        });
    }
    if let Some(n) = expr.strip_prefix('<') {
        return Some(AmountRange {
            min: None,
            max: Some(num(n)?.checked_sub(one).ok()?),
        });
    }
    let exact = num(expr.strip_prefix('=').unwrap_or(expr))?;
    Some(AmountRange {
        min: Some(exact),
        max: Some(exact),
    })
}

/// «2026-09», «сен», «2026», «июн..сен», «2026-06..2026-09».
fn parse_months(value: &str, year: u16) -> Option<MonthRange> {
    if let Some((a, b)) = value.split_once("..") {
        let from = parse_one_month(a, year)?.from;
        let to = parse_one_month(b, year)?.to;
        return (from <= to).then_some(MonthRange { from, to });
    }
    parse_one_month(value, year)
}

fn parse_one_month(value: &str, year: u16) -> Option<MonthRange> {
    let single = |m: YearMonth| MonthRange { from: m, to: m };
    if value.len() == 4 && value.bytes().all(|b| b.is_ascii_digit()) {
        let y: u16 = value.parse().ok()?;
        return Some(MonthRange {
            from: YearMonth::new(y, 1).ok()?,
            to: YearMonth::new(y, 12).ok()?,
        });
    }
    if let Ok(m) = YearMonth::parse(value) {
        return Some(single(m));
    }
    if value.chars().count() < 3 {
        return None;
    }
    let idx = MONTH_NAMES.iter().position(|n| n.starts_with(value))?;
    let month = u8::try_from(idx.checked_add(1)?).ok()?;
    YearMonth::new(year, month).ok().map(single)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn p(s: &str) -> ParsedQuery {
        parse(s, 2026)
    }

    fn ym(y: u16, m: u8) -> YearMonth {
        YearMonth::new(y, m).unwrap()
    }

    fn rub(n: i64) -> Money {
        Money::from_kopecks(n * 100)
    }

    #[test]
    fn plain_words_become_terms() {
        let q = p("лента  \"подарок коллеге\"");
        let texts: Vec<_> = q.text.iter().map(|t| (t.text.as_str(), t.phrase)).collect();
        assert_eq!(texts, [("лента", false), ("подарок коллеге", true)]);
        assert_eq!(q.filter, Filter::default());
    }

    #[test]
    fn full_example_from_docs() {
        let q = p("лента статус:оплачено период:июн..сен сумма>10000");
        assert_eq!(q.text.len(), 1);
        assert_eq!(q.filter.statuses, [TxStatus::Paid]);
        assert_eq!(
            q.filter.months,
            Some(MonthRange {
                from: ym(2026, 6),
                to: ym(2026, 9)
            })
        );
        assert_eq!(
            q.filter.amount,
            Some(AmountRange {
                min: Some(Money::from_kopecks(1_000_001)),
                max: None
            })
        );
        assert!(q.errors.is_empty());
    }

    #[test]
    fn amount_forms() {
        let cases = [
            ("сумма>5000", Some(Money::from_kopecks(500_001)), None),
            ("сумма<=1000", None, Some(rub(1000))),
            ("amount<1000", None, Some(Money::from_kopecks(99_999))),
            ("сумма:1000..5000", Some(rub(1000)), Some(rub(5000))),
            ("сумма:>=700", Some(rub(700)), None),
            (">5000", Some(Money::from_kopecks(500_001)), None),
            ("сумма:..300", None, Some(rub(300))),
            (
                "сумма=250,5",
                Some(Money::from_kopecks(25_050)),
                Some(Money::from_kopecks(25_050)),
            ),
        ];
        for (input, min, max) in cases {
            let q = p(input);
            assert_eq!(q.filter.amount, Some(AmountRange { min, max }), "{input}");
            assert_eq!(q.spans.first().map(|s| s.kind), Some(TokenKind::Amount));
        }
    }

    #[test]
    fn two_amount_tokens_intersect() {
        let q = p(">1000 <=5000");
        assert_eq!(
            q.filter.amount,
            Some(AmountRange {
                min: Some(Money::from_kopecks(100_001)),
                max: Some(rub(5000))
            })
        );
    }

    #[test]
    fn status_aliases() {
        let cases = [
            ("статус:план", vec![TxStatus::Planned]),
            ("status:paid", vec![TxStatus::Paid]),
            ("статус:оплачено,долг", vec![TxStatus::Paid, TxStatus::Debt]),
            ("статус:внеплан", vec![TxStatus::Unplanned]),
            ("статус:незапланировано", vec![TxStatus::Unplanned]),
            ("СТАТУС:Оплачено", vec![TxStatus::Paid]),
        ];
        for (input, expected) in cases {
            assert_eq!(p(input).filter.statuses, expected, "{input}");
        }
    }

    #[test]
    fn category_kind_source_record() {
        let q = p("кат:\"хобби и игры\" cat:прод тип:желания src:импорт расход");
        assert_eq!(
            q.filter.categories,
            [
                CategoryRef("хобби и игры".into()),
                CategoryRef("прод".into())
            ]
        );
        assert_eq!(q.filter.kinds, [CategoryKind::Wants]);
        assert_eq!(q.filter.source, Some(Source::Import));
        assert_eq!(q.filter.record, RecordKind::Expense);
        assert!(q.text.is_empty());
    }

    #[test]
    fn conflicting_record_words_cancel_out() {
        assert_eq!(p("доход расход").filter.record, RecordKind::Any);
    }

    #[test]
    fn month_forms() {
        let cases = [
            ("мес:сен", (2026, 9, 2026, 9)),
            ("мес:2026-09", (2026, 9, 2026, 9)),
            ("период:июн..сен", (2026, 6, 2026, 9)),
            ("период:2026", (2026, 1, 2026, 12)),
            ("month:2025-11..фев", (2025, 11, 2026, 2)),
            ("мес:сентябрь", (2026, 9, 2026, 9)),
            ("период:2025..2026", (2025, 1, 2026, 12)),
        ];
        for (input, (fy, fm, ty, tm)) in cases {
            let range = p(input).filter.months;
            let expected = MonthRange {
                from: ym(fy, fm),
                to: ym(ty, tm),
            };
            assert_eq!(range, Some(expected), "{input}");
        }
    }

    #[test]
    fn year_defaults_to_current() {
        let q = parse("мес:мар", 2031);
        assert_eq!(q.filter.months.map(|m| m.from), Some(ym(2031, 3)));
    }

    #[test]
    fn yo_equals_ye() {
        let a = p("кат:ёлка тип:займы");
        assert_eq!(a.filter.categories, [CategoryRef("елка".into())]);
    }

    #[test]
    fn tags_and_negation() {
        let q = p("#Подарки -#работа -кат:подписки");
        assert_eq!(q.filter.tags, ["подарки"]);
        assert_eq!(q.filter.exclude_tags, ["работа"]);
        assert_eq!(
            q.filter.exclude_categories,
            [CategoryRef("подписки".into())]
        );
        assert!(
            q.spans
                .iter()
                .all(|s| s.negated || s.kind == TokenKind::Tag)
        );
        assert_eq!(q.spans.iter().filter(|s| s.negated).count(), 2);
    }

    #[test]
    fn spans_cover_tokens_in_chars() {
        let q = p("лента кат:еда");
        assert_eq!(q.spans[0].range, 0..5);
        assert_eq!(q.spans[1].range, 6..13);
        assert_eq!(q.spans[1].kind, TokenKind::Category);
    }

    #[test]
    fn unknown_values_fall_back_to_text_with_error() {
        let cases = [
            ("статус:абракадабра", QueryErrorKind::UnknownValue),
            ("мес:вчера", QueryErrorKind::BadMonth),
            ("период:сен..июн", QueryErrorKind::BadMonth),
            ("сумма>много", QueryErrorKind::BadAmount),
            ("кат:", QueryErrorKind::EmptyValue),
            ("-статус:план", QueryErrorKind::NegationUnsupported),
        ];
        for (input, kind) in cases {
            let q = p(input);
            assert_eq!(q.errors.first().map(|e| e.kind), Some(kind), "{input}");
            assert_eq!(q.filter, Filter::default(), "{input}");
        }
    }

    #[test]
    fn unknown_keys_and_numbers_are_text() {
        let q = p("12:30 foo:bar -100 -");
        assert_eq!(q.text.len(), 4);
        assert!(q.errors.is_empty());
    }

    #[test]
    fn unterminated_quote_takes_rest() {
        let q = p("кат:\"хобби и");
        assert_eq!(q.filter.categories, [CategoryRef("хобби и".into())]);
    }

    #[test]
    fn filter_roundtrips_through_json() {
        let q = p("статус:долг сумма:100..200 период:2026 #а -кат:б");
        let json = serde_json::to_string(&q.filter).unwrap();
        assert_eq!(serde_json::from_str::<Filter>(&json).unwrap(), q.filter);
    }

    proptest! {
        #[test]
        fn never_panics_and_spans_stay_in_bounds(s in ".*") {
            let q = parse(&s, 2026);
            let len = s.chars().count();
            let mut prev_end = 0;
            for span in &q.spans {
                prop_assert!(span.range.start >= prev_end && span.range.end <= len);
                prop_assert!(span.range.start < span.range.end);
                prev_end = span.range.end;
            }
            for e in &q.errors {
                prop_assert!(e.range.end <= len);
            }
        }

        #[test]
        fn token_soup_never_panics(s in "[ а-яa-z0-9:<>=.,#\"-]{0,40}") {
            let _ = parse(&s, 2026);
        }
    }
}
