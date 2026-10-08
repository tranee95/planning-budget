/** Дольше этого ждать ответа `readText` не стоит: без фокуса окна он может не ответить. */
const READ_TIMEOUT_MS = 1500;

/** Сколько секрет лежит в буфере обмена, прежде чем его стирают. */
export const SECRET_CLIPBOARD_TTL_MS = 30_000;

export type SecretCopy = {
  /** Секрет стёрт из буфера. `false`, если буфер нельзя прочитать или в нём уже другое. */
  readonly cleared: boolean;
  /** Стереть сразу (закрытие экрана, подтверждение) и отменить таймер. */
  clearNow: () => Promise<void>;
};

/** Отмена предыдущего копирования: его таймер и ожидание фокуса не должны стирать новую копию. */
let cancelPrevious: (() => void) | undefined;

/**
 * Копирует секрет и через `ttlMs` стирает его, если он всё ещё в буфере. Чужое содержимое не
 * трогаем: сначала читаем буфер, и если прочесть нельзя, ничего не стираем вслепую.
 */
export async function copySecret(
  secret: string,
  ttlMs: number = SECRET_CLIPBOARD_TTL_MS
): Promise<SecretCopy> {
  cancelPrevious?.();
  await navigator.clipboard.writeText(secret);
  let cleared = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  let waitingForFocus = false;
  const onFocus = (): void => {
    waitingForFocus = false;
    void clearNow();
  };

  const clearNow = async (): Promise<void> => {
    clearTimeout(timer);
    timer = undefined;
    if (cleared) return;
    let readTimer: ReturnType<typeof setTimeout> | undefined;
    try {
      const current = await Promise.race([
        navigator.clipboard.readText(),
        new Promise<never>((_, reject) => {
          readTimer = setTimeout(() => {
            reject(new Error('clipboard read timed out'));
          }, READ_TIMEOUT_MS);
        })
      ]);
      if (current === secret) {
        await navigator.clipboard.writeText('');
        cleared = true;
      }
    } catch {
      // Буфер недоступен для чтения (например, окно без фокуса): стирать вслепую значит затереть
      // чужое. Повторяем один раз, когда окно вернёт фокус.
      if (!waitingForFocus) {
        waitingForFocus = true;
        window.addEventListener('focus', onFocus, { once: true });
      }
    } finally {
      clearTimeout(readTimer);
    }
  };

  timer = setTimeout(() => void clearNow(), ttlMs);
  cancelPrevious = () => {
    clearTimeout(timer);
    window.removeEventListener('focus', onFocus);
  };
  return {
    get cleared() {
      return cleared;
    },
    clearNow: async () => {
      window.removeEventListener('focus', onFocus);
      waitingForFocus = false;
      await clearNow();
    }
  };
}
