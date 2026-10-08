-- Строка истории с amount = NULL означает «лимита нет с этого месяца».
-- Раньше убрать лимит было нельзя: удаление строки возвращало предыдущий.
CREATE TABLE category_limits_new (
  category_id  INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
  valid_from   TEXT    NOT NULL,
  amount       INTEGER CHECK (amount IS NULL OR amount >= 0),
  PRIMARY KEY (category_id, valid_from)
);
INSERT INTO category_limits_new (category_id, valid_from, amount)
SELECT category_id, valid_from, amount FROM category_limits;
DROP TABLE category_limits;
ALTER TABLE category_limits_new RENAME TO category_limits;
