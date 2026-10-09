<script lang="ts">
  import { Button, Card, Kbd, StatusDot, Tabs } from '$lib/components';
  import { HELP_EXTRA, INTRO_STEPS, KEY_GROUPS, STATUS_EXPLAINED } from '$lib/help/content';
  import { onboarding } from '$lib/stores/onboarding.svelte';

  const TABS = [
    { value: 'guide', label: 'Как пользоваться' },
    { value: 'keys', label: 'Горячие клавиши' }
  ] as const;

  let tab = $state<'guide' | 'keys'>('guide');
  const sections = [...INTRO_STEPS, ...HELP_EXTRA];
</script>

<svelte:head>
  <title>Planning Budget — справка</title>
</svelte:head>

<div class="screen">
  <Tabs
    tabs={TABS}
    value={tab}
    label="Разделы справки"
    onchange={(value: 'guide' | 'keys') => {
      tab = value;
    }}
  />

  {#if tab === 'guide'}
    <div class="guide">
      {#each sections as section (section.id)}
        <Card title={section.title}>
          {#each section.paragraphs as paragraph (paragraph)}
            <p>{paragraph}</p>
          {/each}
          {#if section.id === 'statuses'}
            <ul class="statuses">
              {#each STATUS_EXPLAINED as item (item.status)}
                <li>
                  <StatusDot status={item.status} /><b>{item.label}</b> <span>{item.text}</span>
                </li>
              {/each}
            </ul>
          {:else if section.id === 'first-month'}
            <div>
              <Button
                variant="secondary"
                onclick={() => {
                  onboarding.openWizard();
                }}>Спланировать месяц</Button
              >
            </div>
          {/if}
        </Card>
      {/each}
    </div>
  {:else}
    <div class="guide">
      {#each KEY_GROUPS as group (group.title)}
        <Card title={group.title}>
          <dl class="keys">
            {#each group.items as item (item.text)}
              <div>
                <dt>
                  {#each item.keys as key, index (index)}<Kbd>{key}</Kbd>{/each}
                </dt>
                <dd>{item.text}</dd>
              </div>
            {/each}
          </dl>
        </Card>
      {/each}
    </div>
  {/if}
</div>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    max-width: var(--content-narrow);
    padding-bottom: var(--sp-6);
  }
  .guide {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }
  p {
    margin: 0;
    color: var(--ink-2);
    font-size: var(--fs-14);
    line-height: var(--lh-body);
  }
  .statuses {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--fs-13);
  }
  .statuses li {
    display: flex;
    align-items: baseline;
    gap: var(--sp-2);
  }
  .statuses span {
    color: var(--muted);
  }
  .keys {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    margin: 0;
  }
  .keys div {
    display: grid;
    grid-template-columns: 200px 1fr;
    align-items: center;
    gap: var(--sp-3);
    font-size: var(--fs-13);
  }
  dt {
    display: flex;
    gap: var(--sp-1);
  }
  dd {
    margin: 0;
    color: var(--ink-2);
  }
</style>
