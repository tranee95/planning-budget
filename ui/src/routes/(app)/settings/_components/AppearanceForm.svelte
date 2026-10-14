<script lang="ts">
  import type { Theme } from '$lib/api/bindings';
  import Button from '$lib/components/Button.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import Switch from '$lib/components/Switch.svelte';
  import { updates } from '$lib/stores/updates.svelte';
  import { THEMES, type SettingsVm } from '../settings.svelte';

  let { vm }: { vm: SettingsVm } = $props();

  const UPDATE_TEXT = {
    upToDate: 'Установлена последняя версия.',
    failed: 'Не удалось проверить обновления.',
    available: 'Доступна новая версия.',
    disabled: '',
    skipped: ''
  } as const;
  const updateText = $derived(updates.status ? UPDATE_TEXT[updates.status.state] : '');
</script>

<div class="form">
  <Segmented
    options={THEMES}
    value={vm.theme}
    label="Тема"
    onchange={(value: Theme) => void vm.setTheme(value)}
  />
  {#key vm.switchKey}
    <Switch checked={vm.reducedMotion} onchange={(on: boolean) => void vm.setReducedMotion(on)}>
      Меньше анимаций
    </Switch>
  {/key}
  {#key vm.switchKey}
    <Switch checked={vm.checkUpdates} onchange={(on: boolean) => void vm.setCheckUpdates(on)}>
      Проверять обновления при запуске
    </Switch>
  {/key}
  <div class="update">
    <Button variant="secondary" loading={updates.checking} onclick={() => void updates.checkNow()}>
      Проверить обновления сейчас
    </Button>
    {#if updateText}<span class="status" role="status" aria-live="polite">{updateText}</span>{/if}
  </div>
  <Button variant="secondary" onclick={() => void vm.showIntroAgain()}>
    Показать знакомство снова
  </Button>
</div>

<style>
  .form {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sp-4);
  }
  .update {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-3);
  }
  .status {
    color: var(--muted);
    font-size: var(--fs-13);
  }
</style>
