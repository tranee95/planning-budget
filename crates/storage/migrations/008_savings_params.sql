-- Накопления: параметры ставки и налога на категорию, план процентом или суммой
--.

CREATE TABLE savings_params (
  category_id      INTEGER PRIMARY KEY REFERENCES categories(id) ON DELETE CASCADE,
  annual_rate_bp   INTEGER NOT NULL DEFAULT 0 CHECK (annual_rate_bp BETWEEN 0 AND 10000),
  tax_bp           INTEGER NOT NULL DEFAULT 0 CHECK (tax_bp BETWEEN 0 AND 10000),
  initial_balance  INTEGER NOT NULL DEFAULT 0 CHECK (initial_balance >= 0),
  initial_month    TEXT    NOT NULL
);

-- Настройки облигаций переходят на первую по порядку категорию сбережений (неархивную, если есть);
-- остальные накопления начинаются без процентов. Ключи bonds.* пока остаются: их читает старая
-- страница «Облигации», они убираются миграцией 009 вместе с ней.
INSERT INTO savings_params (category_id, annual_rate_bp, tax_bp, initial_balance, initial_month)
SELECT c.id,
       CASE WHEN c.id = first.id
            THEN COALESCE((SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'bonds.rate_bp'), 0)
            ELSE 0 END,
       CASE WHEN c.id = first.id
            THEN COALESCE((SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'bonds.coupon_tax_bp'), 0)
            ELSE 0 END,
       CASE WHEN c.id = first.id
            THEN COALESCE((SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'bonds.initial_balance'), 0)
            ELSE 0 END,
       COALESCE((SELECT trim(value, '"') FROM settings WHERE key = 'bonds.initial_month'),
                strftime('%Y-%m', 'now'))
FROM categories c
JOIN (SELECT id FROM categories WHERE kind = 'savings'
      ORDER BY archived_at IS NOT NULL, sort_order, id LIMIT 1) AS first
WHERE c.kind = 'savings';

-- План накопления: процент от дохода или фиксированная сумма на выбор.
CREATE TABLE savings_category_rates_new (
  category_id  INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
  valid_from   TEXT    NOT NULL,
  plan_kind    TEXT    NOT NULL DEFAULT 'percent' CHECK (plan_kind IN ('percent', 'fixed')),
  rate_bp      INTEGER NOT NULL DEFAULT 0 CHECK (rate_bp BETWEEN 0 AND 10000),
  amount       INTEGER CHECK (amount IS NULL OR amount >= 0),
  PRIMARY KEY (category_id, valid_from),
  CHECK ((plan_kind = 'percent' AND amount IS NULL) OR (plan_kind = 'fixed' AND amount IS NOT NULL))
);
INSERT INTO savings_category_rates_new (category_id, valid_from, plan_kind, rate_bp, amount)
SELECT category_id, valid_from, 'percent', rate_bp, NULL FROM savings_category_rates;
DROP TABLE savings_category_rates;
ALTER TABLE savings_category_rates_new RENAME TO savings_category_rates;
