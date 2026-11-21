<script lang="ts">
  // One upload in the review queue, up close: "n / total" with previous and next (the arrow
  // keys), its kind, name, uploader, flags and tags, the picture at every size chat uses with its
  // dimensions and weight, and two tabs: Activity (what happened to it, with date and time) and
  // Notes (what reviewers tell each other). Reject and Approve sit at the bottom with the smaller
  // verdicts (approve as unlisted, approve for its owner only) between them. A approves, R
  // rejects, U and P approve unlisted and private, S skips to the next; after a verdict the page
  // opens the next one. Under the facts, "Possible duplicates": other emotes from the very same
  // file, then ones with a close name (emoteDuplicates), each opening in a new tab.
  import { gql } from '$lib/api';
  import { ready, src } from '$lib/emote';
  import { bytes, clean, count, dateTime } from '$lib/format';
  import type { EmoteFieldsFragment, EmoteHistoryQuery, EmoteVisibility } from '$lib/gql/graphql';
  import { ADD_EMOTE_NOTE, DELETE_EMOTE_NOTE, EMOTE_DUPLICATES, EMOTE_HISTORY, EMOTE_NOTES } from '$lib/queries';
  import type { EmoteDuplicatesQuery } from '$lib/gql/graphql';
  import Thumb from '$lib/tool/Thumb.svelte';
  import { STATE_LABEL } from '$lib/tool/state';
  import IconButton from '$lib/components/IconButton.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import { useCopied } from '$lib/clipboard.svelte';
  import AccountLink from './AccountLink.svelte';
  import NoteList from './NoteList.svelte';
  import EmoteFlagsPanel, { canEditFlags } from './EmoteFlagsPanel.svelte';
  import { describe, kindOf, sizes } from './queue';

  type Props = {
    emote: EmoteFieldsFragment | null;
    /** 1-based place in the whole filtered queue, and its size. */
    place: number;
    total: number;
    busy: boolean;
    /** Resolved uploads: nothing to decide, only to read. */
    resolved: boolean;
    hasPrev: boolean;
    hasNext: boolean;
    onprev: () => void;
    onnext: () => void;
    /** S: on to the next upload without a verdict. */
    onskip: () => void;
    onapprove: (visibility?: EmoteVisibility) => void;
    onreject: () => void;
    onclose: () => void;
    /** Staff with emotes.flags saved new flags (ADR 0024): the API's answer. */
    onflags?: (e: EmoteFieldsFragment) => void;
  };
  let { emote, place, total, busy, resolved, hasPrev, hasNext, onprev, onnext, onskip, onapprove, onreject, onclose, onflags }: Props = $props();

  let dialog: HTMLDialogElement;
  let tab = $state<'activity' | 'notes' | 'flags'>('activity');
  // Staff with emotes.flags change a reviewed upload's flags here too (ADR 0024).
  const flagTab = $derived(resolved && canEditFlags());
  $effect(() => {
    if (!flagTab && tab === 'flags') tab = 'activity';
  });
  let noteCount = $state<number | null>(null);
  let history = $state<EmoteHistoryQuery['emoteHistory'] | null>(null);
  let historyProblem = $state(false);
  let seq = 0;
  const idCopy = useCopied();

  export function open() {
    if (!dialog.open) dialog.showModal();
  }
  export function close() {
    if (dialog.open) dialog.close();
  }
  export function isOpen() {
    return dialog?.open ?? false;
  }

  async function loadHistory(id: string) {
    const mine = ++seq;
    history = null;
    historyProblem = false;
    try {
      const r = (await gql(EMOTE_HISTORY, { emoteId: id })).emoteHistory;
      if (mine === seq) history = r;
    } catch {
      if (mine === seq) historyProblem = true;
    }
  }
  $effect(() => {
    const id = emote?.id;
    noteCount = null;
    if (id) {
      loadHistory(id);
      loadDupes(id);
    }
  });

  // Possible duplicates: null while loading (the strip keeps its place), [] for none.
  let dupes = $state<EmoteDuplicatesQuery['emoteDuplicates'] | null>(null);
  let dupesProblem = $state(false);
  let dupeSeq = 0;
  async function loadDupes(id: string) {
    const mine = ++dupeSeq;
    dupes = null;
    dupesProblem = false;
    try {
      const r = (await gql(EMOTE_DUPLICATES, { emoteId: id })).emoteDuplicates;
      if (mine === dupeSeq) dupes = r;
    } catch {
      if (mine === dupeSeq) dupesProblem = true;
    }
  }
  const sameFile = $derived(dupes?.filter((d) => d.sameFile).length ?? 0);

  function onkeydown(ev: KeyboardEvent) {
    if (ev.ctrlKey || ev.metaKey || ev.altKey || ev.defaultPrevented || !emote) return;
    const t = ev.target as HTMLElement | null;
    if (t?.closest('input, textarea, select, [contenteditable]')) return;
    // A dialog opened over this one (the reject reason) takes its own keys.
    if (t?.closest('dialog') !== dialog) return;
    if (ev.key === 'ArrowLeft' && !t?.closest('[role="radiogroup"]')) {
      ev.preventDefault();
      if (hasPrev) onprev();
      return;
    }
    if (ev.key === 'ArrowRight' && !t?.closest('[role="radiogroup"]')) {
      ev.preventDefault();
      if (hasNext) onnext();
      return;
    }
    const k = ev.key.toLowerCase();
    if (k === 's' && !ev.shiftKey && !ev.repeat) {
      ev.preventDefault();
      if (hasNext) onskip();
      return;
    }
    if (resolved || busy || ev.repeat) return;
    if (k === 'a' && ready(emote)) (ev.preventDefault(), onapprove());
    else if (k === 'u' && ready(emote)) (ev.preventDefault(), onapprove('UNLISTED'));
    else if (k === 'p' && ready(emote)) (ev.preventDefault(), onapprove('PRIVATE'));
    else if (k === 'r') (ev.preventDefault(), onreject());
  }

  const files = $derived(emote ? sizes(emote.files) : []);
  const VIS: Record<EmoteVisibility, string> = { PUBLIC: 'Public', UNLISTED: 'Unlisted', PRIVATE: 'Private (owner only)' };
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<dialog class="ui-dialog detail" bind:this={dialog} aria-labelledby="rd-name" {onkeydown} onclose={onclose}>
  {#if emote}
    {@const e = emote}
    <div class="top">
      <IconButton label="Previous" icon="arrow-left" size="s" disabled={!hasPrev} onclick={onprev} />
      <span class="place ui-num" aria-live="polite">{count(place)} / {count(total)}</span>
      <IconButton label="Next" icon="arrow-right" size="s" disabled={!hasNext} onclick={onnext} />
      <span class="grow"></span>
      <IconButton label="Close" icon="x" size="s" onclick={close} />
    </div>

    <div class="main">
      <section class="pics" aria-label="Sizes">
        {#if ready(e) && files.length}
          <div class="sizes">
            {#each files as f (f.name)}
              <figure>
                <span class="frame" style:width="{f.width}px" style:height="{f.height}px">
                  <img src={src(e, f.scale as 1 | 2 | 3 | 4)} width={f.width} height={f.height} alt={f.scale === 4 ? clean(e.name) : ''} decoding="async" />
                </span>
                <figcaption class="ui-mono"><b>{f.scale}x</b><span>{f.width}×{f.height}</span><span>{bytes(f.size)}</span></figcaption>
              </figure>
            {/each}
          </div>
        {:else}
          <State bare kind="loading" title="Files not made yet" />
        {/if}
      </section>

      <div class="side">
        <div class="scroll">
          <section class="facts adm-sec">
            <span class="adm-label">{kindOf(e)}</span>
            <h2 id="rd-name" class="adm-name"><bdi>{clean(e.name)}</bdi></h2>
            <p class="who"><Icon name="user" size={14} /> <AccountLink id={e.owner?.id} name={e.owner?.displayName} /></p>
            <dl class="adm-kv">
              <dt>Uploaded</dt><dd>{dateTime(e.createdAt)}</dd>
              <dt>Visibility</dt><dd>{VIS[e.visibility]}</dd>
              <dt>State</dt><dd><span class="ui-chip {e.state.toLowerCase()}">{e.state === 'TAKEN_DOWN' ? 'Taken down' : e.state.charAt(0) + e.state.slice(1).toLowerCase()}</span></dd>
              {#if e.flags.nsfw || e.flags.zeroWidth || e.flags.wide}
                <dt>Flags</dt>
                <dd class="chips">{#if e.flags.nsfw}<span class="ui-chip warn">NSFW</span>{/if}{#if e.flags.zeroWidth}<span class="ui-chip">Zero-width</span>{/if}{#if e.flags.wide}<span class="ui-chip">Wide</span>{/if}</dd>
              {/if}
              <dt>Tags</dt>
              <dd class="chips">{#each e.tags as t (t)}<span class="ui-chip" title={t}><bdi>{t}</bdi></span>{:else}<span class="ui-hint">None</span>{/each}</dd>
              <dt>ID</dt><dd class="id"><span class="ui-mono">{e.id}</span> <IconButton label={idCopy.copied ? 'Copied' : 'Copy ID'} icon={idCopy.copied ? 'check' : 'copy'} size="s" onclick={() => idCopy.copy(e.id)} /></dd>
            </dl>
          </section>

          <section class="dupes adm-sec" aria-labelledby="rd-dupes">
            <h3 id="rd-dupes" class="adm-label">
              Possible duplicates
              {#if dupes?.length}<span class="n">{count(dupes.length)}</span>{/if}
              {#if sameFile}<span class="ui-chip warn">{sameFile === 1 ? 'Same file as 1 other' : `Same file as ${count(sameFile)} others`}</span>{/if}
            </h3>
            {#if dupesProblem}
              <p class="ui-hint">Couldn’t check for duplicates. <button class="retry" type="button" onclick={() => loadDupes(e.id)}>Try again</button></p>
            {:else if dupes === null}
              <p class="ui-hint" aria-busy="true">Checking…</p>
            {:else if dupes.length === 0}
              <p class="ui-hint">None: no other emote has this file or a close name.</p>
            {:else}
              <ul class="strip">
                {#each dupes as d (d.emote.id)}
                  <li>
                    <a href="/emotes/{encodeURIComponent(d.emote.id)}" target="_blank" rel="noopener" title="{clean(d.emote.name)} by {clean(d.emote.owner?.displayName) || 'unknown'}">
                      <span class="dthumb"><Thumb emote={d.emote} name="" box={40} /></span>
                      <span class="dtext">
                        <span class="dname"><bdi>{clean(d.emote.name)}</bdi></span>
                        <span class="dby"><bdi>{clean(d.emote.owner?.displayName) || 'Unknown'}</bdi></span>
                        <span class="dwhy">
                          {#if d.sameFile}<span class="ui-chip warn">Same file</span>{:else}<span class="ui-chip">Name {Math.round(d.similarity * 100)}%</span>{/if}
                          {#if d.emote.state !== 'APPROVED'}<span class="ui-chip {d.emote.state.toLowerCase()}">{STATE_LABEL[d.emote.state]}</span>{/if}
                        </span>
                      </span>
                    </a>
                  </li>
                {/each}
              </ul>
            {/if}
          </section>

          <section class="adm-sec lower">
            <div class="tabs">
              <Segmented label="Show" size="s" value={tab} onchange={(v) => (tab = v)}
                options={[
                  { value: 'activity', label: history ? `Activity (${history.length + 1})` : 'Activity' },
                  { value: 'notes', label: noteCount === null ? 'Notes' : `Notes (${noteCount})` },
                  ...(flagTab ? [{ value: 'flags' as const, label: 'Emote flags' }] : []),
                ]} />
            </div>
            <div class="pane">
              <div hidden={tab !== 'activity'}>
                {#if historyProblem}
                  <State bare kind="error" title="That didn’t load">
                    {#snippet actions()}<button class="ui-btn secondary small" type="button" onclick={() => loadHistory(e.id)}>Try again</button>{/snippet}
                  </State>
                {:else if !history}
                  <State bare kind="loading" title="Loading activity…" />
                {:else}
                  <ol class="ui-list log">
                    {#each history as h (h.id)}
                      {@const d = describe(h)}
                      <li>
                        <span class="line">
                          <span>{d.text} · <AccountLink id={h.actorId} /></span>
                          {#if d.detail}<span class="more">{d.detail}</span>{/if}
                        </span>
                        <time class="ui-mono" datetime={h.createdAt}>{dateTime(h.createdAt)}</time>
                      </li>
                    {/each}
                    <li>
                      <span class="line"><span>Uploaded · <AccountLink id={e.owner?.id} name={e.owner?.displayName} /></span></span>
                      <time class="ui-mono" datetime={e.createdAt}>{dateTime(e.createdAt)}</time>
                    </li>
                  </ol>
                {/if}
              </div>
              <div hidden={tab !== 'notes'}>
                <NoteList
                  key={e.id}
                  load={async () => (await gql(EMOTE_NOTES, { emoteId: e.id })).emoteNotes}
                  add={async (body) => (await gql(ADD_EMOTE_NOTE, { emoteId: e.id, body })).addEmoteNote}
                  remove={(id) => gql(DELETE_EMOTE_NOTE, { id })}
                  placeholder="A note for other reviewers: why you held it, what to compare it with."
                  oncount={(n) => (noteCount = n)}
                />
              </div>
              {#if flagTab}
                <div hidden={tab !== 'flags'}>
                  <EmoteFlagsPanel bare emote={e} id="rd-flags" onsaved={(next) => (onflags?.(next), loadHistory(e.id))} />
                </div>
              {/if}
            </div>
          </section>
        </div>

        {#if !resolved}
          <footer>
            <button class="ui-btn danger" type="button" disabled={busy} onclick={onreject}><Icon name="x" size={15} />Reject</button>
            <span class="small">
              <IconButton label="Skip to the next, no verdict" icon="arrow-right" variant="outline" tip="top" disabled={!hasNext} onclick={onskip} />
              <IconButton label="Approve as unlisted" icon="eye-off" variant="outline" tip="top" disabled={busy || !ready(e)} onclick={() => onapprove('UNLISTED')} />
              <IconButton label="Approve for its owner only" icon="user" variant="outline" tip="top" disabled={busy || !ready(e)} onclick={() => onapprove('PRIVATE')} />
              <IconButton label="Open the uploader’s account" icon="circle-user" variant="outline" tip="top" href={e.owner ? `/admin/users/${encodeURIComponent(e.owner.id)}` : undefined} disabled={!e.owner} />
            </span>
            <button class="ui-btn primary" type="button" disabled={busy || !ready(e)} onclick={() => onapprove()}><Icon name="check" size={15} />Approve</button>
          </footer>
        {/if}
      </div>
    </div>
  {/if}
</dialog>

<style>
  .detail { width: min(1080px, calc(100vw - 32px)); height: min(720px, calc(100dvh - 32px)); max-height: none; }
  .detail[open] { display: flex; flex-direction: column; }
  .detail:focus-visible { outline: none; }
  /* Walked through with the keys, so it opens and closes with no animation at all. */
  .detail.detail, .detail.detail::backdrop { transition: none; opacity: 1; scale: 1; }
  .top { display: flex; align-items: center; gap: 6px; min-height: 48px; padding: 0 8px; border-bottom: 1px solid var(--border); }
  .place { min-width: 84px; text-align: center; font-weight: 600; font-size: var(--t-small); }
  .grow { flex: 1; }
  /* The stage on --bg to the left, the facts and the verdicts to the right. */
  .main { flex: 1; min-height: 0; display: grid; grid-template-columns: minmax(0, 1fr) minmax(400px, 460px); }
  .pics { display: grid; place-items: center; min-height: 0; padding: 24px; background: var(--bg); overflow: auto; }
  .sizes { display: flex; flex-wrap: wrap; justify-content: center; align-items: flex-end; gap: 24px; max-width: 100%; }
  .sizes figure { display: grid; justify-items: center; gap: 6px; margin: 0; }
  .frame { display: block; max-width: 100%; }
  .frame img { display: block; width: 100%; height: 100%; object-fit: contain; }
  .sizes figcaption { display: grid; justify-items: center; font-size: 11px; line-height: 1.4; color: var(--dim); white-space: nowrap; }
  .sizes figcaption b { font-weight: 600; color: var(--muted); }
  .side { display: flex; flex-direction: column; min-width: 0; min-height: 0; border-left: 1px solid var(--border); }
  .scroll { flex: 1; min-height: 0; overflow: auto; padding: 4px 16px 8px; overscroll-behavior: contain; }
  .facts { min-width: 0; }
  .who { display: flex; align-items: center; gap: 6px; margin: 0; color: var(--muted); font-size: var(--t-small); }
  .chips { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 4px; }
  .chips .ui-chip { max-width: 100%; overflow: hidden; text-overflow: ellipsis; text-transform: none; letter-spacing: 0; }
  .chips .warn, .dupes .warn { color: var(--red); }
  .id { display: flex; align-items: center; justify-content: flex-end; gap: 6px; }
  .id .ui-mono { overflow: hidden; text-overflow: ellipsis; font-size: 12px; }
  .dupes h3 { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; }
  .dupes .ui-chip, .dwhy .ui-chip { height: 20px; font-size: 11px; text-transform: none; letter-spacing: 0; }
  .dupes p { margin: 0; min-height: 20px; }
  .retry { padding: 0; border: 0; background: none; font: inherit; color: var(--accent-text); cursor: pointer; }
  /* One row that scrolls sideways; the strip is the only horizontal scroller. */
  .strip { list-style: none; display: grid; grid-auto-flow: column; grid-auto-columns: minmax(180px, 220px); gap: 8px; margin: 0; padding: 0 0 4px;
    overflow-x: auto; overscroll-behavior-x: contain; scrollbar-width: thin; }
  .strip a { display: grid; grid-template-columns: 44px minmax(0, 1fr); align-items: center; gap: 8px; padding: 6px 8px 6px 6px; min-height: 56px;
    border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface); color: var(--text); text-decoration: none; }
  @media (hover: hover) and (pointer: fine) { .strip a:hover { border-color: var(--border-strong); } }
  .strip a:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  .dthumb { display: grid; place-items: center; width: 44px; height: 44px; border-radius: var(--radius); background: var(--bg); }
  .dtext { display: grid; gap: 1px; min-width: 0; }
  .dname, .dby { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dname { font-size: var(--t-small); font-weight: 600; }
  .dby { font-size: 12px; color: var(--muted); }
  .dwhy { display: flex; gap: 4px; margin-top: 2px; }
  .tabs { display: flex; }
  .pane { min-height: 120px; }
  .log > li { align-items: flex-start; justify-content: space-between; gap: 12px; }
  .line { display: grid; gap: 2px; min-width: 0; font-size: var(--t-small); }
  .more { color: var(--muted); white-space: pre-wrap; overflow-wrap: anywhere; }
  .log time { flex: none; font-size: 12px; font-weight: 600; color: var(--dim); user-select: none; }
  .detail footer { position: static; display: flex; align-items: center; justify-content: space-between; gap: 6px; padding: 12px; border-top: 1px solid var(--border); background: var(--elevated); }
  .small { display: flex; gap: 6px; }
  /* Phones: full screen, one scrolling column, the verdicts pinned at the bottom. */
  @media (max-width: 860px) {
    .detail { width: 100vw; max-width: 100vw; height: 100dvh; max-height: 100dvh; border: 0; border-radius: 0; }
    .main { display: flex; flex-direction: column; overflow: auto; overscroll-behavior: contain; }
    .pics { flex: none; min-height: 180px; padding: 16px; overflow: visible; }
    .side { display: contents; }
    .scroll { overflow: visible; flex: none; }
    .detail footer { position: sticky; bottom: 0; flex-wrap: wrap; padding-bottom: max(12px, env(safe-area-inset-bottom)); }
    .detail footer > .ui-btn { flex: 1; }
    .small { order: 3; width: 100%; justify-content: center; }
  }
</style>
