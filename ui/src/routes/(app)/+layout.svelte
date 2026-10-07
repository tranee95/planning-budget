<script lang="ts">
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import { searchApi } from '$lib/api/data';
  import { categories } from '$lib/stores/categories.svelte';
  import { page } from '$app/state';
  import CommandPalette from '$lib/components/CommandPalette.svelte';
  import ToastHost from '$lib/components/ToastHost.svelte';
  import { monthOut, monthSlide, routeIn, routeOut } from '$lib/motion';
  import { routeMeta } from '$lib/nav';
  import { shortcuts } from '$lib/shortcuts';
  import { month } from '$lib/stores/month.svelte';
  import { onboarding } from '$lib/stores/onboarding.svelte';
  import { prefs } from '$lib/stores/prefs.svelte';
  import { session } from '$lib/stores/session.svelte';
  import { buildCommands, openTarget } from './_shell/commands';
  import { registerShellShortcuts } from './_shell/shortcuts';
  import Intro from './_intro/Intro.svelte';
  import PlanWizard from './_wizard/PlanWizard.svelte';
  import Sidebar from './_components/Sidebar.svelte';
  import Topbar from './_components/Topbar.svelte';

  let { children }: { children: Snippet } = $props();

  let paletteOpen = $state(false);

  // Страницы без данных месяца (настройки, аналитика с собственными периодами) не перерисовываются при смене месяца.
  const meta = $derived(routeMeta(page.url.pathname));
  const monthScoped = $derived(meta?.monthScoped ?? true);
  const title = $derived(meta ? (meta.title ?? meta.label) : 'Planning Budget');

  const commands = buildCommands();

  onMount(() => {
    const offShortcuts = registerShellShortcuts(() => {
      paletteOpen = !paletteOpen;
    });
    const offCategories = categories.watch();
    return () => {
      offShortcuts();
      offCategories();
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
          {#key page.url.pathname}
            <div class="route" in:routeIn out:routeOut>
              {#key `${monthScoped ? month.current : ''}:${String(onboarding.refreshKey)}`}
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
  {#if prefs.current?.introDone === false && !onboarding.wizardOpen}
    <Intro
      onfinish={() => {
        void prefs.set({ introDone: true });
      }}
      onplan={() => {
        void prefs.set({ introDone: true });
        onboarding.openWizard();
      }}
    />
  {/if}
  {#if onboarding.wizardOpen}
    <PlanWizard />
  {/if}
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
