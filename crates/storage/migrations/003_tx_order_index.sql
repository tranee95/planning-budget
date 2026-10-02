-- Чтение набора данных (`Db::dataset`) сортирует траты по (month, category_id, sort_key, id).
-- Прежний индекс (month, category_id) этого порядка не давал: SQLite сортировал строки во
-- временном B-дереве (50 000 трат — 166 мс). Новый индекс отдаёт строки уже в нужном порядке
-- и покрывает выбираемые колонки, поэтому обращений к таблице нет (6 мс).
-- Список колонок индекса и запрос `period_transactions` меняются вместе.
DROP INDEX ix_tx_month_cat;
CREATE INDEX ix_tx_ledger ON transactions(month, category_id, sort_key, id, title, amount, status)
  WHERE deleted_at IS NULL;
