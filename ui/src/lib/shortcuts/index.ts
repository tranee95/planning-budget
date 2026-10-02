export interface Shortcut {
  id: string;
  /** `KeyboardEvent.code`: работает в любой раскладке. */
  code: string;
  /** Ctrl на Windows и Linux, Cmd на macOS. */
  mod?: boolean;
  shift?: boolean;
  label: string;
  /** Срабатывает, даже когда фокус в поле ввода. */
  inInput?: boolean;
  run: () => void;
}

const TEXT_TAGS = new Set(['INPUT', 'TEXTAREA', 'SELECT']);

function inTextField(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return TEXT_TAGS.has(target.tagName) || target.isContentEditable;
}

function dialogOpen(): boolean {
  return document.querySelector('[aria-modal="true"]') !== null;
}

export function matches(shortcut: Shortcut, event: KeyboardEvent): boolean {
  const mod = event.ctrlKey || event.metaKey;
  if (event.code !== shortcut.code) return false;
  if (Boolean(shortcut.mod) !== mod) return false;
  if (Boolean(shortcut.shift) !== event.shiftKey) return false;
  return !event.altKey;
}

export function createShortcuts() {
  const items = new Map<string, Shortcut>();

  return {
    register(shortcut: Shortcut): () => void {
      items.set(shortcut.id, shortcut);
      // Уходящий экран (outro) снимает регистрацию позже, чем новый зарегистрировал ту же клавишу:
      // снимать можно только свою запись.
      return () => {
        if (items.get(shortcut.id) === shortcut) items.delete(shortcut.id);
      };
    },
    handle(event: KeyboardEvent): void {
      if (event.defaultPrevented || event.isComposing) return;
      const typing = inTextField(event.target);
      for (const shortcut of items.values()) {
        if (!matches(shortcut, event)) continue;
        if (typing && !shortcut.inInput) continue;
        if (!shortcut.inInput && dialogOpen()) continue;
        event.preventDefault();
        shortcut.run();
        return;
      }
    },
    list(): Shortcut[] {
      return [...items.values()];
    }
  };
}

/** Общий реестр приложения: оболочка вешает один обработчик на `window`. */
export const shortcuts = createShortcuts();

const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform);

const CODE_LABELS: Record<string, string> = {
  ArrowLeft: '←',
  ArrowRight: '→',
  Escape: 'Esc',
  Slash: '?'
};

/** Подпись клавиши для тултипов и палитры: «Ctrl K», «N». */
export function shortcutLabel(shortcut: Pick<Shortcut, 'code' | 'mod' | 'shift'>): string {
  const base = CODE_LABELS[shortcut.code] ?? shortcut.code.replace(/^Key|^Digit/, '').toUpperCase();
  const parts = [shortcut.mod ? (isMac ? '⌘' : 'Ctrl') : '', shortcut.shift ? '⇧' : '', base];
  return parts.filter(Boolean).join(' ');
}
