import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { ApiError } from '$lib/api/call';
import { errorText, messages } from './errors';

const ROOT = resolve(__dirname, '../../../..');

function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory())
      return name === 'target' || name === 'tests' ? [] : sources(path);
    return name.endsWith('.rs') ? [readFileSync(path, 'utf8')] : [];
  });
}

/** Ключи, которые бэкенд отдаёт напрямую: литералы `errors.…` и `StorageError::Invalid/Conflict("…")`. */
function backendKeys(): Set<string> {
  const keys = new Set<string>();
  for (const dir of ['crates', 'src-tauri/src']) {
    for (const text of sources(join(ROOT, dir))) {
      for (const m of text.matchAll(/"(errors\.[a-z_.]+)"/g)) keys.add(m[1] ?? '');
      for (const m of text.matchAll(/StorageError::(?:Invalid|Conflict)\("([a-z_.]+)"\)/g)) {
        keys.add(`errors.${m[1] ?? ''}`);
      }
    }
  }
  return keys;
}

// Только в dev-сборке и в тестах: пользователь их не видит.
const INTERNAL = new Set(['errors.dev_only', 'errors.test']);

describe('тексты ошибок', () => {
  it('у каждого ключа бэкенда есть русский текст', () => {
    const missing = [...backendKeys()].filter((k) => !INTERNAL.has(k) && !(k in messages));
    expect(missing).toEqual([]);
  });

  it('запись, которой нет, не называется хранилищем', () => {
    const error = new ApiError({ code: 'NotFound', entity: 'transaction', id: 7 });
    expect(errorText(error)).toBe('Запись не найдена: возможно, её уже удалили.');
    expect(errorText(new ApiError({ code: 'NotFound', entity: 'vault', id: 0 }))).toBe(
      'Хранилище не найдено.'
    );
  });
});
