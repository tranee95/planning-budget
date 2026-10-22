<script lang="ts">
  import { onMount } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import ChangePasswordForm from './_components/ChangePasswordForm.svelte';
  import AppearanceForm from './_components/AppearanceForm.svelte';
  import SavingsForm from './_components/SavingsForm.svelte';
  import RekeyForm from './_components/RekeyForm.svelte';
  import LockForm from './_components/LockForm.svelte';
  import { SettingsVm } from './settings.svelte';

  const vm = new SettingsVm();

  const SECTIONS = [
    { id: 'appearance-title', label: 'Вид' },
    { id: 'savings-title', label: 'Сбережения' },
    { id: 'security-title', label: 'Безопасность' },
    { id: 'danger-title', label: 'Опасные действия' }
  ] as const;

  let current = $state<string>(SECTIONS[0].id);

  // Подсвечивается раздел, который сейчас у верхнего края экрана.
  onMount(() => {
    const visible = new SvelteSet<string>();
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) visible.add(entry.target.id);
          else visible.delete(entry.target.id);
        }
        const first = SECTIONS.find((s) => visible.has(s.id));
        if (first !== undefined) current = first.id;
      },
      { rootMargin: '0px 0px -60% 0px' }
    );
    for (const section of SECTIONS) {
      const el = document.getElementById(section.id);
      if (el !== null) observer.observe(el);
    }
    return () => {
      observer.disconnect();
    };
  });

  function jump(id: string): void {
    document.getElementById(id)?.scrollIntoView({ block: 'start' });
    current = id;
  }
</script>

<svelte:head>
  <title>Planning Budget — настройки</title>
</svelte:head>

<div class="layout">
  <nav class="toc" aria-label="Разделы настроек">
    <ul>
      {#each SECTIONS as section (section.id)}
        <li>
          <button
            type="button"
            class:on={current === section.id}
            aria-current={current === section.id ? 'location' : undefined}
            onclick={() => {
              jump(section.id);
            }}>{section.label}</button
          >
        </li>
      {/each}
    </ul>
  </nav>
  <div class="sections">
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
    </section>

    <section class="card danger" aria-labelledby="danger-title">
      <h2 id="danger-title">Опасные действия</h2>
      <RekeyForm {vm} />
    </section>
  </div>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: 180px minmax(0, var(--content-narrow));
    gap: var(--sp-6);
    align-items: start;
  }
  @media (max-width: 900px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
    }
    .toc {
      display: none;
    }
  }
  .toc {
    position: sticky;
    top: var(--sp-4);
  }
  .toc ul {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .toc button {
    width: 100%;
    min-height: 32px;
    padding-inline: var(--sp-3);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: var(--fs-13);
    text-align: start;
    cursor: pointer;
  }
  .toc button.on {
    background: var(--primary-subtle);
    color: var(--ink);
    font-weight: var(--fw-medium);
  }
  .toc button:focus-visible {
    outline: none;
    box-shadow:
      0 0 0 2px var(--bg),
      0 0 0 4px var(--primary);
  }
  h2 {
    margin: 0 0 var(--sp-2);
    font-size: var(--fs-17);
    font-weight: var(--fw-semibold);
  }
  .card + .card {
    margin-top: var(--sp-4);
  }
  .card {
    max-width: var(--content-narrow);
    padding: var(--sp-5);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  /* «Опасные действия» отличаются от обычных секций: красная рамка и заголовок */
  .card.danger {
    border-color: var(--danger);
  }
  .card.danger h2 {
    color: var(--danger);
  }
  .muted {
    margin: 0 0 var(--sp-5);
    color: var(--muted);
    font-size: var(--fs-13);
  }
</style>
