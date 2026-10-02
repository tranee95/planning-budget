-- План сбережений задаётся процентом на каждую категорию типа 'savings'.
-- Общий план месяца — сумма процентов категорий.

CREATE TABLE savings_category_rates (
  category_id  INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
  valid_from   TEXT    NOT NULL,
  rate_bp      INTEGER NOT NULL CHECK (rate_bp BETWEEN 0 AND 10000),
  PRIMARY KEY (category_id, valid_from)
);

-- Переносим старую норму на существующие категории сбережений.
INSERT INTO savings_category_rates (category_id, valid_from, rate_bp)
SELECT c.id,
       COALESCE((SELECT min(valid_from) FROM category_limits), strftime('%Y-%m', 'now')),
       COALESCE((SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'savings.target_norm_bp'), 1400)
FROM categories c
WHERE c.kind = 'savings';

-- Старый ручной процент месяца относился ко всему плану: переносим его на каждую категорию сбережений.
CREATE TABLE savings_plan_overrides_new (
  month        TEXT    NOT NULL,
  category_id  INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
  rate_bp      INTEGER NOT NULL CHECK (rate_bp BETWEEN 0 AND 10000),
  PRIMARY KEY (month, category_id)
);
INSERT INTO savings_plan_overrides_new (month, category_id, rate_bp)
SELECT o.month, c.id, o.rate_bp
FROM savings_plan_overrides o JOIN categories c ON c.kind = 'savings';
DROP TABLE savings_plan_overrides;
ALTER TABLE savings_plan_overrides_new RENAME TO savings_plan_overrides;
