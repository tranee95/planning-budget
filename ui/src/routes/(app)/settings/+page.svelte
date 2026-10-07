<script lang="ts">
  import ChangePasswordForm from './_components/ChangePasswordForm.svelte';
  import AppearanceForm from './_components/AppearanceForm.svelte';
  import SavingsForm from './_components/SavingsForm.svelte';
  import RekeyForm from './_components/RekeyForm.svelte';
  import LockForm from './_components/LockForm.svelte';
  import { SettingsVm } from './settings.svelte';

  const vm = new SettingsVm();
</script>

<svelte:head>
  <title>Planning Budget — настройки</title>
</svelte:head>

<section class="card" aria-labelledby="appearance-title">
  <h2 id="appearance-title">Вид</h2>
  <AppearanceForm {vm} />
</section>

<section class="card" aria-labelledby="savings-title">
  <h2 id="savings-title">Сбережения</h2>
  <SavingsForm />
</section>

<section class="card" aria-labelledby="security-title">
  <h2 id="security-title">Безопасность</h2>
  <p class="muted">
    Защита не действует против вредоносных программ на разблокированном компьютере, перехвата
    клавиатуры и принуждения.
  </p>
  <LockForm />
  <ChangePasswordForm {vm} />
  <RekeyForm {vm} />
</section>

<style>
  h2 {
    margin: 0 0 var(--sp-2);
    font-size: var(--fs-17);
    font-weight: var(--fw-semibold);
  }
  .card + .card {
    margin-top: var(--sp-4);
  }
  .card {
    max-width: 520px;
    padding: var(--sp-6);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  .muted {
    margin: 0 0 var(--sp-5);
    color: var(--muted);
    font-size: var(--fs-13);
  }
</style>
