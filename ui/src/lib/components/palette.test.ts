import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, test, vi } from 'vitest';
import type { SearchResultDto } from '$lib/api/bindings';
import { recentQueries, type SearchFn } from '$lib/palette/search.svelte';
import CommandPalette from './CommandPalette.svelte';

let user: ReturnType<typeof userEvent.setup>;

beforeEach(() => {
  user = userEvent.setup();
  recentQueries.reset();
});

function answer(requestId: number): SearchResultDto {
  return {
    requestId,
    spans: [
      { start: 0, end: 5, kind: 'text', negated: false },
      { start: 6, end: 21, kind: 'status', negated: false }
    ],
    hints: [],
    transactions: {
      total: 2,
      sum: 3_200_000,
      items: [
        {
          id: 1,
          title: 'Лента',
          categoryId: 2,
          category: 'Продукты',
          month: '2026-09',
          date: '2026-09-12',
          amount: 1_320_000,
          status: 'paid'
        }
      ]
    },
    incomes: { total: 0, sum: 0, items: [] },
    categories: [],
    months: []
  };
}

const reply: SearchFn = (_q, id) => Promise.resolve(answer(id));

test('поиск: чипы токенов, группа с итогом, Enter открывает запись и запоминает запрос', async () => {
  const search = vi.fn(reply);
  const onopen = vi.fn();
  const onclose = vi.fn();
  render(CommandPalette, { props: { commands: [], onclose, search, onopen } });

  await user.type(screen.getByRole('combobox'), 'лента статус:оплачено');
  await screen.findByText(/Траты · найдено 2/);
  expect(screen.getByText('текст: «лента»')).toBeInTheDocument();
  expect(screen.getByText('статус:оплачено')).toBeInTheDocument();
  expect(search.mock.lastCall?.[0]).toBe('лента статус:оплачено');

  await user.keyboard('{Enter}');
  expect(onopen).toHaveBeenCalledWith({ type: 'transaction', id: 1, month: '2026-09' });
  expect(onclose).toHaveBeenCalledOnce();
  expect(recentQueries.items).toEqual(['лента статус:оплачено']);
});

test('Tab дополняет ключ фильтра, а без дополнения остаётся обычным Tab', async () => {
  render(CommandPalette, {
    props: { commands: [], onclose: vi.fn(), search: reply }
  });
  const box = screen.getByRole('combobox');
  await user.type(box, 'лента стат');
  await user.tab();
  expect(box).toHaveValue('лента статус:');
  expect(box).toHaveFocus();
});

test('крестик на чипе убирает токен из запроса', async () => {
  render(CommandPalette, {
    props: { commands: [], onclose: vi.fn(), search: reply }
  });
  const box = screen.getByRole('combobox');
  await user.type(box, 'лента статус:оплачено');
  await user.click(await screen.findByRole('button', { name: 'Убрать статус статус:оплачено' }));
  await waitFor(() => {
    expect(box).toHaveValue('лента');
  });
});

test('пустой запрос показывает недавние запросы, выбор подставляет их в поле', async () => {
  recentQueries.remember('сумма>5000');
  render(CommandPalette, {
    props: { commands: [], onclose: vi.fn(), search: reply }
  });
  await user.click(screen.getByRole('option', { name: /сумма>5000/ }));
  expect(screen.getByRole('combobox')).toHaveValue('сумма>5000');
});

test('Enter не открывает запись, пока ответ посчитан для прошлого ввода', async () => {
  const onopen = vi.fn();
  let calls = 0;
  // Первый ответ сразу, следующие — через 400 мс: прежняя выдача остаётся на экране.
  const search: SearchFn = (_q, id) => {
    calls += 1;
    if (calls === 1) return Promise.resolve(answer(id));
    return new Promise((resolve) => {
      setTimeout(() => {
        resolve(answer(id));
      }, 400);
    });
  };
  render(CommandPalette, { props: { commands: [], onclose: vi.fn(), search, onopen } });
  const box = screen.getByRole('combobox');
  await user.type(box, 'лента');
  await screen.findByText(/Траты · найдено 2/);
  await user.type(box, ' м');
  await user.keyboard('{Enter}');
  expect(screen.getByText(/Траты · найдено 2/)).toBeInTheDocument();
  expect(onopen).not.toHaveBeenCalled();
});

test('недавний запрос: id опции без пробелов', () => {
  recentQueries.remember('лента статус:оплачено');
  render(CommandPalette, { props: { commands: [], onclose: vi.fn(), search: reply } });
  const option = screen.getByRole('option', { name: /лента статус:оплачено/ });
  expect(option.id).not.toMatch(/\s/);
});
