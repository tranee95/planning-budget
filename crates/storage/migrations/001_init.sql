--. Менять нельзя: изменения схемы — только новой миграцией.

CREATE TABLE categories (
  id           INTEGER PRIMARY KEY,
  name         TEXT    NOT NULL,
  kind         TEXT    NOT NULL CHECK (kind IN ('mandatory','wants','savings','loans')),
  color        TEXT    NOT NULL,
  icon         TEXT,
  sort_order   INTEGER NOT NULL,
  note         TEXT,
  archived_at  TEXT,
  created_at   TEXT    NOT NULL,
  updated_at   TEXT    NOT NULL
);
CREATE UNIQUE INDEX ux_categories_name_active ON categories(norm(name)) WHERE archived_at IS NULL;

CREATE TABLE category_limits (
  category_id  INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
  valid_from   TEXT    NOT NULL,
  amount       INTEGER NOT NULL CHECK (amount >= 0),
  PRIMARY KEY (category_id, valid_from)
);

CREATE TABLE import_batches (
  id             INTEGER PRIMARY KEY,
  source         TEXT NOT NULL CHECK (source IN ('tbank_csv','ofx','generic_csv','legacy_xlsx','seed_json')),
  file_name      TEXT NOT NULL,
  file_sha256    TEXT NOT NULL,
  period_from    TEXT, period_to TEXT,
  rows_total     INTEGER NOT NULL, rows_imported INTEGER NOT NULL, rows_skipped INTEGER NOT NULL,
  imported_at    TEXT NOT NULL,
  undone_at      TEXT
);

CREATE TABLE transactions (
  id             INTEGER PRIMARY KEY,
  month          TEXT    NOT NULL,
  date           TEXT,
  category_id    INTEGER NOT NULL REFERENCES categories(id),
  title          TEXT    NOT NULL,
  amount         INTEGER NOT NULL CHECK (amount > 0),
  status         TEXT    NOT NULL CHECK (status IN ('paid','debt','unplanned','planned')),
  comment        TEXT,
  source         TEXT    NOT NULL DEFAULT 'manual' CHECK (source IN ('manual','import','legacy','seed')),
  import_batch_id INTEGER REFERENCES import_batches(id),
  fingerprint    TEXT,
  bank_description TEXT, mcc TEXT,
  bank_pending   INTEGER NOT NULL DEFAULT 0,
  sort_key       INTEGER NOT NULL DEFAULT 0,
  created_at     TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT,
  CHECK (date IS NULL OR substr(date,1,7) = month)
);
CREATE INDEX ix_tx_month_cat ON transactions(month, category_id) WHERE deleted_at IS NULL;
CREATE INDEX ix_tx_status    ON transactions(month, status)      WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX ux_tx_fingerprint ON transactions(fingerprint) WHERE fingerprint IS NOT NULL AND deleted_at IS NULL;

CREATE TABLE incomes (
  id             INTEGER PRIMARY KEY,
  month          TEXT    NOT NULL,
  date           TEXT,
  source_name    TEXT    NOT NULL,
  amount         INTEGER NOT NULL CHECK (amount > 0),
  status         TEXT    NOT NULL CHECK (status IN ('received','expected')),
  comment        TEXT,
  source         TEXT NOT NULL DEFAULT 'manual' CHECK (source IN ('manual','import','legacy','seed')),
  import_batch_id INTEGER REFERENCES import_batches(id),
  fingerprint    TEXT,
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT,
  CHECK (date IS NULL OR substr(date,1,7) = month)
);
CREATE INDEX ix_inc_month ON incomes(month) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX ux_inc_fingerprint ON incomes(fingerprint) WHERE fingerprint IS NOT NULL AND deleted_at IS NULL;

CREATE TABLE tags (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
-- COLLATE NOCASE сворачивает только ASCII: уникальность, как у категорий, через norm().
CREATE UNIQUE INDEX ux_tags_name ON tags(norm(name));
CREATE TABLE transaction_tags (
  transaction_id INTEGER NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
  tag_id         INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (transaction_id, tag_id)
);

CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);

CREATE TABLE savings_plan_overrides (month TEXT PRIMARY KEY, rate_bp INTEGER NOT NULL CHECK (rate_bp BETWEEN 0 AND 10000));

CREATE TABLE import_rules (
  id           INTEGER PRIMARY KEY,
  priority     INTEGER NOT NULL DEFAULT 100,
  field        TEXT NOT NULL CHECK (field IN ('description','mcc','bank_category')),
  op           TEXT NOT NULL CHECK (op IN ('contains','equals','starts_with','regex')),
  pattern      TEXT NOT NULL,
  action       TEXT NOT NULL CHECK (action IN ('categorize','ignore','as_income')),
  category_id  INTEGER REFERENCES categories(id),
  status       TEXT CHECK (status IN ('paid','debt','unplanned','planned')),
  title_override TEXT,
  origin       TEXT NOT NULL CHECK (origin IN ('user','learned','system')),
  hits         INTEGER NOT NULL DEFAULT 0,
  created_at   TEXT NOT NULL, updated_at TEXT NOT NULL
);

CREATE TABLE import_profiles (
  id INTEGER PRIMARY KEY, name TEXT NOT NULL, spec TEXT NOT NULL, created_at TEXT NOT NULL
);

CREATE TABLE dashboards (id INTEGER PRIMARY KEY, name TEXT NOT NULL, sort_order INTEGER NOT NULL, is_default INTEGER NOT NULL DEFAULT 0);
CREATE TABLE charts (
  id INTEGER PRIMARY KEY,
  dashboard_id INTEGER NOT NULL REFERENCES dashboards(id) ON DELETE CASCADE,
  spec   TEXT NOT NULL,
  x INTEGER NOT NULL, y INTEGER NOT NULL, w INTEGER NOT NULL, h INTEGER NOT NULL,
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);

CREATE TABLE saved_filters (id INTEGER PRIMARY KEY, name TEXT NOT NULL, query TEXT NOT NULL, created_at TEXT NOT NULL);

-- Представления без удалённых строк.
CREATE VIEW v_transactions AS SELECT * FROM transactions WHERE deleted_at IS NULL;
CREATE VIEW v_incomes      AS SELECT * FROM incomes      WHERE deleted_at IS NULL;

-- Полнотекстовый поиск. Строки хранят норм-текст из norm() (lower, ё→е).
-- rowid строки = id*2 для трат и id*2+1 для доходов: так триггеры меняют строку
-- по rowid, без сканирования всей таблицы по UNINDEXED-колонкам.
CREATE VIRTUAL TABLE search_fts USING fts5(
  kind UNINDEXED, ref_id UNINDEXED, title, extra,
  tokenize = "unicode61 remove_diacritics 2", prefix = '2 3'
);

CREATE VIEW v_tx_search AS
SELECT t.id AS id,
       norm(t.title) AS title,
       norm(c.name
            || ' ' || coalesce((SELECT group_concat(g.name, ' ')
                                  FROM transaction_tags tt JOIN tags g ON g.id = tt.tag_id
                                 WHERE tt.transaction_id = t.id), '')
            || ' ' || coalesce(t.comment, '')
            || ' ' || coalesce(t.bank_description, '')) AS extra
  FROM transactions t JOIN categories c ON c.id = t.category_id
 WHERE t.deleted_at IS NULL;

CREATE TRIGGER tx_fts_ai AFTER INSERT ON transactions BEGIN
  INSERT INTO search_fts(rowid, kind, ref_id, title, extra)
  SELECT id * 2, 'tx', id, title, extra FROM v_tx_search WHERE id = NEW.id;
END;
CREATE TRIGGER tx_fts_au AFTER UPDATE ON transactions BEGIN
  DELETE FROM search_fts WHERE rowid = OLD.id * 2;
  INSERT INTO search_fts(rowid, kind, ref_id, title, extra)
  SELECT id * 2, 'tx', id, title, extra FROM v_tx_search WHERE id = NEW.id;
END;
CREATE TRIGGER tx_fts_ad AFTER DELETE ON transactions BEGIN
  DELETE FROM search_fts WHERE rowid = OLD.id * 2;
END;

CREATE TRIGGER txtag_fts_ai AFTER INSERT ON transaction_tags BEGIN
  DELETE FROM search_fts WHERE rowid = NEW.transaction_id * 2;
  INSERT INTO search_fts(rowid, kind, ref_id, title, extra)
  SELECT id * 2, 'tx', id, title, extra FROM v_tx_search WHERE id = NEW.transaction_id;
END;
CREATE TRIGGER txtag_fts_ad AFTER DELETE ON transaction_tags BEGIN
  DELETE FROM search_fts WHERE rowid = OLD.transaction_id * 2;
  INSERT INTO search_fts(rowid, kind, ref_id, title, extra)
  SELECT id * 2, 'tx', id, title, extra FROM v_tx_search WHERE id = OLD.transaction_id;
END;

CREATE TRIGGER tag_fts_au AFTER UPDATE OF name ON tags BEGIN
  DELETE FROM search_fts
   WHERE rowid IN (SELECT transaction_id * 2 FROM transaction_tags WHERE tag_id = NEW.id);
  INSERT INTO search_fts(rowid, kind, ref_id, title, extra)
  SELECT id * 2, 'tx', id, title, extra FROM v_tx_search
   WHERE id IN (SELECT transaction_id FROM transaction_tags WHERE tag_id = NEW.id);
END;

CREATE TRIGGER category_fts_au AFTER UPDATE OF name ON categories BEGIN
  DELETE FROM search_fts
   WHERE rowid IN (SELECT id * 2 FROM transactions WHERE category_id = NEW.id);
  INSERT INTO search_fts(rowid, kind, ref_id, title, extra)
  SELECT id * 2, 'tx', id, title, extra FROM v_tx_search
   WHERE id IN (SELECT id FROM transactions WHERE category_id = NEW.id);
END;

CREATE TRIGGER inc_fts_ai AFTER INSERT ON incomes BEGIN
  INSERT INTO search_fts(rowid, kind, ref_id, title, extra)
  SELECT id * 2 + 1, 'inc', id, norm(source_name), norm(coalesce(comment, ''))
    FROM incomes WHERE id = NEW.id AND deleted_at IS NULL;
END;
CREATE TRIGGER inc_fts_au AFTER UPDATE ON incomes BEGIN
  DELETE FROM search_fts WHERE rowid = OLD.id * 2 + 1;
  INSERT INTO search_fts(rowid, kind, ref_id, title, extra)
  SELECT id * 2 + 1, 'inc', id, norm(source_name), norm(coalesce(comment, ''))
    FROM incomes WHERE id = NEW.id AND deleted_at IS NULL;
END;
CREATE TRIGGER inc_fts_ad AFTER DELETE ON incomes BEGIN
  DELETE FROM search_fts WHERE rowid = OLD.id * 2 + 1;
END;
