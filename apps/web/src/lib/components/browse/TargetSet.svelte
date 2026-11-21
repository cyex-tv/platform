<script lang="ts">
  // The toolbar's target set: which set "Add" (the tile menu and the selection bar) goes to.
  // Signed in only. Starts as your channel's active set; a pick is remembered on this device.
  // A pick list: the chosen set is bold with an accent tick, each row shows how full it is. Sets
  // are grouped by whose they are (yours, then each account you edit for); with more than one
  // group each folds shut, and what is shut stays shut while the page is open.
  //
  // With more than 8 sets a filter field leads the list (by set or channel name). The field has
  // focus on open; ↓ ↑ walk the field, the group heads and the sets, Enter picks (in the field:
  // the first match), Escape clears the field first, then closes.
  import { SvelteSet } from 'svelte/reactivity';
  import Capacity from '$lib/components/Capacity.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Popover from '$lib/components/Popover.svelte';
  import State from '$lib/components/State.svelte';
  import { clean } from '$lib/format';
  import { FILTER_FROM, filterGroups, groupSets } from './set-filter';
  // Not imported as `target`: the test runner's module rewrite would also rename Svelte's `new.target`.
  import { target as sets } from './target.svelte';

  let open = $state(false);
  let query = $state('');
  let pick = $state<HTMLDivElement>();
  let field = $state<HTMLInputElement>();
  $effect(() => {
    sets.ensure();
  });
  // Every opening starts with an empty filter.
  $effect(() => {
    if (!open) query = '';
  });
  // The panel puts focus on the field itself; the phone sheet does too where there is a mouse,
  // but not on a touch screen, where it would throw the keyboard over the list.
  $effect(() => {
    if (!open || !field || document.activeElement === field || !matchMedia('(pointer: fine)').matches) return;
    const f = requestAnimationFrame(() => field?.focus({ preventScroll: true }));
    return () => cancelAnimationFrame(f);
  });
  const set = $derived(sets.current);
  const groups = $derived(sets.sets ? groupSets(sets.sets) : []);
  const filterable = $derived((sets.sets?.length ?? 0) > FILTER_FROM);
  const asking = $derived(filterable && query.trim() !== '');
  const shown = $derived(filterGroups(groups, filterable ? query : ''));
  const heads = $derived(groups.length > 1);
  /** Groups folded shut (by key). Kept for the page view, not only while the picker is open. */
  const shut = new SvelteSet<string>();
  // While filtering, every group with a match is open (a match in a shut group would be lost).
  const unfolded = (key: string) => !heads || asking || !shut.has(key);

  function choose(id: string) {
    sets.choose(id);
    open = false;
  }
  function toggle(key: string) {
    if (shut.has(key)) shut.delete(key);
    else shut.add(key);
  }

  /** What ↓ ↑ walk: the field, then the group heads and sets in view, in order. */
  const stops = () => [...(pick?.querySelectorAll<HTMLElement>('[data-stop]') ?? [])];
  function onkeydown(ev: KeyboardEvent) {
    const el = ev.target as HTMLElement;
    if (ev.key === 'Escape' && query) {
      // First Escape empties the filter; the next one closes (the Popover).
      ev.preventDefault();
      ev.stopPropagation();
      query = '';
      field?.focus();
      return;
    }
    if (ev.key === 'Enter') {
      if (el === field) {
        const first = shown.find((g) => unfolded(g.key))?.sets[0];
        if (first) {
          ev.preventDefault();
          choose(first.id);
        }
      } else if (el instanceof HTMLInputElement && el.type === 'radio') {
        ev.preventDefault();
        choose(el.value);
      }
      return;
    }
    if (ev.key !== 'ArrowDown' && ev.key !== 'ArrowUp') return;
    const list = stops();
    const at = list.indexOf(el);
    if (at < 0) return;
    // Arrows move focus only: a radio is picked with Enter or Space, never by walking past it.
    ev.preventDefault();
    const next = ev.key === 'ArrowDown' ? Math.min(at + 1, list.length - 1) : Math.max(at - 1, 0);
    list[next]?.focus();
  }
</script>

<Popover label="Add to which set" sheet align="end" width="320px" bind:open>
  {#snippet trigger(props)}
    <button class="btn btn-s tgt" type="button" title="Where Add puts emotes" {...props}>
      <Icon name="folder" size={15} />
      <span class="sr-only">Adding to </span><bdi>{set ? clean(set.name) : sets.sets ? 'No set' : 'Your set'}</bdi>
      <Icon name="chevron-down" size={14} />
    </button>
  {/snippet}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="pick" bind:this={pick} {onkeydown}>
    {#if filterable}
      <label class="sr-only" for="target-find">Find a set</label>
      <input
        id="target-find"
        class="ui-input find"
        type="search"
        placeholder="Set or channel name"
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
        enterkeyhint="go"
        data-stop
        bind:this={field}
        bind:value={query}
      />
    {/if}
    <p class="cap-line">Add puts emotes in</p>
    {#if sets.error}
      <p class="msg">{sets.error} <button class="link" type="button" onclick={() => sets.refresh()}>Try again</button></p>
    {:else if !sets.sets}
      <p class="msg">Loading your sets…</p>
    {:else if sets.sets.length === 0}
      <p class="msg">You have no sets to add to yet. <a class="link" href="/me/sets">Make one</a></p>
    {:else if shown.length === 0}
      <State bare title="No sets match">No set or channel is called “<bdi>{clean(query.trim())}</bdi>”.</State>
    {:else}
      <div role="radiogroup" aria-label="Set" class="sets">
        {#each shown as g (g.key)}
          {@const on = unfolded(g.key)}
          {#if heads}
            {#if asking}
              <p class="grp"><span class="gl"><bdi>{g.label}</bdi></span><span class="gn">{g.sets.length}</span></p>
            {:else}
              <button class="grp" type="button" aria-expanded={on} data-stop onclick={() => toggle(g.key)}>
                <span class="chev" class:on aria-hidden="true"><Icon name="chevron-right" size={14} /></span>
                <span class="gl"><bdi>{g.label}</bdi></span><span class="gn">{g.sets.length}</span>
              </button>
            {/if}
          {/if}
          {#if on}
            {#each g.sets as t (t.id)}
              <label class="set" class:on={sets.chosenId === t.id}>
                <input type="radio" name="target-set" value={t.id} data-stop checked={sets.chosenId === t.id} onchange={() => choose(t.id)} />
                <span class="nm"><bdi>{clean(t.name)}</bdi>{#if t.active}<small>On your channel</small>{/if}</span>
                <span class="cap"><Capacity used={t.held.size} capacity={t.capacity} /></span>
                <span class="tick" aria-hidden="true"><Icon name="check" size={14} /></span>
              </label>
            {/each}
          {/if}
        {/each}
      </div>
    {/if}
  </div>
</Popover>

<style>
  .tgt { height: 34px; max-width: 220px; min-width: 0; }
  .tgt bdi { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tgt :global(svg) { flex: none; color: var(--dim); }
  .pick { display: grid; gap: 6px; }
  .find { margin-bottom: 2px; }
  .find::-webkit-search-cancel-button { display: none; }
  .cap-line { margin: 0; padding: 4px 10px 0; font-size: var(--t-label); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .sets { display: grid; gap: 1px; max-height: 320px; overflow-y: auto; }
  /* A group's head: its name and how many sets it holds; a chevron when it folds. */
  .grp { display: flex; align-items: center; gap: 6px; width: 100%; min-height: 32px; margin: 4px 0 0; padding: 0 10px; border: 0; border-radius: var(--radius);
    background: none; color: var(--muted); font: inherit; font-size: 12px; font-weight: 600; text-align: left; }
  button.grp { cursor: pointer; }
  .grp:first-child { margin-top: 0; }
  p.grp { padding-left: 30px; }
  .gl { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .gn { flex: none; color: var(--dim); font-weight: 400; font-variant-numeric: tabular-nums; }
  .chev { display: inline-flex; flex: none; color: var(--dim); transition: transform var(--fast); }
  .chev.on { transform: rotate(90deg); }
  .grp:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .set { position: relative; display: flex; align-items: center; gap: 10px; min-height: 40px; padding: 6px 10px; border-radius: var(--radius);
    font-size: var(--t-nav); color: var(--muted); cursor: pointer; }
  .set input { position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: pointer; }
  .set:has(input:focus-visible) { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .nm { flex: 1; min-width: 0; display: grid; line-height: 1.3; }
  .nm bdi { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .nm small { font-size: 12px; font-weight: 400; color: var(--dim); }
  .cap { font-size: 12px; color: var(--dim); font-variant-numeric: tabular-nums; }
  .tick { display: inline-flex; color: var(--accent-text); opacity: 0; }
  .set.on { color: var(--text); font-weight: 600; }
  .set.on .tick { opacity: 1; }
  .msg { margin: 0; padding: 4px 10px 8px; font-size: var(--t-fine); color: var(--muted); }
  .link { border: 0; background: none; padding: 0; color: var(--accent-text); font: inherit; cursor: pointer; }
  @media (hover: hover) and (pointer: fine) {
    .set:hover { background: var(--row-hover); color: var(--text); }
    button.grp:hover { color: var(--text); }
  }
  @media (pointer: coarse) { .grp { min-height: 44px; } }
  @media (prefers-reduced-motion: reduce) { .chev { transition: none; } }
  @media (max-width: 520px) { .tgt { max-width: 160px; } }
</style>
