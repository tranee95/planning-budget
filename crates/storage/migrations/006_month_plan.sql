-- План месяца и его фиксация «План готов».

-- Плановая сумма траты на момент фиксации плана; NULL — вне плана.
ALTER TABLE transactions ADD COLUMN planned_amount INTEGER
  CHECK (planned_amount IS NULL OR planned_amount > 0);

-- Индекс чтения набора данных остаётся покрывающим и с новой колонкой.
DROP INDEX ix_tx_ledger;
CREATE INDEX ix_tx_ledger ON transactions(month, category_id, sort_key, id, title, amount, status, planned_amount)
  WHERE deleted_at IS NULL;

-- Строка есть — план месяца зафиксирован; разблокировка удаляет строку.
CREATE TABLE month_plans (
  month               TEXT    PRIMARY KEY,
  locked_at           TEXT    NOT NULL,
  repayments_planned  INTEGER NOT NULL DEFAULT 0 CHECK (repayments_planned >= 0)
);

-- План по накоплениям на момент фиксации.
CREATE TABLE month_plan_savings (
  month       TEXT    NOT NULL REFERENCES month_plans(month) ON DELETE CASCADE,
  category_id INTEGER NOT NULL REFERENCES categories(id),
  amount      INTEGER NOT NULL CHECK (amount >= 0),
  PRIMARY KEY (month, category_id)
);
