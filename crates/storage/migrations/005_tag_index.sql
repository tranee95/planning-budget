-- Фильтр по тегу (`#тег`) ищет траты по tag_id, а первичный ключ (transaction_id, tag_id)
-- такого поиска не даёт: без этого индекса каждая трата проверялась отдельным запросом.
CREATE INDEX ix_tt_tag ON transaction_tags(tag_id, transaction_id);
