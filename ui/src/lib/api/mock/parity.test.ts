import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { expect, test } from 'vitest';
import { handlers } from './handlers';

/** Команды, которые мок намеренно не обслуживает (системные диалоги и файлы, проверяются в нативном окне). */
const NOT_MOCKED = new Set<string>([
  // Нативный диалог выбора файла на стороне Rust.
  'legacy_import',
  // Только debug-сборка Rust: заливка эталонного набора.
  'dev_seed'
]);

test('мок IPC обслуживает каждую команду из bindings.ts и не держит лишних обработчиков', () => {
  const source = readFileSync(resolve(__dirname, '../bindings.ts'), 'utf8');
  const commands = new Set(
    [...source.matchAll(/__TAURI_INVOKE\("([a-z0-9_]+)"/g)].map((m) => m[1] ?? '')
  );
  const mocked = new Set(Object.keys(handlers));
  const missing = [...commands].filter((c) => !mocked.has(c) && !NOT_MOCKED.has(c)).sort();
  const extra = [...mocked].filter((c) => !commands.has(c)).sort();
  expect({ missing, extra }).toEqual({ missing: [], extra: [] });
});
