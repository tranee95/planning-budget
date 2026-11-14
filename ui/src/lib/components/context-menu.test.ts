import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import ContextMenu from './ContextMenu.svelte';
import type { MenuItem } from './menu';

const items: MenuItem[] = [
  { id: 'edit', label: 'Изменить' },
  { id: 'paid', label: 'Оплачено', checked: true, separator: true },
  { id: 'off', label: 'Недоступно', disabled: true },
  { id: 'delete', label: 'Удалить', danger: true }
];

function open(): { onselect: ReturnType<typeof vi.fn>; onclose: ReturnType<typeof vi.fn> } {
  const onselect = vi.fn();
  const onclose = vi.fn();
  render(ContextMenu, { items, x: 10, y: 10, label: 'Действия', onselect, onclose });
  return { onselect, onclose };
}

describe('ContextMenu', () => {
  it('рисует пункты с ролями и отметкой', () => {
    open();
    expect(screen.getByRole('menu', { name: 'Действия' })).toBeTruthy();
    expect(screen.getByRole('menuitem', { name: 'Изменить' })).toBeTruthy();
    expect(
      screen.getByRole('menuitemradio', { name: /Оплачено/ }).getAttribute('aria-checked')
    ).toBe('true');
  });

  it('выбор пункта сообщает id и закрывает меню', async () => {
    const { onselect, onclose } = open();
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Удалить' }));
    expect(onselect).toHaveBeenCalledWith('delete');
    expect(onclose).toHaveBeenCalled();
  });

  it('недоступный пункт не выбирается', async () => {
    const { onselect } = open();
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Недоступно' }));
    expect(onselect).not.toHaveBeenCalled();
  });

  it('Esc закрывает меню без выбора', async () => {
    const { onselect, onclose } = open();
    await fireEvent.keyDown(screen.getByRole('menu'), { key: 'Escape' });
    expect(onclose).toHaveBeenCalled();
    expect(onselect).not.toHaveBeenCalled();
  });

  it('нажатие вне меню закрывает его', async () => {
    const { onclose } = open();
    await fireEvent.pointerDown(document.body);
    expect(onclose).toHaveBeenCalled();
  });
});
