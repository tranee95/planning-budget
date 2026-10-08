//! Текст запроса: выражение FTS5 и сопоставление слов без FTS.

use planning_budget_core::YearMonth;
use planning_budget_core::query::Term;

use crate::migrations::normalize;

/// Запрос FTS5: слово → `"слово"*`, фраза → `"a b"`; слова через пробел (AND).
/// Текст есть, но в нём нет ни букв, ни цифр (`???`, `-`): искать нечего, результат пуст,
/// а не «всё подряд», как при пустом запросе.
pub(crate) fn matches_nothing(text: &[Term], fts: Option<&str>) -> bool {
    !text.is_empty() && fts.is_none()
}

pub(crate) fn fts_match(text: &[Term]) -> Option<String> {
    let parts: Vec<String> = text
        .iter()
        .filter_map(|term| {
            let norm = normalize(&term.text);
            if !norm.chars().any(char::is_alphanumeric) {
                return None;
            }
            let quoted = norm.replace('"', "\"\"");
            Some(if term.phrase {
                format!("\"{quoted}\"")
            } else {
                format!("\"{quoted}\"*")
            })
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join(" "))
}

/// Слова поискового текста, каждое из которых начинает слово в `haystack` (или фраза входит).
pub(super) fn terms_match(text: &[Term], haystack: &str) -> bool {
    let hay = normalize(haystack);
    text.iter().all(|term| {
        let needle = normalize(&term.text);
        if term.phrase {
            hay.contains(&needle)
        } else {
            hay.split(' ').any(|word| word.starts_with(&needle))
        }
    })
}

/// Дешёвая проверка до запроса: группировка по месяцам читает всю таблицу.
pub(super) fn could_name_a_month(term: &Term) -> bool {
    let needle = normalize(&term.text);
    needle.starts_with(|c: char| c.is_ascii_digit())
        || (YearMonth::new(2000, 1).is_ok()
            && (1..=12).any(|m| {
                YearMonth::new(2000, m)
                    .is_ok_and(|ym| normalize(ym.month_name_ru()).starts_with(&needle))
            }))
}
