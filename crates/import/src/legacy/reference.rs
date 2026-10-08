//! Лист «Справочник»: категории и настройки.

use calamine::Data;
use planning_budget_core::{CategoryKind, LegacyCategory, LegacySettings, Money};

use super::SHEET_REFERENCE;
use super::cells::*;
use crate::ImportError;

pub(super) fn reference(
    rows: &Rows,
) -> Result<(Vec<LegacyCategory>, Option<LegacySettings>), ImportError> {
    let header = rows
        .iter()
        .position(|r| matches!(r.first(), Some(Data::String(s)) if s.trim() == "Категория"))
        .ok_or(ImportError::Layout {
            sheet: SHEET_REFERENCE,
            row: 0,
        })?;
    let mut categories = Vec::new();
    for row in header + 1..rows.len() {
        let Some(name) = text(rows, row, 0) else {
            break;
        };
        let kind = kind(
            &text(rows, row, 1).ok_or(ImportError::UnknownKind {
                row: row_number(row),
            })?,
            row,
        )?;
        let limit = if kind == CategoryKind::Savings {
            None
        } else {
            rubles(rows, row, 2)?
        };
        categories.push(LegacyCategory {
            name,
            kind,
            limit,
            note: text(rows, row, 5),
        });
    }

    let mut values = [None::<f64>; 6];
    for row in header..rows.len() {
        let Some(label) = text(rows, row, 12) else {
            continue;
        };
        let value = match cell(rows, row, 13) {
            Data::Float(v) => *v,
            #[allow(
                clippy::cast_precision_loss,
                reason = "значения настроек много меньше 2^53"
            )]
            Data::Int(v) => *v as f64,
            _ => continue,
        };
        let label = label.to_lowercase();
        let slot = if label.contains("минимум") {
            0
        } else if label.contains("норма") && label.contains("цель") {
            1
        } else if label.contains("максимум") {
            2
        } else if label.starts_with("доходность облигаций") {
            3
        } else if label.starts_with("ндфл") {
            4
        } else if label.starts_with("уже вложено") {
            5
        } else {
            continue;
        };
        if let Some(target) = values.get_mut(slot) {
            *target = Some(value);
        }
    }
    let settings = match values {
        [
            Some(min),
            Some(norm),
            Some(max),
            Some(rate),
            Some(tax),
            Some(initial),
        ] => Some(LegacySettings {
            savings_min: basis_points(min, header)?,
            savings_norm: basis_points(norm, header)?,
            savings_max: basis_points(max, header)?,
            bonds_rate: basis_points(rate, header)?,
            bonds_coupon_tax: basis_points(tax, header)?,
            bonds_initial_balance: Money::from_kopecks_f64((initial * 100.0).round()).map_err(
                |_| ImportError::Number {
                    sheet: SHEET_REFERENCE,
                    row: row_number(header),
                },
            )?,
        }),
        _ => None,
    };
    Ok((categories, settings))
}
