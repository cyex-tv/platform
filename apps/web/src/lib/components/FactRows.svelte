<script lang="ts">
  // The inspector's facts (redesign round 2): label left, value right, a hairline under each row,
  // numbers in mono. A row can carry a meter under its value (capacity), a link, an avatar, a
  // tone, or your own markup through the `value` snippet (for rows with a `key`).
  //   <FactRows items={[{ label: 'Emotes', value: '24 / 1.000', mono: true, meter: { used: 24, capacity: 1000 } }, { label: 'Owner', key: 'owner' }]}>
  //     {#snippet value(key)}{#if key === 'owner'}…{/if}{/snippet}
  //   </FactRows>
  import type { Snippet } from 'svelte';
  import Avatar from './Avatar.svelte';
  import Meter from './Meter.svelte';

  type Fact = {
    label: string;
    value?: string | number;
    /** Mono, tabular (counts, dates, ids). On by default for numbers. */
    mono?: boolean;
    href?: string;
    /** An avatar before the value (a person). */
    avatar?: { src?: string | null; name: string };
    meter?: { used: number; capacity: number };
    tone?: 'ok' | 'warn' | 'bad' | 'muted';
    /** Render this row's value through the `value` snippet. */
    key?: string;
  };
  type Props = { items: Fact[]; label?: string; value?: Snippet<[string]> };
  let { items, label, value: custom }: Props = $props();
</script>

<dl class="facts" aria-label={label}>
  {#each items as f, i (i)}
    <div class="fr">
      <dt>{f.label}</dt>
      <dd class="{f.tone ?? ''}" class:mono={f.mono ?? typeof f.value === 'number'}>
        {#if f.key && custom}{@render custom(f.key)}
        {:else}
          {#if f.avatar}<Avatar src={f.avatar.src} name={f.avatar.name} size={20} />{/if}
          {#if f.href}<a href={f.href}><bdi>{f.value}</bdi></a>{:else}<bdi>{f.value}</bdi>{/if}
        {/if}
        {#if f.meter}<span class="m"><Meter used={f.meter.used} capacity={f.meter.capacity} /></span>{/if}
      </dd>
    </div>
  {/each}
</dl>

<style>
  .facts { display: grid; margin: 0; }
  .fr { display: grid; grid-template-columns: auto minmax(0, 1fr); align-items: center; column-gap: 16px; min-height: 40px; padding: 8px 0;
    border-bottom: 1px solid var(--border); }
  .fr:last-child { border-bottom: 0; }
  dt { margin: 0; font-size: var(--t-small); color: var(--muted); white-space: nowrap; }
  dd { display: flex; flex-wrap: wrap; align-items: center; justify-content: flex-end; gap: 6px 8px; min-width: 0; margin: 0; text-align: right;
    font-size: var(--t-small); font-weight: 600; overflow-wrap: anywhere; }
  dd.mono { font-family: var(--font-mono); font-weight: 500; font-variant-numeric: tabular-nums; }
  dd.ok { color: var(--green); }
  dd.warn { color: var(--yellow); }
  dd.bad { color: var(--red); }
  dd.muted { color: var(--muted); font-weight: 500; }
  dd bdi { min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .m { flex-basis: 100%; display: flex; justify-content: flex-end; }
  .m :global(.meter) { width: 120px; }
  a { color: var(--text); text-decoration: none; }
  @media (hover: hover) and (pointer: fine) { a:hover { text-decoration: underline; text-underline-offset: 3px; } }
</style>
