<script lang="ts">
  // A flat section of a page (redesign round 2, 2026/11/21): no card box. A small uppercase label
  // (with an optional count and actions at its right), an optional line under it, then the
  // content; a hairline above every section after the first. Forms inside read as Rows.
  //   <Section title="Connections" count={2}>{#snippet actions()}…{/snippet}<Row …/></Section>
  import type { Snippet } from 'svelte';

  type Props = {
    title: string;
    /** A number after the label (mono). */
    count?: string | number;
    /** One short line under the label. */
    lead?: string;
    /** The label's heading level (2 on a page whose h1 is the PageHead). */
    level?: 2 | 3;
    id?: string;
    /** Buttons or a link at the label's right. */
    actions?: Snippet;
    /** A danger zone: the label in red. */
    tone?: 'danger';
    children: Snippet;
  };
  let { title, count, lead, level = 2, id, actions, tone, children }: Props = $props();
  const uid = $props.id();
  const hid = $derived(id ?? `${uid}-h`);
</script>

<section class="sec" class:danger={tone === 'danger'} aria-labelledby={hid}>
  <div class="sec-head">
    <svelte:element this={level === 2 ? 'h2' : 'h3'} class="sec-label" id={hid}>{title}{#if count !== undefined}<span class="sec-n">{count}</span>{/if}</svelte:element>
    {#if actions}<div class="sec-acts">{@render actions()}</div>{/if}
  </div>
  {#if lead}<p class="sec-lead">{lead}</p>{/if}
  <div class="sec-body">{@render children()}</div>
</section>

<style>
  .sec { min-width: 0; padding: 20px 0 12px; }
  :global(.sec + .sec) { border-top: 1px solid var(--border); }
  .sec-head { display: flex; align-items: center; gap: 12px; min-height: 32px; }
  .sec-label { display: flex; align-items: baseline; gap: 8px; margin: 0; font-size: var(--t-label, 12px); font-weight: 600; letter-spacing: .06em;
    text-transform: uppercase; color: var(--dim); line-height: 1.2; }
  .danger .sec-label { color: var(--red); }
  .sec-n { font-family: var(--font-mono); font-size: 12px; font-weight: 500; letter-spacing: 0; color: var(--muted); font-variant-numeric: tabular-nums; }
  .sec-acts { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin-left: auto; }
  .sec-lead { margin: 4px 0 0; max-width: 72ch; font-size: var(--t-small); line-height: 1.5; color: var(--muted); text-wrap: pretty; }
  .sec-body { min-width: 0; margin-top: 4px; }
</style>
