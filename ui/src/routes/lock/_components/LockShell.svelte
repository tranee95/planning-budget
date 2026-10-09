<script lang="ts">
  import type { Snippet } from 'svelte';
  import ShieldCheck from '@lucide/svelte/icons/shield-check';
  import Logo from '$lib/components/Logo.svelte';

  type Props = {
    title: string;
    subtitle: string;
    /** Показывать бейдж шифрования под формой. */
    badge?: boolean;
    children: Snippet;
    footer?: Snippet;
  };

  let { title, subtitle, badge = true, children, footer }: Props = $props();
</script>

<main class="screen">
  <div class="glow" aria-hidden="true"></div>
  <div class="panel">
    <Logo />
    <h1>{title}</h1>
    <p class="subtitle">{subtitle}</p>
    {@render children()}
    {#if badge}
      <p class="badge">
        <ShieldCheck size={16} strokeWidth={1.75} aria-hidden="true" />
        <span>SQLCipher · AES-256 · Argon2id</span>
      </p>
    {/if}
    {@render footer?.()}
  </div>
</main>

<style>
  .screen {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: var(--screen-h);
    padding: var(--sp-8) var(--sp-4);
    overflow: hidden;
  }
  .glow {
    position: absolute;
    inset: 0;
    background: radial-gradient(600px 400px at 50% 38%, var(--accent-soft), transparent 70%);
  }
  .panel {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 380px;
    max-width: 100%;
    animation: pop var(--dur-pop) var(--ease-spring) both;
  }
  h1 {
    margin: var(--sp-6) 0 var(--sp-2);
    font-size: var(--fs-26);
    font-weight: var(--fw-bold);
    letter-spacing: var(--tracking-tight);
    text-align: center;
  }
  .subtitle {
    margin: 0 0 var(--sp-6);
    color: var(--muted);
    font-size: var(--fs-14);
    line-height: var(--lh-body);
    text-align: center;
  }
  .badge {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    margin: var(--sp-6) 0 0;
    color: var(--muted);
    font-size: var(--fs-13);
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(12px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
</style>
