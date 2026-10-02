<script lang="ts">
  import type { Theme } from '$lib/api/bindings';
  import Segmented from '$lib/components/Segmented.svelte';
  import Switch from '$lib/components/Switch.svelte';
  import { THEMES, type SettingsVm } from '../settings.svelte';

  let { vm }: { vm: SettingsVm } = $props();
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
    <Switch checked={vm.showTips} onchange={(on: boolean) => void vm.setShowTips(on)}>
      Подсказки для начала работы
    </Switch>
  {/key}
</div>

<style>
  .form {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sp-4);
  }
</style>
