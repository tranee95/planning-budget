import { token } from './index';

/** Поле «трясётся» один раз: ±4 px, три колебания за `--dur-shake` (300 мс). При reduced-motion не двигается. */
export function shake(node: HTMLElement): Animation | null {
  const ms = token('--dur-shake');
  if (!ms) return null;
  return node.animate(
    [
      { transform: 'translateX(0)' },
      { transform: 'translateX(-4px)' },
      { transform: 'translateX(4px)' },
      { transform: 'translateX(-4px)' },
      { transform: 'translateX(4px)' },
      { transform: 'translateX(-4px)' },
      { transform: 'translateX(4px)' },
      { transform: 'translateX(0)' }
    ],
    { duration: ms, easing: 'ease-in-out' }
  );
}
