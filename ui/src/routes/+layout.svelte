<script lang="ts">
  import '@fontsource-variable/onest';
  import '$lib/styles/tokens.css';
  import '$lib/styles/base.css';

  import { onMount } from 'svelte';
  import type { Snippet } from 'svelte';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { startActivityPing } from '$lib/activity';
  import { events } from '$lib/api/bindings';
  import { ApiError } from '$lib/api/call';
  import { errorText } from '$lib/i18n/errors';
  import { prefs } from '$lib/stores/prefs.svelte';
  import { routeFor, session } from '$lib/stores/session.svelte';

  let { children }: { children: Snippet } = $props();

  let bootError = $state<string | null>(null);

  onMount(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    // VaultLocked приходит от бэкенда (автоблокировка, ⌘L, сброс): чистим данные и уходим на /lock.
    void events.vaultLocked
      .listen(() => {
        session.lockedFromBackend();
      })
      .then((off) => {
        if (disposed) off();
        else unlisten = off;
      });
    void (async () => {
      await prefs.load();
      try {
        await session.boot();
      } catch (e) {
        bootError =
          e instanceof ApiError
            ? errorText(e)
            : 'Не удалось связаться с приложением. Закройте и откройте его снова.';
      }
    })();
    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  // Пока сейф открыт, активность сообщается бэкенду на любом экране, включая экран
  // recovery-кода на /lock: иначе автоблокировка сработала бы у работающего пользователя.
  $effect(() => {
    if (session.phase !== 'unlocked') return;
    return startActivityPing();
  });

  $effect(() => {
    const target = routeFor(session.phase, page.url.pathname, session.recoveryCode !== null);
    if (target) void goto(resolve(target), { replaceState: true });
  });
</script>

{#if bootError}
  <p class="boot-error" role="alert">{bootError}</p>
{:else if session.phase !== 'booting'}
  {@render children()}
{/if}

<style>
  .boot-error {
    margin: var(--sp-10);
    color: var(--unpl);
  }
</style>
