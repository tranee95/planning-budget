//! Поиск палитры: разбор запроса `core::query` и выполнение `storage::search`.

use chrono::{DateTime, NaiveDate, Utc};
use planning_budget_core::YearMonth;
use planning_budget_core::query::{ParsedQuery, parse};
use planning_budget_storage::{Db, LIST_LIMIT, SearchResult};

use crate::AppError;
use crate::dto::{
    CategoryHitDto, CategoryUsageDto, FilterScreenDto, IncomeDto, IncomeGroupDto, IncomeHitDto,
    IncomeSearchDto, MonthHitDto, QueryHintSpanDto, QueryParseDto, SavedFilterDto, SearchResultDto,
    TitleSuggestionDto, TokenSpanDto, TransactionDto, TransactionGroupDto, TransactionHitDto,
    TransactionSearchDto,
};
use crate::services::month_of;

/// Сколько строк группы показывает палитра.
const PER_GROUP: usize = 8;

/// Длиннее запрос не нужен: токены и FTS-запрос растут вместе с ним, а сохранить фильтр можно только до этого предела.
const MAX_QUERY_CHARS: usize = 500;

fn clip(query: &str) -> &str {
    match query.char_indices().nth(MAX_QUERY_CHARS) {
        Some((end, _)) => &query[..end],
        None => query,
    }
}

fn pos(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

pub fn run(
    db: &Db,
    query: &str,
    request_id: u32,
    current_year: u16,
) -> Result<SearchResultDto, AppError> {
    let parsed = parse(clip(query), current_year);
    let found = db.search(&parsed.text, &parsed.filter, PER_GROUP)?;
    Ok(to_dto(request_id, &parsed, &found))
}

fn spans(parsed: &ParsedQuery) -> Vec<TokenSpanDto> {
    parsed
        .spans
        .iter()
        .map(|s| TokenSpanDto {
            start: pos(s.range.start),
            end: pos(s.range.end),
            kind: s.kind.into(),
            negated: s.negated,
        })
        .collect()
}

fn hints(parsed: &ParsedQuery) -> Vec<QueryHintSpanDto> {
    parsed
        .errors
        .iter()
        .map(|e| QueryHintSpanDto {
            start: pos(e.range.start),
            end: pos(e.range.end),
            hint: e.kind.into(),
        })
        .collect()
}

fn to_dto(request_id: u32, parsed: &ParsedQuery, found: &SearchResult) -> SearchResultDto {
    SearchResultDto {
        request_id,
        spans: spans(parsed),
        hints: hints(parsed),
        transactions: TransactionGroupDto {
            total: found.transactions.total,
            sum: found.transactions.sum.kopecks(),
            items: found
                .transactions
                .items
                .iter()
                .map(|t| TransactionHitDto {
                    id: t.id.0,
                    title: t.title.clone(),
                    category_id: t.category_id.0,
                    category: t.category.clone(),
                    month: t.month.to_string(),
                    date: t.date.map(|d| d.to_string()),
                    amount: t.amount.kopecks(),
                    status: t.status.into(),
                })
                .collect(),
        },
        incomes: IncomeGroupDto {
            total: found.incomes.total,
            sum: found.incomes.sum.kopecks(),
            items: found
                .incomes
                .items
                .iter()
                .map(|i| IncomeHitDto {
                    id: i.id.0,
                    source_name: i.source_name.clone(),
                    month: i.month.to_string(),
                    date: i.date.map(|d| d.to_string()),
                    amount: i.amount.kopecks(),
                    status: i.status.into(),
                })
                .collect(),
        },
        categories: found
            .categories
            .iter()
            .map(|c| CategoryHitDto {
                id: c.id.0,
                name: c.name.clone(),
            })
            .collect(),
        months: found
            .months
            .iter()
            .map(|m| MonthHitDto {
                month: m.month.to_string(),
                records: m.records,
            })
            .collect(),
    }
}

/// Только разбор строки: данные не читаются, поэтому БД не нужна.
pub fn query_parse(query: &str, current_year: u16) -> QueryParseDto {
    let parsed = parse(clip(query), current_year);
    QueryParseDto {
        spans: spans(&parsed),
        hints: hints(&parsed),
    }
}

/// Полный список трат по запросу: строки таблицы «Расходы» при активном фильтре.
pub fn tx_list(
    db: &Db,
    query: &str,
    request_id: u32,
    current_year: u16,
) -> Result<TransactionSearchDto, AppError> {
    let parsed = parse(clip(query), current_year);
    let found = db.transaction_search(&parsed.text, &parsed.filter, LIST_LIMIT)?;
    Ok(TransactionSearchDto {
        request_id,
        spans: spans(&parsed),
        hints: hints(&parsed),
        total: found.total,
        sum: found.sum.kopecks(),
        truncated: found.total > found.items.len() as u64,
        items: found.items.iter().map(TransactionDto::from).collect(),
    })
}

pub fn income_list(
    db: &Db,
    query: &str,
    request_id: u32,
    current_year: u16,
) -> Result<IncomeSearchDto, AppError> {
    let parsed = parse(clip(query), current_year);
    let found = db.income_search(&parsed.text, &parsed.filter, LIST_LIMIT)?;
    Ok(IncomeSearchDto {
        request_id,
        spans: spans(&parsed),
        hints: hints(&parsed),
        total: found.total,
        sum: found.sum.kopecks(),
        truncated: found.total > found.items.len() as u64,
        items: found.items.iter().map(IncomeDto::from).collect(),
    })
}

pub fn filters_list(db: &Db) -> Result<Vec<SavedFilterDto>, AppError> {
    Ok(db
        .saved_filters()?
        .into_iter()
        .map(SavedFilterDto::from)
        .collect())
}

pub fn filter_save(
    db: &mut Db,
    name: &str,
    query: &str,
    screen: FilterScreenDto,
    now: DateTime<Utc>,
) -> Result<SavedFilterDto, AppError> {
    Ok(db
        .saved_filter_save(name, query, screen.into(), now)?
        .into())
}

pub fn filter_delete(db: &mut Db, id: i64) -> Result<(), AppError> {
    Ok(db.saved_filter_delete(id)?)
}

/// Сколько подсказок наименований показывает быстрое добавление.
const SUGGESTIONS: usize = 5;

/// За сколько последних месяцев считается частота категорий.
const USAGE_MONTHS: u8 = 6;

pub fn suggest(db: &Db, query: &str) -> Result<Vec<TitleSuggestionDto>, AppError> {
    Ok(db
        .title_suggestions(query, SUGGESTIONS)?
        .into_iter()
        .map(|s| TitleSuggestionDto {
            title: s.title,
            category_id: s.category_id.0,
            uses: s.uses,
        })
        .collect())
}

pub fn category_usage(db: &Db, today: NaiveDate) -> Result<Vec<CategoryUsageDto>, AppError> {
    let mut since: YearMonth = month_of(today)?;
    for _ in 1..USAGE_MONTHS {
        since = since.pred().unwrap_or(since);
    }
    Ok(db
        .category_usage(since)?
        .into_iter()
        .map(|(id, uses)| CategoryUsageDto {
            category_id: id.0,
            uses,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_parse_returns_chips_and_hints_without_a_database() {
        let parsed = query_parse("тип:желания сумма>500 кофе", 2026);
        assert_eq!(parsed.spans.len(), 3);
        assert!(parsed.hints.is_empty());
        // Границы — в символах, как у tx_search: чипы режут строку по ним.
        assert_eq!((parsed.spans[0].start, parsed.spans[0].end), (0, 11));
    }

    #[test]
    fn query_parse_flags_unknown_values() {
        let parsed = query_parse("статус:несуществует", 2026);
        assert_eq!(parsed.hints.len(), 1);
    }

    #[test]
    fn empty_query_has_no_chips() {
        let parsed = query_parse("   ", 2026);
        assert!(parsed.spans.is_empty() && parsed.hints.is_empty());
    }
}
