<script lang="ts">
  // The browse page's set rail (the Workspace browse, signed in): every set you can add to,
  // grouped "Your sets" first, then one group per account you edit for. The set you pick is the
  // target: the tiles' + and the menus' "Add to" put emotes there (bold with an accent icon, no
  // fill). Each row shows the name, LIVE when a channel shows it, a fill meter and used/capacity.
  //   - Groups fold and unfold; what is folded is remembered per account on this device
  //     (rail-state). A big rail starts folded except your sets and the target's group.
  //   - More than 8 sets: a filter field (by set, account or channel name) leads the rail; while
  //     it has text every group with a match is open.
  //   - A row is a drop target: drag a tile (or the picked tiles) onto it to add them there.
  //   - ↓ ↑ walk the field, the group heads and the rows, Home and End jump; Enter or Space picks.
  //   - Right-click a row, or Shift+F10 / the context-menu key on it, for the set menu
  //     (SetActions): rename in place, copy link, set active, capacity, open, delete.
  // The same rail fills the phone's bottom sheet (`sheet`), where picking a set closes it.
  import Icon from '$lib/components/Icon.svelte';
  import InlineRename from '$lib/sets/InlineRename.svelte';
  import SetActions from '$lib/sets/SetActions.svelte';
  import { isMenuKey } from '$lib/sets/set-menu';
  import State from '$lib/components/State.svelte';
  import { clean, count } from '$lib/format';
  import { session } from '$lib/tool/session.svelte';
  import { dropped, endDrag, isOurs } from './drag';
  import { defaultShut, loadShut, saveShut } from './rail-state';
  import { FILTER_FROM, filterGroups, groupSets } from './set-filter';
  // Not imported as `target`: the test runner's module rewrite would also rename Svelte's `new.target`.
  import { target as store, type TargetSet } from './target.svelte';

  type Props = { sheet?: boolean; onpicked?: () => void };
  let { sheet = false, onpicked }: Props = $props();

  $effect(() => {
    store.ensure();
  });

  const uid = $props.id();
  let query = $state('');
  let root = $state<HTMLElement>();
  const me = $derived(session.me?.id ?? '');
  const groups = $derived(store.sets ? groupSets(store.sets) : []);
  const filterable = $derived((store.sets?.length ?? 0) > FILTER_FROM);
  const asking = $derived(filterable && query.trim() !== '');
  const shown = $derived(filterGroups(groups, filterable ? query : ''));
  const targetGroup = $derived.by(() => {
    const c = store.current;
    return c ? (c.mine ? 'mine' : c.ownerId) : null;
  });

  // Folded groups: remembered ones, else the default for this rail's size.
  let shut = $state<Set<string>>(new Set());
  let decidedFor = '';
  $effect(() => {
    if (!me || !store.sets) return;
    if (decidedFor === me) return;
    decidedFor = me;
    shut = loadShut(me) ?? defaultShut(groups, targetGroup);
  });
  const open = (key: string) => asking || !shut.has(key);
  function toggle(key: string) {
    const next = new Set(shut);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    shut = next;
    if (me) saveShut(me, next);
  }

  function pick(id: string) {
    store.choose(id);
    onpicked?.();
  }

  // ---- Drops: a tile (or the picked tiles) dragged onto a set adds them there.
  let over = $state('');
  function dragover(ev: DragEvent, s: TargetSet) {
    if (!isOurs(ev.dataTransfer)) return;
    ev.preventDefault();
    if (ev.dataTransfer) ev.dataTransfer.dropEffect = 'copy';
    over = s.id;
  }
  function drop(ev: DragEvent, s: TargetSet) {
    if (!ev.dataTransfer || !isOurs(ev.dataTransfer)) return;
    ev.preventDefault();
    over = '';
    const items = dropped(ev.dataTransfer);
    endDrag();
    if (items.length) void store.put(s, items);
  }

  // ---- Keys: arrows walk the stops in view.
  const stops = () => [...(root?.querySelectorAll<HTMLElement>('[data-stop]') ?? [])];
  function onkeydown(ev: KeyboardEvent) {
    const el = ev.target as HTMLElement;
    if (ev.key === 'Escape' && query && el.tagName === 'INPUT') {
      ev.preventDefault();
      ev.stopPropagation();
      query = '';
      return;
    }
    if (ev.key === 'Enter' && el.tagName === 'INPUT') {
      const first = shown.find((g) => open(g.key))?.sets[0];
      if (first) {
        ev.preventDefault();
        pick(first.id);
      }
      return;
    }
    if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(ev.key)) return;
    const list = stops();
    const at = list.indexOf(el);
    if (at < 0 || (el.tagName === 'INPUT' && (ev.key === 'Home' || ev.key === 'End'))) return;
    ev.preventDefault();
    const next = ev.key === 'Home' ? 0 : ev.key === 'End' ? list.length - 1 : ev.key === 'ArrowDown' ? Math.min(at + 1, list.length - 1) : Math.max(at - 1, 0);
    list[next]?.focus();
  }

  const pct = (s: TargetSet) => (s.capacity > 0 ? Math.min(100, (s.held.size / s.capacity) * 100) : 0);

  // ---- The set menu, and renaming a row in place.
  let acts = $state<SetActions>();
  let renaming = $state<string | null>(null);
  function openMenu(ev: MouseEvent, s: TargetSet) {
    if (renaming === s.id) return;
    ev.preventDefault();
    acts?.open({ id: s.id, name: s.name, kind: s.kind ?? 'CHANNEL', ownerId: s.ownerId || me || null, capacity: s.capacity }, ev);
  }
  function renamed(id: string, next: string | null) {
    renaming = null;
    if (next) void store.refresh();
    queueMicrotask(() => root?.querySelector<HTMLElement>(`[data-set="${CSS.escape(id)}"]`)?.focus());
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="rail" class:sheet bind:this={root} {onkeydown}>
  <div class="rail-head">
    <h2 class="label" id="{uid}-h">Sets</h2>
    {#if store.sets}<span class="total">{count(store.sets.length)}</span>{/if}
  </div>
  {#if filterable}
    <label class="sr-only" for="{uid}-find">Find a set</label>
    <div class="find">
      <Icon name="search" size={14} />
      <input
        id="{uid}-find"
        type="search"
        placeholder="Set, account or channel"
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
        enterkeyhint="go"
        data-stop
        bind:value={query}
      />
    </div>
  {/if}

  {#if store.error}
    <p class="msg">{store.error} <button class="link" type="button" onclick={() => store.refresh()}>Try again</button></p>
  {:else if !store.sets}
    <div class="loading" aria-busy="true" aria-label="Loading your sets">
      {#each [0, 1, 2] as i (i)}<span class="ph ui-skeleton"></span>{/each}
    </div>
  {:else if store.sets.length === 0}
    <p class="msg">You have no sets to add to yet. <a class="link" href="/me/sets">Make one</a></p>
  {:else if shown.length === 0}
    <State bare title="No sets match">No set or channel is called “<bdi>{clean(query.trim())}</bdi>”.</State>
  {:else}
    <div class="groups" aria-labelledby="{uid}-h">
      {#each shown as g (g.key)}
        {@const on = open(g.key)}
        <section class="group" aria-label={g.label}>
          <button class="grp" type="button" aria-expanded={on} data-stop disabled={asking} onclick={() => toggle(g.key)}>
            <span class="chev" class:on aria-hidden="true"><Icon name="chevron-right" size={14} /></span>
            <span class="gl"><bdi>{g.label}</bdi>{#if g.key !== 'mine'}<span class="role">{' · editor'}</span>{/if}</span>
            <span class="gn">{count(g.sets.length)}</span>
          </button>
          {#if on}
            <ul class="sets">
              {#each g.sets as s (s.id)}
                {@const chosen = store.chosenId === s.id}
                {@const full = s.held.size >= s.capacity}
                <li>
                  {#if renaming === s.id}
                  <div class="set renaming" class:chosen>
                    <span class="ico" aria-hidden="true"><Icon name="pencil" size={14} /></span>
                    <span class="nm"><InlineRename id={s.id} name={s.name} ondone={(n) => renamed(s.id, n)} /></span>
                    <span class="ct" aria-hidden="true">{count(s.held.size)}/{count(s.capacity)}</span>
                    <span class="meter" class:full aria-hidden="true"><i style:width="{pct(s)}%"></i></span>
                  </div>
                  {:else}
                  <button
                    class="set"
                    class:chosen
                    class:over={over === s.id}
                    type="button"
                    data-stop
                    data-set={s.id}
                    aria-pressed={chosen}
                    onclick={() => pick(s.id)}
                    oncontextmenu={(ev) => openMenu(ev, s)}
                    onkeydown={(ev) => {
                      if (!isMenuKey(ev)) return;
                      ev.preventDefault();
                      ev.stopPropagation();
                      acts?.open({ id: s.id, name: s.name, kind: s.kind ?? 'CHANNEL', ownerId: s.ownerId || me || null, capacity: s.capacity }, ev.currentTarget);
                    }}
                    ondragover={(ev) => dragover(ev, s)}
                    ondragleave={() => (over = over === s.id ? '' : over)}
                    ondrop={(ev) => drop(ev, s)}
                  >
                    <span class="ico" aria-hidden="true"><Icon name={chosen ? 'check' : 'folder'} size={14} /></span>
                    <span class="nm"><bdi>{clean(s.name)}</bdi>{#if s.active}<span class="live" title="A channel shows this set">Live</span>{/if}</span>
                    <span class="ct" class:full><span aria-hidden="true">{count(s.held.size)}/{count(s.capacity)}</span><span class="sr-only">, {count(s.held.size)} of {count(s.capacity)} used</span></span>
                    <span class="meter" class:full aria-hidden="true"><i style:width="{pct(s)}%"></i></span>
                  </button>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      {/each}
    </div>
    <p class="hint">The set you pick is where <b>+</b> adds emotes. Drag emotes onto any set to add them there. Right-click a set for more.</p>
  {/if}
</div>
<SetActions bind:this={acts} onrename={(id) => (renaming = id)} onchanged={() => void store.refresh()} />

<style>
  p, h2 { margin: 0; }
  .rail { display: grid; align-content: start; gap: 8px; min-width: 0; }
  .rail-head { display: flex; align-items: center; gap: 8px; min-height: 28px; padding: 0 8px; }
  .label { font-size: var(--t-label); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .total { margin-left: auto; font-size: var(--t-fine); color: var(--dim); font-variant-numeric: tabular-nums; }
  .find { position: relative; display: flex; align-items: center; margin: 0 4px; }
  .find :global(svg) { position: absolute; left: 10px; color: var(--dim); pointer-events: none; }
  .find input { width: 100%; height: 34px; padding: 0 10px 0 30px; border: 1px solid var(--border-strong); border-radius: var(--radius-btn);
    background: var(--bg); color: var(--text); font: inherit; font-size: var(--t-small); }
  .find input::-webkit-search-cancel-button { display: none; }
  .find input:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 1px; }

  .groups { display: grid; gap: 6px; }
  .group { display: grid; gap: 1px; min-width: 0; }
  .grp { display: flex; align-items: center; gap: 6px; width: 100%; min-height: 32px; padding: 0 8px; border: 0; border-radius: var(--radius);
    background: none; color: var(--muted); font: inherit; font-size: var(--t-fine); font-weight: 600; text-align: left; cursor: pointer; }
  .grp:disabled { cursor: default; }
  .grp:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .gl { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .role { font-weight: 400; color: var(--dim); }
  .gn { flex: none; font-weight: 400; color: var(--dim); font-variant-numeric: tabular-nums; }
  .chev { display: inline-flex; flex: none; color: var(--dim); transition: transform var(--fast) var(--ease); }
  .chev.on { transform: rotate(90deg); }

  .sets { list-style: none; margin: 0; padding: 0; display: grid; gap: 1px; }
  /* A row: fixed height, the name and count on one line, the meter under them. */
  .set { position: relative; display: grid; grid-template-columns: 18px minmax(0, 1fr) auto; grid-template-rows: 20px 4px; column-gap: 8px; row-gap: 5px;
    align-items: center; width: 100%; height: 48px; padding: 8px 10px 9px 8px; border: 1px solid transparent; border-radius: var(--radius-btn);
    background: none; color: var(--muted); font: inherit; font-size: var(--t-small); text-align: left; cursor: pointer;
    transition: background-color var(--fast), color var(--fast), border-color var(--fast); }
  .set:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .ico { grid-row: 1; display: inline-flex; color: var(--dim); }
  .nm { grid-row: 1; display: flex; align-items: center; gap: 6px; min-width: 0; }
  .nm bdi { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .live { flex: none; font-size: 10px; font-weight: 700; letter-spacing: .06em; text-transform: uppercase; color: var(--green); }
  .ct { grid-row: 1; font-size: var(--t-fine); color: var(--dim); font-variant-numeric: tabular-nums; }
  .ct.full { color: var(--red); }
  .meter { grid-column: 2 / 4; grid-row: 2; height: 3px; border-radius: 2px; background: var(--border); overflow: hidden; }
  .meter i { display: block; height: 100%; background: var(--accent); border-radius: inherit; transition: width var(--base, .2s) var(--ease); }
  .meter.full i { background: var(--red); }
  /* The target: bold with an accent icon (owner rule: no fill, no bar). */
  .set.chosen { color: var(--text); font-weight: 700; }
  .set.chosen .ico { color: var(--accent-text); }
  /* A drag over a set: the accent hairline says "drop here". */
  .set.over { border-color: var(--accent-border); color: var(--text); }
  .set.renaming { cursor: default; color: var(--text); }

  .msg { padding: 4px 8px; font-size: var(--t-small); color: var(--muted); }
  .link { border: 0; background: none; padding: 0; color: var(--accent-text); font: inherit; cursor: pointer; }
  .loading { display: grid; gap: 6px; padding: 0 4px; }
  .ph { display: block; height: 40px; border-radius: var(--radius-btn); }
  .hint { padding: 8px 8px 0; font-size: var(--t-fine); line-height: 1.45; color: var(--dim); }
  .hint b { color: var(--muted); }

  @media (hover: hover) and (pointer: fine) {
    .set:hover { background: var(--row-hover); color: var(--text); }
    .grp:not(:disabled):hover { color: var(--text); }
  }
  @media (pointer: coarse) { .grp { min-height: 44px; } .set { height: 52px; } }
  .sheet .hint { display: none; }
  @media (forced-colors: active) { .set.chosen { outline: 1px solid Highlight; } .meter i { background: Highlight; } }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .chev, :global(html:not([data-motion='full'])) .meter i { transition: none; } }
</style>
