<script lang="ts">
  // Picks the badges and paints in a shop item or a code batch: every badge and paint in the
  // catalog with a checkbox, a search, and the count against the limit. A picked row is bold with
  // the accent tick (the checkbox), never a filled box. Paints are drawn in their own paint.
  import Icon from '$lib/components/Icon.svelte';
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import type { Badge, Paint } from '$lib/cosmetics/store.svelte';
  import { clean } from '$lib/format';

  type Props = {
    badges: Badge[];
    paints: Paint[];
    badgeIds: string[];
    paintIds: string[];
    /** The most that can be picked in all (shop items: 10). */
    max?: number;
    error?: string | null;
  };
  let { badges, paints, badgeIds = $bindable(), paintIds = $bindable(), max = 10, error = null }: Props = $props();

  let find = $state('');
  const q = $derived(find.trim().toLowerCase());
  const shownBadges = $derived(q ? badges.filter((b) => b.name.toLowerCase().includes(q)) : badges);
  const shownPaints = $derived(q ? paints.filter((p) => p.name.toLowerCase().includes(q)) : paints);
  const picked = $derived(badgeIds.length + paintIds.length);

  function toggle(list: string[], id: string, on: boolean): string[] {
    return on ? (list.includes(id) ? list : [...list, id]) : list.filter((x) => x !== id);
  }
</script>

<fieldset class="pick" aria-describedby={error ? 'pick-err' : undefined}>
  <legend class="ui-label">Badges and paints <span class="n ui-num">{picked} of {max}</span></legend>
  <label class="find">
    <span class="sr-only">Search badges and paints</span>
    <Icon name="search" size={15} />
    <input class="ui-input" type="search" placeholder="Search" bind:value={find} autocomplete="off" />
  </label>
  <div class="cols">
    <div class="col">
      <h3>Badges</h3>
      <ul>
        {#each shownBadges as b (b.id)}
          {@const on = badgeIds.includes(b.id)}
          <li>
            <label class="opt" class:on>
              <input type="checkbox" checked={on} disabled={!on && picked >= max} onchange={(e) => (badgeIds = toggle(badgeIds, b.id, e.currentTarget.checked))} />
              <BadgeIcon badge={b} size={22} /><span class="lb"><bdi>{clean(b.name)}</bdi></span>
            </label>
          </li>
        {:else}
          <li class="none">{badges.length ? 'No badge matches.' : 'No badges yet.'}</li>
        {/each}
      </ul>
    </div>
    <div class="col">
      <h3>Paints</h3>
      <ul>
        {#each shownPaints as p (p.id)}
          {@const on = paintIds.includes(p.id)}
          <li>
            <label class="opt" class:on>
              <input type="checkbox" checked={on} disabled={!on && picked >= max} onchange={(e) => (paintIds = toggle(paintIds, p.id, e.currentTarget.checked))} aria-label={p.name} />
              <span class="lb" aria-hidden="true"><PaintedName name={p.name} paint={p} /></span>
            </label>
          </li>
        {:else}
          <li class="none">{paints.length ? 'No paint matches.' : 'No paints yet.'}</li>
        {/each}
      </ul>
    </div>
  </div>
  {#if error}<p class="ui-error" id="pick-err">{error}</p>{/if}
</fieldset>

<style>
  .pick { display: grid; gap: 10px; min-width: 0; margin: 0; padding: 0; border: 0; }
  legend { padding: 0; margin-bottom: 8px; }
  .n { margin-left: 8px; letter-spacing: 0; text-transform: none; font-weight: 500; color: var(--muted); }
  .find { position: relative; display: block; }
  .find :global(svg) { position: absolute; left: 10px; top: 50%; translate: 0 -50%; color: var(--muted); pointer-events: none; }
  .find .ui-input { padding-left: 32px; }
  .cols { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
  .col { min-width: 0; }
  h3 { margin: 0 0 4px; font-size: var(--t-fine); font-weight: 600; color: var(--dim); }
  ul { list-style: none; margin: 0; padding: 0; max-height: 220px; overflow-y: auto; border: 1px solid var(--border); border-radius: var(--radius); }
  .opt { display: flex; align-items: center; gap: 10px; min-height: 40px; padding: 4px 10px; cursor: pointer; font-size: var(--t-small); color: var(--muted); }
  .opt input { flex: none; width: 16px; height: 16px; margin: 0; accent-color: var(--accent); }
  .opt.on { color: var(--text); font-weight: 700; }
  .opt:has(input:focus-visible) { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .opt:has(input:disabled) { opacity: .5; cursor: not-allowed; }
  .lb { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .none { padding: 10px; font-size: var(--t-small); color: var(--dim); }
  @media (hover: hover) and (pointer: fine) { .opt:hover { background: var(--row-hover); color: var(--text); } }
  @media (max-width: 639.98px) { .cols { grid-template-columns: minmax(0, 1fr); } }
</style>
