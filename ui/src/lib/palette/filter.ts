export interface PaletteCommand {
  id: string;
  label: string;
  group: string;
  /** Дополнительные слова для поиска (латиница, синонимы). */
  keywords?: string;
  /** Подпись клавиши: «Ctrl L», «N». */
  shortcut?: string;
  run: () => void;
}

const norm = (s: string): string => s.toLowerCase().replaceAll('ё', 'е');

/** Подстрока по названию и ключевым словам; совпадение с начала названия идёт выше. */
export function filterCommands(items: readonly PaletteCommand[], query: string): PaletteCommand[] {
  const q = norm(query.trim());
  if (!q) return [...items];
  const ranked: { item: PaletteCommand; rank: number }[] = [];
  for (const item of items) {
    const label = norm(item.label);
    const at = label.indexOf(q);
    if (at >= 0) ranked.push({ item, rank: at === 0 ? 0 : 1 });
    else if (norm(item.keywords ?? '').includes(q)) ranked.push({ item, rank: 2 });
  }
  return ranked.sort((a, b) => a.rank - b.rank).map((r) => r.item);
}
