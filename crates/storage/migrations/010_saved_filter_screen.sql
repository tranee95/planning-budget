-- Экран сохранённого фильтра: расходы или доходы. Существующие строки
-- получают 'expenses': раньше палитра открывала любой фильтр в «Расходах».
ALTER TABLE saved_filters ADD COLUMN screen TEXT NOT NULL DEFAULT 'expenses' CHECK (screen IN ('expenses', 'incomes'));
