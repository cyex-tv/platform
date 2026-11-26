<script lang="ts">
  // The sticky bar while multi-select is on: "n / 100 selected", Add to the target set, Select
  // all shown, Clear. Adding sends each emote with the same mutation the set editor uses, one
  // after another, with progress and a summary; a full set takes what fits. Over the cap, the
  // page passes a calm note that shows here for a few seconds. Signed out, Add asks you to sign in.
  import { fade, fly } from 'svelte/transition';
  import { ApiError } from '$lib/api';
  import { clean, count } from '$lib/format';
  import { dur } from '$lib/motion';
  import { bottomBar } from '$lib/tool/bottom-bars.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { addState, plan, summary } from './bulk';
  import { CAP } from './selection';
  // Imported as `addTarget`: a binding called `target` breaks Vite's SSR transform of the dev-mode
  // `new.target` check, so the component couldn't be loaded in a test.
  import { target as addTarget } from './target.svelte';

  type Props = {
    ids: readonly string[];
    shown: number;
    signin: string;
    /** A note from the page (the cap), shown in place of the count until it clears. */
    note: string;
    onall: () => void;
    onclear: () => void;
    onexit: () => void;
    /** The ids that went into the set (or were there already), so the page can unpick them; returns the ids left picked. */
    onadded: (ids: string[]) => readonly string[];
  };
  let { ids, shown, signin, note, onall, onclear, onexit, onadded }: Props = $props();

  let run = $state<{ done: number; total: number } | null>(null);
  // The last summary, shown until the selection changes again.
  let result = $state<{ text: string; after: string } | null>(null);
  const said = $derived(result && result.after === ids.join() ? result.text : '');
  const set = $derived(addTarget.current);
  const preview = $derived(set ? plan(ids, set.held, set.capacity - set.held.size) : null);
  const addNow = $derived(addState(ids.length, preview));

  $effect(() => {
    addTarget.ensure();
  });

  async function add() {
    const t = set;
    if (!t || !preview || run) return;
    const p = preview;
    const sent: string[] = [];
    let taken = 0;
    let failed = 0;
    let overflow = p.overflow.length;
    run = { done: 0, total: p.add.length };
    for (const [i, id] of p.add.entries()) {
      try {
        await addTarget.add(id, t);
        sent.push(id);
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
    const text = summary(clean(t.name), { added: sent.length, already: p.already.length, overflow, taken, failed });
    result = { text, after: onadded([...sent, ...p.already]).join() };
  }
</script>

<div class="bar" role="region" aria-label="Selection" use:bottomBar in:fly|global={{ y: 8, duration: dur(200) }} out:fade|global={{ duration: dur(120) }}>
  <p class="n" aria-live="polite">
    {#if run}Adding {count(run.done)} of {count(run.total)}…
    {:else if note}<span class="note">{note}</span>
    {:else if said}{said}
    {:else}<b>{count(ids.length)}</b> / {CAP} selected{/if}
  </p>
  <div class="acts">
    {#if !run && ids.length < Math.min(shown, CAP)}<button class="link" type="button" onclick={onall}>Select all shown</button>{/if}
    {#if ids.length && !run}<button class="link" type="button" onclick={onclear}>Clear</button>{/if}
    <button class="link" type="button" onclick={onexit} disabled={!!run}>Done</button>
    {#if !session.me && session.known}
      <a class="btn btn-s btn-primary" href={signin}>Sign in to add</a>
    {:else if session.me && addTarget.sets && !set}
      <a class="btn btn-s" href="/me/sets">Make a set to add to</a>
    {:else}
      <button class="btn btn-s btn-primary add" type="button" disabled={!ids.length || !!run || !preview || preview.add.length === 0} onclick={add}>
        <span class="lbl">
          {#if addNow === 'already'}Already in <bdi>{clean(set!.name)}</bdi>
          {:else if addNow === 'full'}<bdi>{clean(set!.name)}</bdi> is full
          {:else}Add {ids.length ? count(preview?.add.length ?? ids.length) : ''} to <bdi>{set ? clean(set.name) : 'set'}</bdi>{/if}
        </span>
      </button>
    {/if}
  </div>
</div>

<style>
  .bar { position: sticky; bottom: 12px; z-index: 5; display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between;
    gap: 8px 16px; margin-top: 16px; padding: 8px 8px 8px 16px; border: 1px solid var(--border-strong); border-radius: var(--radius-card);
    background: var(--elevated-2); box-shadow: var(--shadow-pop); }
  .n { margin: 0; font-size: var(--t-nav); color: var(--muted); min-width: 0; font-variant-numeric: tabular-nums; }
  .n b { color: var(--text); font-weight: 700; }
  .note { color: var(--text); }
  .acts { display: flex; flex-wrap: wrap; align-items: center; gap: 4px 12px; min-width: 0; }
  .add { max-width: 100%; min-width: 0; }
  /* Not pressable (nothing picked, full, already in): the quiet disabled look, not the accent. */
  .add:disabled { background: var(--surface); border-color: var(--border-strong); color: var(--muted); opacity: .5; cursor: not-allowed; }
  .add .lbl { min-width: 0; max-width: 28ch; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .link { min-height: 40px; padding: 0 4px; border: 0; background: none; color: var(--accent-text); font: inherit; font-size: var(--t-small); cursor: pointer; }
  .link:disabled { color: var(--dim); cursor: default; }
  @media (max-width: 639.98px) { .bar { bottom: 8px; padding-left: 12px; } .acts { width: 100%; justify-content: flex-end; } }
</style>
