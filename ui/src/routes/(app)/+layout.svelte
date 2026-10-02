<script lang="ts">
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import { searchApi } from '$lib/api/data';
  import { categories } from '$lib/stores/categories.svelte';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import CommandPalette from '$lib/components/CommandPalette.svelte';
  import ToastHost from '$lib/components/ToastHost.svelte';
  import { monthOut, monthSlide, routeIn, routeOut } from '$lib/motion';
  import { sections } from '$lib/nav';
  import { themeCommands } from '$lib/palette/commands';
  import type { PaletteTarget } from '$lib/palette/entries';
  import type { PaletteCommand } from '$lib/palette/filter';
  import { shortcuts, shortcutLabel } from '$lib/shortcuts';
  import { undoStack } from '$lib/stores/undo.svelte';
  import { month } from '$lib/stores/month.svelte';
  import { prefs } from '$lib/stores/prefs.svelte';
  import { session } from '$lib/stores/session.svelte';
  import { toasts } from '$lib/stores/toasts.svelte';
  import OnboardingTips from './_components/OnboardingTips.svelte';
  import Sidebar from './_components/Sidebar.svelte';
  import Topbar from './_components/Topbar.svelte';

  let { children }: { children: Snippet } = $props();

  let paletteOpen = $state(false);

  // Страницы без данных месяца (настройки, аналитика с собственными периодами) не перерисовываются при смене месяца.
  const monthScoped = $derived(!['/settings', '/analytics', '/bonds'].includes(page.url.pathname));

  const guarded = (action: () => void | Promise<void>) => (): void => {
    if (session.phase !== 'unlocked') return;
    Promise.resolve(action()).catch(() => {
      toasts.push({ message: 'Не удалось выполнить команду', kind: 'error' });
    });
  };

  const title = $derived(
    page.url.pathname === '/settings'
      ? 'Настройки'
      : page.url.pathname === '/categories'
        ? 'Категории и лимиты'
        : (sections.find((s) => s.href === page.url.pathname)?.label ?? 'Private Budget')
  );

  /** Переход к результату поиска: месяц записи становится выбранным. */
  function openTarget(target: PaletteTarget): void {
    if (target.type === 'transaction') {
      month.set(target.month);
      // Путь разрешён через resolve, запрос дописывается к нему.
      // eslint-disable-next-line svelte/no-navigation-without-resolve
      void goto(`${resolve('/expenses')}?edit=${String(target.id)}`);
    } else if (target.type === 'income') {
      month.set(target.month);
      void goto(resolve('/incomes'));
    } else if (target.type === 'month') {
      month.set(target.month);
      void goto(resolve('/'));
    } else if (target.type === 'table') {
      const path = resolve(target.screen === 'expenses' ? '/expenses' : '/incomes');
      // eslint-disable-next-line svelte/no-navigation-without-resolve
      void goto(`${path}?q=${encodeURIComponent(target.query)}`);
    } else if (target.type === 'category') {
      void goto(resolve('/categories'));
    }
  }

  const commands: PaletteCommand[] = [
    ...sections.map((s, i) => ({
      id: `go-${s.href}`,
      label: s.label,
      group: 'Переход',
      shortcut: String(i + 1),
      run: () => void goto(resolve(s.href))
    })),
    {
      id: 'go-settings',
      label: 'Настройки',
      group: 'Переход',
      run: () => void goto(resolve('/settings'))
    },
    ...themeCommands((theme) => {
      guarded(() => prefs.set({ theme }))();
    }),
    {
      id: 'lock',
      label: 'Заблокировать',
      group: 'Сейф',
      shortcut: shortcutLabel({ code: 'KeyL', mod: true }),
      run: guarded(() => session.lock())
    }
  ];

  onMount(() => {
    const offs = [
      categories.watch(),
      shortcuts.register({
        id: 'palette',
        code: 'KeyK',
        mod: true,
        inInput: true,
        label: 'Поиск и команды',
        run: guarded(() => {
          paletteOpen = !paletteOpen;
        })
      }),
      shortcuts.register({
        id: 'lock',
        code: 'KeyL',
        mod: true,
        inInput: true,
        label: 'Заблокировать',
        run: guarded(() => session.lock())
      }),
      shortcuts.register({
        id: 'month-prev',
        code: 'ArrowLeft',
        label: 'Предыдущий месяц',
        run: () => {
          month.shift(-1);
        }
      }),
      shortcuts.register({
        id: 'month-next',
        code: 'ArrowRight',
        label: 'Следующий месяц',
        run: () => {
          month.shift(1);
        }
      }),
      shortcuts.register({
        id: 'month-today',
        code: 'KeyT',
        label: 'Текущий месяц',
        run: () => {
          month.today();
        }
      }),
      shortcuts.register({
        id: 'history.undo',
        code: 'KeyZ',
        mod: true,
        label: 'Отменить',
        run: () => {
          void undoStack.undo();
        }
      }),
      shortcuts.register({
        id: 'history.redo',
        code: 'KeyZ',
        mod: true,
        shift: true,
        label: 'Повторить',
        run: () => {
          void undoStack.redo();
        }
      }),
      ...sections.map((s, i) =>
        shortcuts.register({
          id: `section-${s.href}`,
          code: `Digit${String(i + 1)}`,
          label: s.label,
          run: () => void goto(resolve(s.href))
        })
      )
    ];
    return () => {
      for (const off of offs) off();
    };
  });
</script>

<svelte:window
  onkeydown={(event: KeyboardEvent) => {
    shortcuts.handle(event);
  }}
/>

{#if session.phase === 'unlocked'}
  <a class="skip" href="#main">Перейти к содержимому</a>
  <div class="shell">
    <Sidebar onlock={() => void session.lock()} />
    <div class="main">
      <div class="page">
        <Topbar
          {title}
          showMonth={monthScoped}
          onsearch={() => {
            paletteOpen = true;
          }}
        />
        <main class="content" id="main" tabindex="-1">
          {#if prefs.current?.showTips}
            <OnboardingTips
              ondone={() => {
                void prefs.set({ showTips: false });
              }}
            />
          {/if}
          {#key page.url.pathname}
            <div class="route" in:routeIn out:routeOut>
              {#key monthScoped ? month.current : ''}
                <div class="month" in:monthSlide={{ direction: month.direction }} out:monthOut>
                  {@render children()}
                </div>
              {/key}
            </div>
          {/key}
        </main>
      </div>
    </div>
  </div>
  {#if paletteOpen}
    <CommandPalette
      {commands}
      search={searchApi.run}
      onopen={openTarget}
      onclose={() => {
        paletteOpen = false;
      }}
    />
  {/if}
  <ToastHost />
{/if}

<style>
  /* Ссылка для клавиатуры: первым Tab-ом ведёт мимо сайдбара к содержимому. */
  .skip {
    position: absolute;
    top: var(--sp-2);
    left: var(--sp-2);
    z-index: 100;
    padding: var(--sp-2) var(--sp-3);
    border-radius: var(--r-md);
    background: var(--accent);
    color: var(--accent-ink);
    font-size: var(--fs-13);
    transform: translateY(-200%);
  }
  .skip:focus-visible {
    transform: none;
  }
  .content:focus {
    outline: none;
  }
  .shell {
    display: flex;
    min-height: var(--screen-h);
  }
  .shell > :global(.sidebar) {
    flex: 0 0 var(--sidebar-w);
  }
  .main {
    flex: 1;
    min-width: 0;
  }
  /* Базовый макет 1440 px: на широких экранах колонка центрируется, а не липнет к краям. */
  .page {
    max-inline-size: var(--content-max);
    margin-inline: auto;
  }
  .content {
    /* Уходящий и приходящий экраны делят одну ячейку: кроссфейд без скачка раскладки. */
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    padding: var(--sp-1) var(--sp-8) var(--sp-8);
    align-content: start;
  }
  .route,
  .month {
    min-width: 0;
  }
  .route {
    grid-area: 2 / 1;
  }
  .month {
    grid-area: 1 / 1;
  }
  .route {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
  }
  @media (max-width: 1199px) {
    .shell > :global(.sidebar) {
      flex-basis: 64px;
    }
  }
</style>
