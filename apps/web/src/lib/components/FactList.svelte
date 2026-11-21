<script lang="ts">
  // A facts list for panels and cards: label left, value right, a hairline between rows, tabular
  // numbers (the same look as `<dl class="kv">` inside a PanelBlock). A value with `href` is a link.
  //   <FactList label="Order" items={[{ label: 'Total', value: '4,99 €' }, { label: 'Receipt', value: 'Open', href: '/r/1' }]} />
  type Fact = { label: string; value: string | number; href?: string; mono?: boolean };
  let { items, label }: { items: Fact[]; label?: string } = $props();
</script>

<dl class="facts" aria-label={label}>
  {#each items as f, i (i)}
    <dt>{f.label}</dt>
    <dd class:mono={f.mono}>{#if f.href}<a href={f.href}>{f.value}</a>{:else}{f.value}{/if}</dd>
  {/each}
</dl>

<style>
  .facts { display: grid; grid-template-columns: minmax(0, 1fr) auto; margin: 0; }
  dt, dd { margin: 0; padding: 9px 0; border-bottom: 1px solid var(--border); font-size: var(--t-small); line-height: 1.4; }
  dt { color: var(--muted); padding-right: 16px; }
  dd { min-width: 0; text-align: right; font-weight: 600; font-variant-numeric: tabular-nums; overflow-wrap: anywhere; }
  dd.mono { font-family: var(--font-mono); font-weight: 500; }
  dt:last-of-type, dd:last-of-type { border-bottom: 0; }
  a { color: var(--text); text-decoration: underline; text-decoration-color: var(--border-strong); text-underline-offset: 3px; }
</style>
