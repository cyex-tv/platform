<script lang="ts">
  // What an item looks like on the buyer's own name: in the dark and the light chat ground side by
  // side, and as a chat line. `card` shows the first badge and paint; `full` lists every paint and
  // every badge (at chat size and at 2x), as the item dialog does.
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import type { Badge, Paint } from '$lib/cosmetics/store.svelte';
  import { clean } from '$lib/format';
  import Sample from './Sample.svelte';

  type Props = { badges: Badge[]; paints: Paint[]; name: string; full?: boolean };
  let { badges, paints, name, full = false }: Props = $props();

  const badge = $derived(badges[0] ?? null);
  const paint = $derived(paints[0] ?? null);
  const looks = $derived(full ? (paints.length ? paints : [null]) : [paint]);
</script>

<div class="pv" class:full>
  {#each looks as p, i (p?.id ?? 'none')}
    <div class="pair">
      {#if full && p}<span class="cap"><bdi>{clean(p.name)}</bdi></span>{/if}
      <div class="themes">
        <Sample {name} paint={p} {badge} theme="dark" label="{p ? clean(p.name) : 'Your name'} on the dark chat ground" />
        <Sample {name} paint={p} {badge} theme="light" label="{p ? clean(p.name) : 'Your name'} on the light chat ground" />
      </div>
      {#if !full || i === 0}
        <Sample {name} paint={p} {badge} line label="Your name in a chat line" />
      {/if}
    </div>
  {/each}

  {#if full && badges.length}
    <div class="badges">
      <span class="cap">{badges.length === 1 ? 'Badge' : 'Badges'}, chat size and 2x</span>
      <ul>
        {#each badges as b (b.id)}
          <li>
            <span class="px"><BadgeIcon badge={b} size={18} /></span>
            <span class="px"><BadgeIcon badge={b} size={36} label /></span>
            <span class="nm"><bdi>{clean(b.name)}</bdi>{#if b.tooltip && b.tooltip !== b.name}<span class="sub"><bdi>{clean(b.tooltip)}</bdi></span>{/if}</span>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>

<style>
  .pv { display: grid; gap: 10px; min-width: 0; }
  .full { gap: 16px; }
  .pair { display: grid; gap: 6px; min-width: 0; }
  .themes { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 6px; }
  .cap { font-size: var(--t-fine); color: var(--dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .badges { display: grid; gap: 6px; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; }
  li { display: flex; align-items: center; gap: 12px; min-width: 0; }
  .px { display: grid; place-items: center; flex: none; min-width: 36px; height: 40px; }
  .nm { display: grid; min-width: 0; font-size: var(--t-small); font-weight: 600; line-height: 1.3; }
  .nm bdi { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sub { font-size: var(--t-fine); font-weight: 400; color: var(--dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  @media (max-width: 420px) { .themes { grid-template-columns: minmax(0, 1fr); } }
</style>
