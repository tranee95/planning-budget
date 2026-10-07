-- Параметры облигаций живут в savings_params каждого накопления;
-- ключи настроек bonds.* больше никто не читает.
DELETE FROM settings WHERE key LIKE 'bonds.%';
