import { ApiError } from '$lib/api/call';

/** Тексты по `messageKey` из бэкенда. */
export const messages: Record<string, string> = {
  'errors.vault.password_too_short': 'Пароль должен быть не короче 10 символов.',
  'errors.vault.wrong_recovery_code': 'Этот ключ восстановления не подходит к хранилищу.',
  'errors.vault.recovery_code_format':
    'Ключ восстановления введён неверно: 26 символов, группы по 4.',
  'errors.vault.already_exists': 'Хранилище уже создано. Войдите с паролем.',
  'errors.vault.confirm_phrase': 'Фраза подтверждения введена неверно.',
  'errors.vault.orphan_database':
    'В папке данных есть база без файла ключей. Удалите её вручную или верните vault.json.',
  'errors.vault.recovery_save_not_allowed':
    'Сохранить ключ в файл можно только сразу после его создания.',
  'errors.vault.database_missing':
    'Файл базы budget.db не найден. Восстановите его из резервной копии: пустая база взамен не создаётся.',
  'errors.vault.corrupt':
    'Файл ключей vault.json повреждён или создан другой версией приложения. Восстановите его из резервной копии.',
  'errors.rekey.wal_busy':
    'Не удалось подготовить базу к смене ключа: данные ещё в журнале. Ключ не изменён, повторите через несколько секунд.',
  'errors.rekey.journal_mode':
    'Не удалось подготовить базу к смене ключа. Ключ не изменён: закройте и снова откройте приложение и повторите.',
  'errors.vault.rekey_pending':
    'Прошлый перевыпуск ключа не завершён. Заблокируйте приложение и войдите снова, затем повторите.',
  'errors.vault.rekey_interrupted':
    'Перевыпуск ключа был прерван, и этот ключ восстановления больше не подходит. Войдите с паролем.',
  'errors.io.vault': 'Не удалось прочитать или записать файл ключей.',
  'errors.io.migration':
    'Перенос данных из прежней папки снова не удался. Прежние данные не изменены.',
  'errors.io.recovery_file': 'Не удалось записать файл с ключом.',
  'errors.io.delete': 'Не удалось удалить файлы данных.',
  'errors.io.prefs': 'Не удалось сохранить настройки.',
  'errors.prefs.invalid': 'Недопустимое значение настройки.',
  'errors.filter.name_empty': 'Укажите название фильтра.',
  'errors.filter.name_too_long': 'Название фильтра не длиннее 60 символов.',
  'errors.filter.query_invalid': 'Запрос пуст или длиннее 500 символов.',
  'errors.dashboard.name_empty': 'Укажите название дашборда.',
  'errors.dashboard.name_too_long': 'Название дашборда не длиннее 60 символов.',
  'errors.dashboard.exists': 'Дашборд уже есть: стандартный добавляется только в пустой список.',
  'errors.dashboard.last': 'Последний дашборд удалить нельзя.',
  'errors.dashboard.card_size': 'Размер карточки вне сетки: ширина 1–12, высота 1–12.',
  'errors.dashboard.full': 'Дашборд заполнен: удалите ненужные графики или создайте новый дашборд.',
  'errors.dashboard.layout': 'Расположение карточек не помещается в сетку из 12 колонок.',
  'errors.chart.version': 'Неизвестная версия описания графика.',
  'errors.chart.title': 'Название графика — от 1 до 100 символов.',
  'errors.chart.period': 'Начало периода позже конца или период длиннее 20 лет.',
  'errors.chart.filter_too_long': 'Строка фильтра длиннее 500 символов.',
  'errors.chart.filter_source': 'Фильтр по источнику в графиках не поддерживается.',
  'errors.chart.filter_invalid':
    'В фильтре есть нераспознанные условия или свободный текст: графики учитывают только условия вида ключ:значение.',
  'errors.chart.top_n_series': 'Топ-N по сериям недоступен для средних значений.',
  'errors.chart.donut':
    'Кольцевая диаграмма требует разбивку не по месяцам, без серий и без накопления.',
  'errors.chart.kpi': 'Карточка-число считает один показатель без разбивки.',
  'errors.chart.stacked_without_series':
    'Столбцы с накоплением нужны только для разбиения на серии.',
  'errors.chart.needs_month_or_total':
    'Показатель с доходом считается только по месяцам или итогом.',
  'errors.chart.savings_rate_by_month': 'Норма сбережений считается только по месяцам.',
  'errors.chart.limit_usage_by_category': 'Использование лимита считается только по категориям.',
  'errors.chart.limit_usage_series': 'Использование лимита считается без серий.',
  'errors.chart.show_limit': 'Линия лимита есть только у использования лимита.',
  'errors.chart.series_equals_group': 'Серии совпадают с разбивкой.',
  'errors.chart.series_need_transactions':
    'Серии по статусу, типу и категории считаются только по тратам.',
  'errors.chart.metrics_list':
    'Для серий по показателям выберите от 2 до 4 показателей одной единицы.',
  'errors.chart.metrics_without_series':
    'Список показателей задаётся только при сериях по показателям.',
  'errors.chart.cumulative': 'Накопление недоступно для этого показателя или разбивки.',
  'errors.chart.compare_prev': 'Сравнение с прошлым периодом есть только при разбивке по месяцам.',
  'errors.chart.percent': 'Доли недоступны для этого показателя.',
  'errors.chart.top_n': 'Топ-N — число от 1 до 50.',
  'errors.chart.invalid': 'Описание графика не прошло проверку: проверьте поля и повторите.',
  'errors.month_invalid': 'Месяц указан неверно, нужен формат ГГГГ-ММ.',
  'errors.category.color_invalid': 'Цвет категории задан неверно.',
  'errors.category.in_use':
    'На категорию ссылаются записи, правила импорта, долги или снимки плана: удалить её нельзя, только убрать в архив.',
  'errors.category.last_savings':
    'Нельзя убрать последнюю категорию сбережений: в этом году в ней есть записи.',
  'errors.category.name_empty': 'Укажите название категории.',
  'errors.category.name_taken': 'Категория с таким названием уже есть.',
  'errors.category.reorder_mismatch': 'Список категорий изменился. Обновите экран и повторите.',
  'errors.limit.negative': 'Лимит не может быть отрицательным.',
  'errors.limit.savings_has_rate': 'У категории сбережений вместо лимита задаётся процент плана.',
  'errors.record.amount_not_positive': 'Сумма должна быть больше нуля.',
  'errors.record.amount_too_large': 'Сумма не больше 999 999 999,99 ₽.',
  'errors.record.category_archived': 'Категория в архиве: выберите другую.',
  'errors.record.date_invalid': 'Дата указана неверно.',
  'errors.request_id_invalid': 'Не удалось отправить запись: повторите действие.',
  'errors.record.date_month_mismatch': 'Дата должна лежать в выбранном месяце.',
  'errors.record.duplicate': 'Такая запись уже есть: восстановить дубликат нельзя.',
  'errors.record.title_empty': 'Укажите название.',
  'errors.savings.not_a_savings_category': 'Процент плана задаётся только у категорий сбережений.',
  'errors.savings.rate_out_of_range': 'Процент плана — от 0 до 100.',
  'errors.savings.negative_amount': 'Сумма накопления не может быть отрицательной.',
  'errors.debt.lender_empty': 'Укажите, у кого или откуда взят долг.',
  'errors.debt.date_month_mismatch': 'Дата займа должна быть в месяце займа.',
  'errors.debt.transaction_linked': 'Для этой траты долг уже создан.',
  'errors.debt.schedule':
    'График погашения не сходится: платежи должны быть положительными, в разные месяцы, не раньше месяца займа, а их сумма — равна сумме долга.',
  'errors.plan.already_locked': 'План этого месяца уже зафиксирован.',
  'errors.plan.not_locked': 'План этого месяца не зафиксирован.',
  'errors.plan.locked': 'План этого месяца зафиксирован: сначала разблокируйте его.',
  'errors.plan.not_empty': 'В этом месяце уже есть плановые строки: копировать план некуда.',
  'errors.plan.line_is_savings': 'Накопления задаются отдельно, а не строкой расходов.',
  'errors.plan.same_month': 'Выберите другой месяц: нельзя копировать план в тот же месяц.',
  'errors.settings.bad_value': 'Недопустимое значение настройки.',
  'errors.settings.corridor_order':
    'Нижняя граница не больше нормы, норма не больше верхней границы.',
  'errors.settings.unknown_key': 'Неизвестная настройка.',
  'errors.tag.name_empty': 'Укажите название тега.',
  'errors.tag.name_taken': 'Тег с таким названием уже есть.',
  'errors.import.legacy.not_xlsx': 'Файл не открывается как таблица xlsx.',
  'errors.import.legacy.missing_sheet': 'В таблице нет нужного листа.',
  'errors.import.legacy.layout': 'Структура таблицы отличается от ожидаемой.',
  'errors.import.legacy.unknown_status': 'В таблице есть неизвестный статус.',
  'errors.import.legacy.unknown_kind': 'В таблице есть неизвестный тип категории.',
  'errors.import.legacy.number': 'В таблице есть число вне допустимого диапазона.',
  'errors.import.legacy.no_year': 'В таблице не найден заголовок месяца с годом.',
  'errors.legacy.empty': 'В таблице нет данных для переноса.',
  'errors.legacy.not_empty': 'Перенос возможен только в пустое хранилище: в нём уже есть записи.',
  'errors.legacy.unknown_category': 'В таблице есть категория, которой нет в хранилище.',
  'errors.search.limit': 'Слишком большой размер выборки.',
  'errors.tags.ids': 'Не удалось получить теги записей.',
  'errors.seed.not_empty': 'Заливка тестовых данных возможна только в пустое хранилище.'
};

const FALLBACK = 'Не удалось выполнить действие. Повторите попытку.';

/** Человеческий текст ошибки без технических деталей. */
export function errorText(e: unknown): string {
  if (!(e instanceof ApiError)) return FALLBACK;
  const err = e.error;
  switch (err.code) {
    case 'WrongPassword':
      return 'Неверный пароль.';
    case 'TooManyAttempts':
      // Пароль в окне задержки не проверялся: нельзя писать, что он неверный.
      return `Слишком много неверных попыток. Повторите через ${String(Math.ceil(err.retryAfterMs / 1000))} с.`;
    case 'Locked':
      return 'Хранилище заблокировано.';
    case 'Validation':
    case 'Conflict':
    case 'Io':
    case 'Import':
      return messages[err.messageKey] ?? FALLBACK;
    case 'NotFound':
      return err.entity === 'vault'
        ? 'Хранилище не найдено.'
        : 'Запись не найдена: возможно, её уже удалили.';
    case 'Internal':
      return `Внутренняя ошибка. Код для журнала: ${err.correlationId}.`;
  }
}

/** Поле, к которому относится ошибка валидации (если бэкенд его указал). */
export function errorField(e: unknown): string | null {
  return e instanceof ApiError && e.error.code === 'Validation' ? e.error.field : null;
}
