import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import AddRow from './AddRow.svelte';

function mount(options: { keepOpen?: boolean; onsubmit?: () => Promise<boolean> } = {}) {
  const onsubmit = vi.fn(options.onsubmit ?? (() => undefined));
  const oncancel = vi.fn();
  render(AddRow, {
    props: { categoryName: 'Накопления', onsubmit, oncancel, keepOpen: options.keepOpen }
  });
  return { onsubmit, oncancel };
}

it('при открытии фокус в поле суммы, название вторым', () => {
  mount();
  expect(screen.getByLabelText('Сумма, ₽')).toHaveFocus();
  expect(screen.getByLabelText('Наименование, Накопления')).not.toHaveFocus();
});

it('Enter с суммой сохраняет; без keepOpen строку закрывает родитель', async () => {
  const { onsubmit } = mount();
  const amount = screen.getByLabelText('Сумма, ₽');
  await userEvent.type(amount, '10000{Enter}');
  expect(onsubmit).toHaveBeenCalledWith({ title: '', amount: 1_000_000 }, false);
});

it('keepOpen: Enter сохраняет, очищает поля и оставляет фокус на сумме', async () => {
  const { onsubmit } = mount({ keepOpen: true, onsubmit: () => Promise.resolve(true) });
  const title = screen.getByLabelText('Наименование, Накопления');
  const amount = screen.getByLabelText('Сумма, ₽');
  await userEvent.type(title, 'Аванс');
  await userEvent.type(amount, '10000{Enter}');
  expect(onsubmit).toHaveBeenCalledWith({ title: 'Аванс', amount: 1_000_000 }, true);
  await waitFor(() => {
    expect(amount).toHaveValue('');
  });
  expect(title).toHaveValue('');
  expect(amount).toHaveFocus();
});

it('keepOpen: при отказе сохранения введённое остаётся', async () => {
  mount({ keepOpen: true, onsubmit: () => Promise.resolve(false) });
  const amount = screen.getByLabelText('Сумма, ₽');
  await userEvent.type(amount, '10000{Enter}');
  await Promise.resolve();
  expect(amount).toHaveValue('10000');
});

it('Esc закрывает строку', async () => {
  const { oncancel } = mount({ keepOpen: true });
  await fireEvent.keyDown(screen.getByLabelText('Сумма, ₽'), { key: 'Escape' });
  expect(oncancel).toHaveBeenCalled();
});
