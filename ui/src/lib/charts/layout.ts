/** Сетка дашборда: 12 колонок, высота ячейки 80 px. Чистые функции без DOM. */

export const COLUMNS = 12;
export const ROW_HEIGHT = 80;

export type Box = { id: number; x: number; y: number; w: number; h: number };

/** Размеры S / M / L ширина 4, 6 или 12 колонок. */
export const SIZES = [
  { value: 'S', label: 'S', w: 4 },
  { value: 'M', label: 'M', w: 6 },
  { value: 'L', label: 'L', w: 12 }
] as const;

export type SizeName = (typeof SIZES)[number]['value'];

export function sizeOf(box: Box): SizeName | null {
  return SIZES.find((s) => s.w === box.w)?.value ?? null;
}

const overlaps = (a: Box, b: Box): boolean =>
  a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;

/** Самая высокая позиция карточки в её колонках, не пересекающая уже поставленные. */
function settleUp(box: Box, placed: readonly Box[]): Box {
  let y = 0;
  for (;;) {
    const candidate = { ...box, y };
    const hit = placed.find((p) => overlaps(candidate, p));
    if (hit === undefined) return candidate;
    y = hit.y + hit.h;
  }
}

const reading = (a: Box, b: Box): number => a.y - b.y || a.x - b.x || a.id - b.id;

/** Прижимает все карточки вверх в порядке чтения; пустот между строками не остаётся. */
export function compact(boxes: readonly Box[]): Box[] {
  const placed: Box[] = [];
  for (const box of [...boxes].sort(reading)) placed.push(settleUp(box, placed));
  return placed;
}

const clampBox = (box: Box): Box => {
  const w = Math.min(Math.max(box.w, 1), COLUMNS);
  return { ...box, w, x: Math.min(Math.max(box.x, 0), COLUMNS - w), y: Math.max(box.y, 0) };
};

/**
 * Раскладка во время перетаскивания: карточка `id` стоит ровно там, куда её довели, остальные
 * освобождают ей место и поднимаются вверх.
 */
export function place(boxes: readonly Box[], id: number, to: Partial<Box>): Box[] {
  const moving = boxes.find((b) => b.id === id);
  if (moving === undefined) return [...boxes];
  const pinned = clampBox({ ...moving, ...to, id });
  const placed: Box[] = [pinned];
  for (const box of boxes.filter((b) => b.id !== id).sort(reading)) {
    placed.push(settleUp(box, placed));
  }
  return placed.sort(reading);
}

/** Итоговая раскладка после отпускания: всё прижато вверх, карточка может занять освободившееся место. */
export function settle(boxes: readonly Box[], id: number, to: Partial<Box>): Box[] {
  return compact(place(boxes, id, to));
}

/** Свободная позиция под всеми карточками для новой. */
export function bottom(boxes: readonly Box[]): number {
  return boxes.reduce((max, b) => Math.max(max, b.y + b.h), 0);
}

export function sameLayout(a: readonly Box[], b: readonly Box[]): boolean {
  return (
    a.length === b.length &&
    a.every((box) => {
      const other = b.find((o) => o.id === box.id);
      return (
        other !== undefined &&
        other.x === box.x &&
        other.y === box.y &&
        other.w === box.w &&
        other.h === box.h
      );
    })
  );
}
