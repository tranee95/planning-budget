-- Долги (займы) и графики погашений.

CREATE TABLE debts (
  id              INTEGER PRIMARY KEY,
  lender          TEXT    NOT NULL,
  amount          INTEGER NOT NULL CHECK (amount > 0),
  taken_month     TEXT    NOT NULL,
  taken_date      TEXT,
  category_id     INTEGER REFERENCES categories(id),
  transaction_id  INTEGER REFERENCES transactions(id),
  comment         TEXT,
  closed_at       TEXT,
  created_at      TEXT    NOT NULL,
  updated_at      TEXT    NOT NULL,
  deleted_at      TEXT,
  CHECK (taken_date IS NULL OR substr(taken_date, 1, 7) = taken_month)
);
CREATE INDEX ix_debts_open ON debts(taken_month) WHERE deleted_at IS NULL;

CREATE TABLE debt_payments (
  id         INTEGER PRIMARY KEY,
  debt_id    INTEGER NOT NULL REFERENCES debts(id) ON DELETE CASCADE,
  month      TEXT    NOT NULL,
  amount     INTEGER NOT NULL CHECK (amount > 0),
  status     TEXT    NOT NULL CHECK (status IN ('planned', 'paid')),
  paid_date  TEXT,
  UNIQUE (debt_id, month)
);
CREATE INDEX ix_debt_pay_month ON debt_payments(month, status);

CREATE VIEW v_debts AS SELECT * FROM debts WHERE deleted_at IS NULL;
