-- Месяц, в который план уже переносили автоматически (U-1): перенос идёт один раз, и очищенный
-- пользователем месяц не заполняется заново.
CREATE TABLE month_carry (
  month      TEXT PRIMARY KEY,
  carried_at TEXT NOT NULL
);
