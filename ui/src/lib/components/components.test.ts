import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { createRawSnippet } from 'svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import type { PaletteCommand } from '$lib/palette/filter';
import Combobox from './Combobox.svelte';
import ComboboxInModal from './ComboboxInModal.test-helper.svelte';
import CommandPalette from './CommandPalette.svelte';
import DataTable from './DataTable.svelte';
import Modal from './Modal.svelte';
import MoneyInput from './MoneyInput.svelte';
import Segmented from './Segmented.svelte';
import StatusDot from './StatusDot.svelte';
import type { Column } from './table';

type TestRow = { id: number; name: string };

let user: ReturnType<typeof userEvent.setup>;

beforeEach(() => {
  user = userEvent.setup();
});

test('MoneyInput: «2596,5» превращается в копейки, стрелки меняют сумму на 100 и 1 000 ₽', async () => {
  let value: number | null = null;
  render(MoneyInput, {
    props: {
      id: 'amount',
      label: 'Сумма',
      get value() {
        return value;
      },
      set value(next: number | null) {
        value = next;
      }
    }
  });
  const input = screen.getByLabelText('Сумма');
  await user.type(input, '2596,5');
  expect(value).toBe(259650);

  await user.keyboard('{ArrowUp}');
  expect(value).toBe(269650);
  await user.keyboard('{Shift>}{ArrowDown}{/Shift}');
  expect(value).toBe(169650);

  await user.tab();
  expect(input).toHaveValue('1 696,50');
});

test('MoneyInput: ошибка связана с полем через aria-describedby', () => {
  render(MoneyInput, { props: { id: 'a', label: 'Сумма', error: 'Введите сумму' } });
  const input = screen.getByLabelText('Сумма');
  expect(input).toHaveAttribute('aria-invalid', 'true');
  expect(screen.getByRole('alert')).toHaveTextContent('Введите сумму');
  expect(input).toHaveAttribute('aria-describedby', 'a-error');
});

test('CommandPalette: поиск, стрелки и Enter запускают команду и закрывают палитру', async () => {
  const goSettings = vi.fn();
  const onclose = vi.fn();
  const commands: PaletteCommand[] = [
    { id: 'overview', label: 'Обзор', group: 'Переход', run: vi.fn() },
    { id: 'settings', label: 'Настройки', group: 'Переход', run: goSettings }
  ];
  render(CommandPalette, { props: { commands, onclose } });

  const box = screen.getByRole('combobox');
  expect(box).toHaveFocus();
  expect(screen.getAllByRole('option')).toHaveLength(2);

  await user.keyboard('{ArrowDown}{Enter}');
  expect(goSettings).toHaveBeenCalledOnce();
  expect(onclose).toHaveBeenCalledOnce();
});

test('CommandPalette: пустой результат показывает пояснение', async () => {
  render(CommandPalette, {
    props: {
      commands: [{ id: 'a', label: 'Обзор', group: 'Переход', run: vi.fn() }],
      onclose: vi.fn()
    }
  });
  await user.type(screen.getByRole('combobox'), 'яяя');
  expect(screen.queryAllByRole('option')).toHaveLength(0);
  expect(screen.getByText(/Ничего не найдено. Фильтры: статус:оплачено/)).toBeInTheDocument();
});

test('Modal: Esc закрывает, Tab не выходит за пределы окна', async () => {
  const onclose = vi.fn();
  render(Modal, {
    props: {
      title: 'Заголовок',
      onclose,
      children: createRawSnippet(() => ({
        render: () => '<div><button>Первая</button><button>Вторая</button></div>'
      }))
    }
  });
  const dialog = screen.getByRole('dialog', { name: 'Заголовок' });
  expect(dialog).toHaveAttribute('aria-modal', 'true');

  const [first, second] = screen.getAllByRole('button', { name: /Первая|Вторая/ });
  expect(first).toHaveFocus();
  await user.tab();
  expect(second).toHaveFocus();
  await user.tab();
  expect(first).toHaveFocus();

  await user.keyboard('{Escape}');
  expect(onclose).toHaveBeenCalledOnce();
});

test('Segmented: стрелка вправо выбирает следующий вариант', async () => {
  const onchange = vi.fn();
  render(Segmented, {
    props: {
      label: 'Размер',
      options: [
        { value: 'a', label: 'А' },
        { value: 'b', label: 'Б' }
      ],
      value: 'a',
      onchange
    }
  });
  screen.getByRole('radio', { name: 'А' }).focus();
  await user.keyboard('{ArrowRight}');
  expect(onchange).toHaveBeenCalledWith('b');
});

test('StatusDot: статус доступен по подписи, а не только по цвету', () => {
  render(StatusDot, { props: { status: 'unplanned' } });
  expect(screen.getByRole('img', { name: 'Незапланировано' })).toBeInTheDocument();
});

test('DataTable: рисует только окно из 5 000 строк', () => {
  const columns: Column[] = [{ id: 'name', label: 'Наименование' }];
  const rows = Array.from({ length: 5000 }, (_, id) => ({ id, name: `Строка ${String(id)}` }));
  render(DataTable, {
    props: {
      columns,
      rows,
      rowKey: (row: unknown) => (row as TestRow).id,
      label: 'Траты',
      cell: createRawSnippet<[unknown, Column]>((row) => ({
        render: () => `<span>${(row() as TestRow).name}</span>`
      }))
    }
  });
  const grid = screen.getByRole('grid', { name: 'Траты' });
  expect(grid).toHaveAttribute('aria-rowcount', '5001');
  expect(screen.getAllByRole('row').length).toBeLessThan(40);
});

test('MoneyInput: некорректная сумма остаётся в поле и получает подсказку', async () => {
  render(MoneyInput, { props: { id: 'bad', label: 'Сумма' } });
  const input = screen.getByLabelText('Сумма');
  await user.type(input, '12,345');
  await user.tab();
  expect(input).toHaveValue('12,345');
  expect(input).toHaveAttribute('aria-invalid', 'true');
  expect(screen.getByRole('alert')).toHaveTextContent('не больше двух знаков');
});

test('Combobox: Esc внутри Modal закрывает только список', async () => {
  const onclose = vi.fn();
  render(ComboboxInModal, { props: { onclose } });
  const box = screen.getByRole('combobox');
  await user.click(box);
  expect(screen.getByRole('listbox')).toBeInTheDocument();
  await user.keyboard('{Escape}');
  expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
  expect(onclose).not.toHaveBeenCalled();
  await user.keyboard('{Escape}');
  expect(onclose).toHaveBeenCalledOnce();
});

const fruit = [
  { value: 'a', label: 'Яблоко' },
  { value: 'b', label: 'Банан' }
];

test('Combobox: без onclear пункта «Не выбрано» нет', async () => {
  render(Combobox, {
    props: { id: 'c', label: 'Плод', options: fruit, value: 'a', onchange: vi.fn() }
  });
  await user.click(screen.getByRole('combobox'));
  expect(screen.queryByRole('option', { name: 'Не выбрано' })).not.toBeInTheDocument();
  expect(screen.getAllByRole('option')).toHaveLength(2);
});

test('Combobox: «Не выбрано» первым пунктом, выбор клавиатурой и мышью вызывает onclear', async () => {
  const onchange = vi.fn();
  const onclear = vi.fn();
  render(Combobox, {
    props: { id: 'c', label: 'Плод', options: fruit, value: 'a', onchange, onclear }
  });
  const box = screen.getByRole('combobox');
  await user.click(box);
  const options = screen.getAllByRole('option');
  expect(options.map((o) => o.textContent.trim())).toEqual(['Не выбрано', 'Яблоко', 'Банан']);
  await user.keyboard('{Enter}');
  expect(onclear).toHaveBeenCalledOnce();
  expect(onchange).not.toHaveBeenCalled();

  await user.click(box);
  await user.pointer({
    keys: '[MouseLeft]',
    target: screen.getByRole('option', { name: 'Не выбрано' })
  });
  expect(onclear).toHaveBeenCalledTimes(2);
});

test('Combobox: пункт «Не выбрано» отмечен при value = null и скрыт при поиске', async () => {
  const onclear = vi.fn();
  render(Combobox, {
    props: {
      id: 'c',
      label: 'Плод',
      options: fruit,
      value: null,
      onchange: vi.fn(),
      onclear,
      clearLabel: 'Без плода'
    }
  });
  await user.click(screen.getByRole('combobox'));
  expect(screen.getByRole('option', { name: 'Без плода' })).toHaveAttribute(
    'aria-selected',
    'true'
  );
  await user.keyboard('бан');
  expect(screen.queryByRole('option', { name: 'Без плода' })).not.toBeInTheDocument();
  expect(screen.getByRole('option', { name: 'Банан' })).toBeInTheDocument();
});

test('DataTable: клавиши из кнопки в ячейке не выбирают строку', async () => {
  const onselect = vi.fn();
  const columns: Column[] = [{ id: 'name', label: 'Наименование' }];
  render(DataTable, {
    props: {
      columns,
      rows: [{ id: 1, name: 'Молоко' }],
      rowKey: (row: unknown) => (row as TestRow).id,
      label: 'Траты',
      onselect,
      cell: createRawSnippet<[unknown, Column]>(() => ({
        render: () => '<button>Удалить</button>'
      }))
    }
  });
  screen.getByRole('button', { name: 'Удалить' }).focus();
  await user.keyboard(' ');
  expect(onselect).not.toHaveBeenCalled();
});

test('CommandPalette: Esc закрывает палитру', async () => {
  const onclose = vi.fn();
  render(CommandPalette, {
    props: { commands: [{ id: 'a', label: 'Обзор', group: 'Переход', run: vi.fn() }], onclose }
  });
  await user.keyboard('{Escape}');
  expect(onclose).toHaveBeenCalledOnce();
});

test('CommandPalette: после закрытия и повторного открытия поле пустое и в фокусе', async () => {
  const commands = [{ id: 'a', label: 'Обзор', group: 'Переход', run: vi.fn() }];
  const first = render(CommandPalette, { props: { commands, onclose: vi.fn() } });
  await user.type(screen.getByRole('combobox'), 'темная');
  await user.keyboard('{Escape}');
  first.unmount();

  render(CommandPalette, { props: { commands, onclose: vi.fn() } });
  const box = screen.getByRole('combobox');
  expect(box).toHaveValue('');
  expect(box).toHaveFocus();
});

test('Modal: кнопки действий лежат в закреплённом footer, содержимое — в прокручиваемой области', () => {
  render(Modal, {
    props: {
      title: 'Форма',
      onclose: vi.fn(),
      children: createRawSnippet(() => ({ render: () => '<p>Содержимое</p>' })),
      footer: createRawSnippet(() => ({ render: () => '<button>Сохранить</button>' }))
    }
  });
  const dialog = screen.getByRole('dialog', { name: 'Форма' });
  expect(dialog.querySelector('.content')).toHaveTextContent('Содержимое');
  expect(dialog.querySelector('.footer')).toContainElement(
    screen.getByRole('button', { name: 'Сохранить' })
  );
  expect(dialog.querySelector('.content')).not.toContainElement(
    screen.getByRole('button', { name: 'Сохранить' })
  );
});

test('MoneyInput: сумма больше 999 999 999,99 ₽ отклоняется с понятным сообщением', async () => {
  render(MoneyInput, { props: { id: 'big', label: 'Сумма' } });
  const input = screen.getByLabelText('Сумма');
  await user.type(input, '1000000000');
  await user.tab();
  expect(input).toHaveAttribute('aria-invalid', 'true');
  expect(screen.getByRole('alert')).toHaveTextContent('Сумма не больше 999 999 999,99');
});

test('MoneyInput: стрелка вверх не поднимает сумму выше 999 999 999,99 ₽', async () => {
  let value: number | null = null;
  render(MoneyInput, {
    props: {
      id: 'cap',
      label: 'Сумма',
      get value() {
        return value;
      },
      set value(next: number | null) {
        value = next;
      }
    }
  });
  await user.type(screen.getByLabelText('Сумма'), '999999999,99');
  await user.keyboard('{ArrowUp}');
  expect(value).toBe(99_999_999_999);
  await user.keyboard('{Shift>}{ArrowUp}{/Shift}');
  expect(value).toBe(99_999_999_999);
});

test('Combobox не раскрывает список по фокусу: модалка с ним в начале не всплывает списком', async () => {
  const onclose = vi.fn();
  render(ComboboxInModal, { props: { onclose } });
  const box = screen.getByRole('combobox');
  expect(box).toHaveFocus(); // Modal отдал фокус первому контролу
  expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
  // Esc закрывает модалку сразу, а не только список.
  await user.keyboard('{Escape}');
  expect(onclose).toHaveBeenCalledOnce();
  // Список по-прежнему открывается стрелкой, кликом и набором текста.
  await user.keyboard('{ArrowDown}');
  expect(screen.getByRole('listbox')).toBeInTheDocument();
});

test('DataTable: активная строка передаётся через aria-activedescendant', async () => {
  const columns: Column[] = [{ id: 'name', label: 'Наименование' }];
  render(DataTable, {
    props: {
      columns,
      rows: [
        { id: 1, name: 'Молоко' },
        { id: 2, name: 'Хлеб' }
      ],
      rowKey: (row: unknown) => (row as TestRow).id,
      label: 'Траты',
      cell: createRawSnippet<[unknown, Column]>((row) => ({
        render: () => `<span>${(row() as TestRow).name}</span>`
      }))
    }
  });
  const grid = screen.getByRole('grid', { name: 'Траты' });
  grid.focus();
  await user.keyboard('{ArrowDown}');
  const active = grid.getAttribute('aria-activedescendant');
  expect(active).toBeTruthy();
  expect(document.getElementById(active ?? '')).toHaveTextContent('Хлеб');
});
