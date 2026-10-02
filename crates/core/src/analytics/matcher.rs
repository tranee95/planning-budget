//! Применение `Filter` к `DataSet` в памяти: то же, что `storage::search` делает SQL-ом.

use std::collections::BTreeSet;

use crate::model::{Category, CategoryId, DataSet, Income, Transaction};
use crate::period::YearMonth;
use crate::query::{CategoryRef, Filter, RecordKind};

/// Нижний регистр и `ё = е`, как `norm()` в хранилище.
pub(crate) fn normalize(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| if c == 'ё' { 'е' } else { c })
        .collect()
}

/// Фильтр, у которого ссылки на категории уже разрешены в id.
pub(crate) struct Matcher {
    filter: Filter,
    /// `None` — положительного условия по категориям нет.
    categories: Option<BTreeSet<CategoryId>>,
    excluded: BTreeSet<CategoryId>,
    tags: BTreeSet<String>,
    excluded_tags: BTreeSet<String>,
}

fn resolve(refs: &[CategoryRef], categories: &[Category]) -> BTreeSet<CategoryId> {
    let needles: Vec<String> = refs.iter().map(|r| normalize(&r.0)).collect();
    categories
        .iter()
        .filter(|c| {
            let name = normalize(&c.name);
            needles.iter().any(|n| {
                name.starts_with(n.as_str()) || name.split(' ').any(|w| w.starts_with(n.as_str()))
            })
        })
        .map(|c| c.id)
        .collect()
}

impl Matcher {
    pub fn new(filter: Filter, data: &DataSet) -> Self {
        Self {
            categories: (!filter.categories.is_empty())
                .then(|| resolve(&filter.categories, &data.categories)),
            excluded: resolve(&filter.exclude_categories, &data.categories),
            tags: filter.tags.iter().map(|t| normalize(t)).collect(),
            excluded_tags: filter.exclude_tags.iter().map(|t| normalize(t)).collect(),
            filter,
        }
    }

    pub fn month(&self, month: YearMonth) -> bool {
        self.filter
            .months
            .is_none_or(|range| range.from <= month && month <= range.to)
    }

    fn amount(&self, amount: crate::Money) -> bool {
        self.filter.amount.is_none_or(|range| {
            range.min.is_none_or(|min| amount >= min) && range.max.is_none_or(|max| amount <= max)
        })
    }

    /// Категория проходит условия `кат:`, `-кат:` и `тип:`; нужна и самой трате, и строке лимитов.
    pub fn category(&self, category: &Category) -> bool {
        self.categories
            .as_ref()
            .is_none_or(|ids| ids.contains(&category.id))
            && !self.excluded.contains(&category.id)
            && (self.filter.kinds.is_empty() || self.filter.kinds.contains(&category.kind))
    }

    /// `tags` — имена тегов траты.
    pub fn transaction(&self, tx: &Transaction, category: &Category, tags: &[String]) -> bool {
        if self.filter.record == RecordKind::Income
            || !self.month(tx.month)
            || !self.amount(tx.amount)
            || !self.category(category)
            || !(self.filter.statuses.is_empty() || self.filter.statuses.contains(&tx.status))
        {
            return false;
        }
        if self.tags.is_empty() && self.excluded_tags.is_empty() {
            return true;
        }
        let tags: BTreeSet<String> = tags.iter().map(|t| normalize(t)).collect();
        (self.tags.is_empty() || self.tags.iter().any(|t| tags.contains(t)))
            && self.excluded_tags.is_disjoint(&tags)
    }

    /// Доходов не касаются условия по категориям, типам, статусам и тегам: при них доходов нет.
    pub fn income(&self, income: &Income) -> bool {
        self.filter.record != RecordKind::Expense
            && self.filter.categories.is_empty()
            && self.filter.kinds.is_empty()
            && self.filter.statuses.is_empty()
            && self.filter.tags.is_empty()
            && self.month(income.month)
            && self.amount(income.amount)
    }
}
