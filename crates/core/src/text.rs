//! Нормализация текста для сравнения и поиска.
//!
//! Единственное определение: на нём построены данные FTS, уникальность имён категорий и
//! сохранённых фильтров (хранилище регистрирует его как SQL-функцию `norm()`), разбор запроса и
//! сопоставление фильтра в аналитике. Менять поведение нельзя без миграции данных.

/// Нижний регистр, `ё` → `е`, любые пробельные последовательности схлопнуты в один пробел.
///
/// ```
/// use planning_budget_core::text::normalize;
/// assert_eq!(normalize("  Лёд   ЛЕНТА\t248 "), "лед лента 248");
/// ```
#[must_use]
pub fn normalize(text: &str) -> String {
    // Без промежуточных `Vec` и `replace`: функцию зовут на каждую трату и тег в аналитике.
    let lower = text.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    for word in lower.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.extend(word.chars().map(|c| if c == 'ё' { 'е' } else { c }));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn lowercases_folds_yo_and_collapses_spaces() {
        assert_eq!(normalize("  Лёд   ЛЕНТА\t248 "), "лед лента 248");
    }

    #[test]
    fn category_names_with_double_spaces_match_their_single_spaced_query() {
        assert_eq!(normalize("Еда  дома"), normalize("еда дома"));
    }
}
