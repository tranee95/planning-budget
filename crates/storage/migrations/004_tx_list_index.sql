-- Таблица «Расходы» (`transaction_search`) сортирует по month DESC, date DESC, id DESC.
-- Индекс читается с конца и отдаёт этот порядок без временного B-дерева; с LIMIT запрос
-- останавливается на первых строках. Колонки category_id, amount и status делают индекс
-- покрывающим для итогов (count, sum) по фильтрам статуса и категории.
CREATE INDEX ix_tx_list ON transactions(month, date, id, category_id, amount, status)
  WHERE deleted_at IS NULL;
