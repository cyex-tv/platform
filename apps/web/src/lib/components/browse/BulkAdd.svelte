<script lang="ts">
  // The sticky bar under multi-select: how many are picked, Select all / Clear, and "Add N to a
  // set" (pick one of your sets or one you edit with EMOTES; each emote goes in with the same
  // mutation the set editor uses, one after another, with progress and a summary; a full set
  // takes what fits). Signed out, the button asks you to sign in.
  import { ApiError, gql } from '$lib/api';
  import { session } from '$lib/tool/session.svelte';
  import { grantOn } from '$lib/tool/editors';
  import { ADD_SET_EMOTE, MY_SETS } from '$lib/queries';
  import { clean, count } from '$lib/format';
  import Popover from '../Popover.svelte';
  import Icon from '../Icon.svelte';
  import { plan, summary } from './bulk';

  type Props = {
    ids: string[];
    shown: number;
    signin: string;
    onall: () => void;
    onclear: () => void;
    onexit: () => void;
    /** Called with the ids that went into the set, so the page can unpick them. */
    onadded: (ids: string[]) => void;
  };
  let { ids, shown, signin, onall, onclear, onexit, onadded }: Props = $props();

  type Target = { id: string; name: string; capacity: number; held: Set<string> };
  let targets = $state<Target[] | null>(null);
  let loadError = $state('');
  let chosen = $state('');
  let open = $state(false);
  let run = $state<{ done: number; total: number } | null>(null);
  let result = $state('');

  const target = $derived(targets?.find((t) => t.id === chosen));
  const preview = $derived(target ? plan(ids, target.held, target.capacity - target.held.size) : null);

  async function loadTargets() {
    loadError = '';
    try {
      const r = await gql(MY_SETS, {});
      const me = session.me?.id;
      const writable = [
        ...r.mySets,
        ...r.editorSets.filter((s) => grantOn(r.editorOf, s.owner?.id, me).has('EMOTES')),
      ].filter((s) => s.kind !== 'GLOBAL');
      targets = [...new Map(writable.map((s) => [s.id, s])).values()].map((s) => ({ id: s.id, name: s.name, capacity: s.capacity, held: new Set(s.emotes.map((m) => m.emote.id)) }));
      if (!targets.some((t) => t.id === chosen)) chosen = targets[0]?.id ?? '';
    } catch (e) {
      loadError = e instanceof Error ? e.message : 'Couldn’t load your sets.';
    }
  }
  $effect(() => {
    // Loaded as soon as the bar shows, so the panel opens at its real height (and flips above the bar).
    if (session.me && !targets && !loadError) void loadTargets();
  });

  async function add() {
    const t = target;
    if (!t || !preview || run) return;
    const p = preview;
    const sent: string[] = [];
    let taken = 0;
    let failed = 0;
    let overflow = p.overflow.length;
    result = '';
    run = { done: 0, total: p.add.length };
    open = false;
    for (const [i, id] of p.add.entries()) {
      try {
        await gql(ADD_SET_EMOTE, { setId: t.id, emoteId: id });
        sent.push(id);
        t.held.add(id);
      } catch (e) {
        const code = e instanceof ApiError ? e.code : '';
        if (code === 'name_taken') taken += 1;
        else if (code === 'set_full') overflow += p.add.length - i;
        else failed += 1;
        if (code === 'set_full') break;
      }
      run = { done: i + 1, total: p.add.length };
    }
    run = null;
    result = summary(clean(t.name), { added: sent.length, already: p.already.length, overflow, taken, failed });
    onadded([...sent, ...p.already]);
    void loadTargets();
  }
</script>

<div class="bar" role="region" aria-label="Selection">
  <p class="n" aria-live="polite">
    {#if run}Adding {count(run.done)} of {count(run.total)}…
    {:else if result}{result}
    {:else if ids.length}<b>{count(ids.length)}</b> selected
    {:else}Pick emotes to add them to a set{/if}
  </p>
  <div class="acts">
    {#if !run && ids.length < shown}<button class="link" type="button" onclick={onall}>Select all {count(shown)}</button>{/if}
    {#if ids.length && !run}<button class="link" type="button" onclick={() => { result = ''; onclear(); }}>Clear</button>{/if}
    {#if !session.me && session.known}
      <a class="ui-btn primary small" href={signin}>Sign in to add</a>
    {:else if ids.length}
      <Popover label="Add to a set" sheet align="end" width="320px" bind:open>
        {#snippet trigger(props)}
          <button class="ui-btn primary small" type="button" disabled={!!run || !session.me} {...props}>
            Add {count(ids.length)} to a set <Icon name="chevron-down" size={14} />
          </button>
        {/snippet}
        <div class="pick">
          {#if loadError}
            <p class="msg">{loadError} <button class="link" type="button" onclick={loadTargets}>Try again</button></p>
          {:else if !targets}
            <p class="msg">Loading your sets…</p>
          {:else if targets.length === 0}
            <p class="msg">You have no sets to add to yet. <a class="link" href="/me/sets">Make one</a></p>
          {:else}
            <div role="radiogroup" aria-label="Set" class="sets">
              {#each targets as t (t.id)}
                <label class="set" class:on={chosen === t.id}>
                  <input type="radio" name="bulk-set" value={t.id} bind:group={chosen} />
                  <span class="nm"><bdi>{clean(t.name)}</bdi></span>
                  <span class="cap">{count(t.held.size)} / {count(t.capacity)}</span>
                  <span class="tick" aria-hidden="true"><Icon name="check" size={14} /></span>
                </label>
              {/each}
            </div>
            {#if preview}
              <p class="msg" aria-live="polite">
                {[
                  `${count(preview.add.length)} will be added.`,
                  preview.already.length ? `${count(preview.already.length)} already there.` : '',
                  preview.overflow.length ? `${count(preview.overflow.length)} won’t fit.` : '',
                ]
                  .filter(Boolean)
                  .join(' ')}
              </p>
            {/if}
            <button class="ui-btn primary" type="button" disabled={!preview || preview.add.length === 0} onclick={add}>
              Add {preview ? count(preview.add.length) : ''}
            </button>
          {/if}
        </div>
      </Popover>
    {/if}
    <button class="link" type="button" onclick={onexit} disabled={!!run}>Done</button>
  </div>
</div>

<style>
  .bar { position: sticky; bottom: 12px; z-index: 5; display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between;
    gap: 8px 16px; margin-top: 16px; padding: 8px 8px 8px 16px; border: 1px solid var(--border-strong); border-radius: var(--radius-card);
    background: var(--elevated-2); box-shadow: var(--shadow-pop, 0 8px 24px rgb(0 0 0 / .25)); }
  .n { margin: 0; font-size: var(--t-nav); color: var(--muted); min-width: 0; }
  .n b { color: var(--text); font-weight: 700; font-variant-numeric: tabular-nums; }
  .acts { display: flex; flex-wrap: wrap; align-items: center; gap: 4px 12px; }
  .link { min-height: 40px; padding: 0 4px; border: 0; background: none; color: var(--accent-text); font: inherit; font-size: var(--t-small); cursor: pointer; }
  .link:disabled { color: var(--dim); cursor: default; }
  .pick { display: grid; gap: 12px; padding: 12px; }
  .sets { display: grid; gap: 1px; max-height: 260px; overflow-y: auto; }
  .set { position: relative; display: flex; align-items: center; gap: 10px; min-height: 40px; padding: 6px 10px; border-radius: var(--radius);
    font-size: var(--t-nav); color: var(--muted); cursor: pointer; }
  .set input { position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: pointer; }
  .set:has(input:focus-visible) { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .nm { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cap { font-size: 12px; color: var(--dim); font-variant-numeric: tabular-nums; }
  .tick { display: inline-flex; color: var(--accent-text); opacity: 0; }
  .set.on { color: var(--text); font-weight: 600; }
  .set.on .tick { opacity: 1; }
  .msg { margin: 0; font-size: var(--t-fine); color: var(--muted); }
  @media (hover: hover) and (pointer: fine) { .set:hover { background: var(--row-hover); color: var(--text); } }
  @media (max-width: 520px) { .bar { bottom: 8px; padding-left: 12px; } }
</style>
