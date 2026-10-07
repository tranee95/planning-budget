<script lang="ts">
  import Lock from '@lucide/svelte/icons/lock';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import Logo from '$lib/components/Logo.svelte';
  import { helpItem, sections, settingsItem } from '$lib/nav';
  import { shortcutLabel } from '$lib/shortcuts';

  let { onlock }: { onlock: () => void } = $props();

  const lockKey = shortcutLabel({ code: 'KeyL', mod: true });
  const current = (href: string): 'page' | undefined =>
    page.url.pathname === href ? 'page' : undefined;
</script>

<aside class="sidebar">
  <div class="brand"><Logo size={32} /><span class="text">Planning Budget</span></div>
  <nav aria-label="Разделы">
    {#each sections as item, i (item.href)}
      <a
        href={resolve(item.href)}
        class="nav"
        title={item.label}
        aria-current={current(item.href)}
        aria-keyshortcuts={String(i + 1)}
      >
        <item.icon size={18} strokeWidth={1.75} aria-hidden="true" />
        <span class="text">{item.label}</span>
      </a>
    {/each}
  </nav>
  <a
    href={resolve(helpItem.href)}
    class="nav"
    title={helpItem.label}
    aria-current={current(helpItem.href)}
    aria-keyshortcuts="?"
  >
    <helpItem.icon size={18} strokeWidth={1.75} aria-hidden="true" />
    <span class="text">{helpItem.label}</span>
    <kbd class="text">?</kbd>
  </a>
  <a
    href={resolve(settingsItem.href)}
    class="nav"
    title={settingsItem.label}
    aria-current={current(settingsItem.href)}
  >
    <settingsItem.icon size={18} strokeWidth={1.75} aria-hidden="true" />
    <span class="text">{settingsItem.label}</span>
  </a>
  <button
    type="button"
    class="nav"
    title="Заблокировать"
    aria-keyshortcuts="Control+L"
    onclick={onlock}
  >
    <Lock size={18} strokeWidth={1.75} aria-hidden="true" />
    <span class="text">Заблокировать</span>
    <kbd class="text">{lockKey}</kbd>
  </button>
</aside>

<style>
  .sidebar {
    position: sticky;
    inset-block-start: 0;
    align-self: flex-start;
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
    height: var(--screen-h);
    min-width: 0;
    padding: var(--sp-5) var(--sp-2);
    border-right: 1px solid var(--line);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: 0 var(--sp-3) var(--sp-5);
    font-size: var(--fs-17);
    font-weight: var(--fw-semibold);
  }
  nav {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--sp-1);
  }
  .nav {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    width: 100%;
    height: var(--h-control);
    padding: 0 var(--sp-3);
    border: 0;
    border-radius: var(--r-md);
    background: transparent;
    color: var(--ink-2);
    font: var(--fw-medium) var(--fs-14) var(--font-sans);
    text-decoration: none;
    cursor: pointer;
    transition:
      background-color var(--dur-base) var(--ease-out),
      color var(--dur-base) var(--ease-out);
  }
  .nav:hover {
    background: var(--line-2);
    color: var(--ink);
  }
  .nav[aria-current='page'] {
    background: var(--surface);
    color: var(--ink);
    box-shadow: var(--shadow);
  }
  .nav .text:not(kbd) {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .nav:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  kbd {
    margin-left: auto;
    flex-shrink: 0;
    white-space: nowrap;
    padding: 0 4px;
    border: 1px solid var(--line);
    border-radius: 5px;
    background: var(--surface-2);
    color: var(--muted);
    font: var(--fw-medium) var(--fs-11) var(--font-sans);
    line-height: 20px;
  }
  /* Рейка 64 px: только иконки, подписи в title. */
  @media (max-width: 1199px) {
    .sidebar {
      width: 64px;
      padding-inline: var(--sp-2);
    }
    .brand {
      padding-inline: var(--sp-1);
    }
    .nav {
      justify-content: center;
      padding: 0;
    }
    .text {
      display: none;
    }
  }
</style>
