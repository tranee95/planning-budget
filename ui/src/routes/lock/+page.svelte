<script lang="ts">
  import { LinkButton } from '$lib/components';
  import { autolockLine } from '$lib/autolock';
  import { prefs } from '$lib/stores/prefs.svelte';
  import { session } from '$lib/stores/session.svelte';
  import ForgotForm from './_components/ForgotForm.svelte';
  import LockShell from './_components/LockShell.svelte';
  import RecoveryCodeView from './_components/RecoveryCodeView.svelte';
  import SetupForm from './_components/SetupForm.svelte';
  import UnlockForm from './_components/UnlockForm.svelte';
  import { LockVm } from './lock.svelte';

  const vm = new LockVm();
</script>

<svelte:head>
  <title>Private Budget — вход</title>
</svelte:head>

{#if vm.view === 'recovery' && session.recoveryCode !== null}
  <LockShell
    title="Сохраните ключ восстановления"
    subtitle="Он понадобится, если вы забудете пароль."
    badge={false}
  >
    <RecoveryCodeView code={session.recoveryCode} />
  </LockShell>
{:else if vm.view === 'setup'}
  <LockShell
    title="Создайте пароль"
    subtitle="Данные шифруются и остаются только на этом устройстве."
  >
    <SetupForm {vm} />
  </LockShell>
{:else if vm.view === 'forgot'}
  <LockShell
    title="Восстановление доступа"
    subtitle="Введите ключ восстановления и задайте новый пароль."
    badge={false}
  >
    <ForgotForm {vm} />
  </LockShell>
{:else}
  <LockShell
    title="С возвращением"
    subtitle="Данные зашифрованы и хранятся только на этом устройстве"
  >
    <UnlockForm {vm} />
    {#snippet footer()}
      {#if prefs.current?.autolockMinutes != null}
        <p class="autolock">{autolockLine(prefs.current.autolockMinutes)}</p>
      {/if}
      <div class="forgot">
        <LinkButton
          onclick={() => {
            vm.showForgot();
          }}>Забыли пароль? Ключ восстановления</LinkButton
        >
      </div>
    {/snippet}
  </LockShell>
{/if}

<style>
  .autolock {
    margin: var(--sp-3) 0 0;
    color: var(--faint);
    font-size: var(--fs-12);
    text-align: center;
  }
  .forgot {
    margin-top: var(--sp-2);
  }
</style>
