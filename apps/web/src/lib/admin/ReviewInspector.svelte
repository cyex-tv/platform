<script lang="ts">
  // The review queue's inspector on wide windows: the picked upload the way the public emote page
  // shows it, in the panel right of the queue (no dialog). Every size chat uses on one baseline
  // (cells sharing hairlines) with dimensions and weight, one line of chat at chat size, the name,
  // the uploader with their painted name and their history here (earlier uploads, how many were
  // approved, rejected or still wait), flags and tags, the facts, possible duplicates (the same
  // file, then close names), and Activity / Notes (and Approvals on resolved uploads). The
  // verdicts sit at the panel's foot (ADR 0038): Reject, Skip, the single approvals and denials,
  // and Approve both. Which ones show follows the queue and the reviewer's permissions. Their keys
  // (A, L, P, D, N, R, S, Z) are the page's; the panel carries data-inspector so they reach it.
  // Phones and narrow windows keep ReviewDetail (the full-screen dialog).
  import { gql } from '$lib/api';
  import { ready, src } from '$lib/emote';
  import { bytes, clean, count, date, dateTime } from '$lib/format';
  import type { AccountEmotesQuery, EmoteDuplicatesQuery, EmoteFieldsFragment, EmoteHistoryQuery } from '$lib/gql/graphql';
  import { ACCOUNT_EMOTES, ADD_EMOTE_NOTE, DELETE_EMOTE_NOTE, EMOTE_DUPLICATES, EMOTE_HISTORY, EMOTE_NOTES } from '$lib/queries';
  import Thumb from '$lib/tool/Thumb.svelte';
  import { STATE_LABEL } from '$lib/tool/state';
  import IconButton from '$lib/components/IconButton.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { looks, wantLooks } from '$lib/cosmetics/store.svelte';
  import { useCopied } from '$lib/clipboard.svelte';
  import AccountLink from './AccountLink.svelte';
  import NoteList from './NoteList.svelte';
  import ApprovalsPanel from './ApprovalsPanel.svelte';
  import { can } from '$lib/tool/session.svelte';
  import { actionLabel, actionsFor, APPROVAL_LABEL, describe, kindOf, sizes, type Action, type Approvals, type Mode, type Reviewer } from './queue';

  type Props = {
    emote: EmoteFieldsFragment & { approvals?: Approvals | null };
    /** 1-based place in the whole filtered queue, and its size. */
    place: number;
    total: number;
    busy: boolean;
    resolved: boolean;
    hasNext: boolean;
    onskip: () => void;
    /** The queue shown (the personal-use one only decides personal use) and what this reviewer may do. */
    mode?: Mode;
    reviewer: Reviewer;
    onact: (action: Action) => void;
    /** A resolved upload's approvals were changed in the Approvals tab: the API's answer. */
    onapprovals?: (id: string, a: Approvals) => void;
  };
  let { emote, place, total, busy, resolved, hasNext, onskip, mode = 'main', reviewer, onact, onapprovals }: Props = $props();

  const e = $derived(emote);
  const files = $derived(sizes(e.files));
  const chatFile = $derived(files.find((f) => f.scale === 1) ?? files[0]);
  const ownerPaint = $derived(e.owner?.id ? (looks[e.owner.id]?.paint ?? null) : null);
  const actions = $derived(actionsFor(mode, reviewer));
  const primary = $derived(actions[0]);
  const idCopy = useCopied();

  let tab = $state<'activity' | 'notes' | 'approvals'>('activity');
  // After the first verdict staff change either approval here (ADR 0038).
  const approvalTab = $derived(resolved && (can('emotes.review') || can('emotes.review_personal')));
  $effect(() => {
    if (!approvalTab && tab === 'approvals') tab = 'activity';
  });
  let noteCount = $state<number | null>(null);

  // Three side loads per upload; each keeps only its latest answer.
  let history = $state<EmoteHistoryQuery['emoteHistory'] | null>(null);
  let historyProblem = $state(false);
  let dupes = $state<EmoteDuplicatesQuery['emoteDuplicates'] | null>(null);
  let dupesProblem = $state(false);
  type Earlier = AccountEmotesQuery['accountEmotes'];
  let earlier = $state<Earlier | null>(null);
  let earlierProblem = $state(false);
  let seq = 0;
  async function loadHistory(id: string, mine: number) {
    history = null;
    historyProblem = false;
    try {
      const r = (await gql(EMOTE_HISTORY, { emoteId: id })).emoteHistory;
      if (mine === seq) history = r;
    } catch {
      if (mine === seq) historyProblem = true;
    }
  }
  async function loadDupes(id: string, mine: number) {
    dupes = null;
    dupesProblem = false;
    try {
      const r = (await gql(EMOTE_DUPLICATES, { emoteId: id })).emoteDuplicates;
      if (mine === seq) dupes = r;
    } catch {
      if (mine === seq) dupesProblem = true;
    }
  }
  // The uploader's history: their latest uploads (this one left out), in every state.
  const EARLIER = 24;
  const owners = new Map<string, Earlier>();
  async function loadEarlier(accountId: string, mine: number) {
    earlierProblem = false;
    const known = owners.get(accountId);
    earlier = known ?? null;
    if (known) return;
    try {
      const r = (await gql(ACCOUNT_EMOTES, { accountId, page: 1, perPage: EARLIER })).accountEmotes;
      owners.set(accountId, r);
      if (mine === seq) earlier = r;
    } catch {
      if (mine === seq) earlierProblem = true;
    }
  }
  $effect(() => {
    const id = e.id;
    const owner = e.owner?.id;
    const mine = ++seq;
    noteCount = null;
    loadHistory(id, mine);
    loadDupes(id, mine);
    if (owner) {
      wantLooks([owner]);
      loadEarlier(owner, mine);
    } else earlier = null;
  });
  const others = $derived(earlier ? earlier.items.filter((x) => x.id !== e.id) : []);
  const tally = $derived.by(() => {
    const t = { APPROVED: 0, REJECTED: 0, PENDING: 0, TAKEN_DOWN: 0 } as Record<string, number>;
    for (const x of others) t[x.state] = (t[x.state] ?? 0) + 1;
    return t;
  });
  const sameFile = $derived(dupes?.filter((d) => d.sameFile).length ?? 0);
</script>

<div class="insp" data-inspector>
  <div class="adm-ip-head">
    <span class="adm-label">Upload <span class="adm-num place" aria-live="polite">{count(place)} / {count(total)}</span></span>
    <IconButton label="Open the uploader’s account" icon="circle-user" size="s" tip="bottom-end" href={e.owner ? `/admin/users/${encodeURIComponent(e.owner.id)}` : undefined} disabled={!e.owner} />
    <IconButton label="Open the emote page" icon="arrow-up-right" size="s" tip="bottom-end" href="/emotes/{encodeURIComponent(e.id)}" />
  </div>

  <div class="adm-ip-body">
    <section class="adm-sec top" aria-label="Sizes">
      {#if ready(e) && files.length}
        <div class="sizes">
          {#each files as f (f.name)}
            <figure class="size">
              <span class="pic"><img src={src(e, f.scale as 1 | 2 | 3 | 4)} width={f.width} height={f.height} style:max-width="{f.width}px" alt={f.scale === 4 ? clean(e.name) : ''} decoding="async" /></span>
              <figcaption><b>{f.scale}x</b><span class="adm-num">{f.width}×{f.height} · {bytes(f.size)}</span></figcaption>
            </figure>
          {/each}
        </div>
        {#if chatFile}
          <p class="chatline"><span class="sr-only">How it looks in chat: </span><span class="viewer">nora_byte</span><span>that ending</span><img
            class="inline" src={src(e, 1)} alt={clean(e.name)} height="28" style:aspect-ratio="{chatFile.width} / {chatFile.height}" /><span>again please</span></p>
        {/if}
      {:else}
        <div class="sizes wait"><State bare kind="loading" title="Files not made yet" /></div>
      {/if}
    </section>

    <section class="adm-sec who">
      <span class="adm-label">{kindOf(e)}</span>
      <h2 class="adm-name big"><bdi>{clean(e.name)}</bdi></h2>
      <p class="by">
        {#if e.owner}
          <a class="owner" href="/admin/users/{encodeURIComponent(e.owner.id)}"><span class="sr-only">Uploaded by </span><PaintedName name={clean(e.owner.displayName)} paint={ownerPaint} /></a>
        {:else}<span>Unknown uploader</span>{/if}
        <span class="when">uploaded <time class="adm-num" datetime={e.createdAt}>{dateTime(e.createdAt)}</time></span>
      </p>
      {#if e.flags.zeroWidth || e.flags.wide || e.flags.private || e.tags.length}
        <div class="labels">
          <ul class="adm-flags" aria-label="Flags">
            {#if e.flags.zeroWidth}<li class="on"><Icon name="layers" size={13} />Zero-width</li>{/if}
            {#if e.flags.wide}<li class="on"><Icon name="wide" size={13} />Wide</li>{/if}
            {#if e.flags.private}<li class="on"><Icon name="lock" size={13} />Private</li>{/if}
          </ul>
          {#if e.tags.length}
            <ul class="tags" aria-label="Tags">{#each e.tags as t (t)}<li class="tag"><span class="hash" aria-hidden="true">#</span><bdi>{t}</bdi></li>{/each}</ul>
          {/if}
        </div>
      {/if}
      <dl class="adm-facts">
        <div><dt>State</dt><dd class:ok={e.state === 'APPROVED'} class:bad={e.state === 'REJECTED' || e.state === 'TAKEN_DOWN'}>{STATE_LABEL[e.state]}</dd></div>
        <div><dt>Public listing</dt><dd class:ok={e.approvals?.publicListing === 'APPROVED'} class:bad={e.approvals?.publicListing === 'DENIED'}>{e.approvals ? (APPROVAL_LABEL[e.approvals.publicListing] ?? e.approvals.publicListing) : 'Waiting'}</dd></div>
        <div><dt>Personal use</dt><dd class:ok={e.approvals?.personalUse === 'APPROVED'} class:bad={e.approvals?.personalUse === 'DENIED'}>{e.approvals ? (APPROVAL_LABEL[e.approvals.personalUse] ?? e.approvals.personalUse) : 'Waiting'}</dd></div>
        <div><dt>Private</dt><dd>{e.flags.private ? 'Yes' : 'No'}</dd></div>
        <div><dt>Uploaded</dt><dd class="n">{date(e.createdAt)}</dd></div>
        <div class="id"><dt>ID</dt><dd class="n"><span class="idv">{e.id}</span><IconButton label={idCopy.copied ? 'Copied' : 'Copy ID'} icon={idCopy.copied ? 'check' : 'copy'} size="s" onclick={() => idCopy.copy(e.id)} /></dd></div>
      </dl>
    </section>

    <section class="adm-sec" aria-labelledby="ri-up">
      <h3 id="ri-up" class="adm-label">Uploader’s history{#if earlier}<span class="n">{count(Math.max(0, earlier.total - (earlier.items.some((x) => x.id === e.id) ? 1 : 0)))} earlier</span>{/if}</h3>
      {#if !e.owner}
        <p class="hint">No uploader on record.</p>
      {:else if earlierProblem}
        <p class="hint">Couldn’t load their uploads. <button class="retry" type="button" onclick={() => e.owner && loadEarlier(e.owner.id, seq)}>Try again</button></p>
      {:else if !earlier}
        <p class="hint" aria-busy="true">Loading…</p>
      {:else if !others.length}
        <p class="hint">This is their first upload.</p>
      {:else}
        <ul class="adm-flags tally">
          <li class="ok"><span class="adm-num">{count(tally.APPROVED)}</span> approved</li>
          <li class:warn={tally.REJECTED > 0}><span class="adm-num">{count(tally.REJECTED)}</span> rejected</li>
          <li><span class="adm-num">{count(tally.PENDING)}</span> waiting</li>
          {#if tally.TAKEN_DOWN}<li class="warn"><span class="adm-num">{count(tally.TAKEN_DOWN)}</span> taken down</li>{/if}
          {#if earlier.total > earlier.items.length}<li>of the latest <span class="adm-num">{count(others.length)}</span></li>{/if}
        </ul>
        <ul class="adm-strip small" aria-label="Earlier uploads">
          {#each others.slice(0, 12) as x (x.id)}
            <li>
              <a class="mini" href="/emotes/{encodeURIComponent(x.id)}" target="_blank" rel="noopener" title="{clean(x.name)}: {STATE_LABEL[x.state]}">
                <span class="mpic"><Thumb emote={x} name="" box={32} /></span>
                <span class="mname"><bdi>{clean(x.name)}</bdi></span>
                <span class="mstate" class:ok={x.state === 'APPROVED'} class:bad={x.state === 'REJECTED' || x.state === 'TAKEN_DOWN'}>{STATE_LABEL[x.state]}</span>
              </a>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="adm-sec" aria-labelledby="ri-dupes">
      <h3 id="ri-dupes" class="adm-label">
        Possible duplicates
        {#if sameFile}<span class="same">{sameFile === 1 ? 'Same file as 1 other' : `Same file as ${count(sameFile)} others`}</span>{/if}
        {#if dupes?.length}<span class="n">{count(dupes.length)}</span>{/if}
      </h3>
      {#if dupesProblem}
        <p class="hint">Couldn’t check for duplicates. <button class="retry" type="button" onclick={() => loadDupes(e.id, seq)}>Try again</button></p>
      {:else if dupes === null}
        <p class="hint" aria-busy="true">Checking…</p>
      {:else if dupes.length === 0}
        <p class="hint">None: no other emote has this file or a close name.</p>
      {:else}
        <ul class="adm-strip" aria-label="Possible duplicates">
          {#each dupes as d (d.emote.id)}
            <li>
              <a class="mini" href="/emotes/{encodeURIComponent(d.emote.id)}" target="_blank" rel="noopener" title="{clean(d.emote.name)} by {clean(d.emote.owner?.displayName) || 'unknown'}">
                <span class="mpic"><Thumb emote={d.emote} name="" box={32} /></span>
                <span class="mname"><bdi>{clean(d.emote.name)}</bdi></span>
                <span class="mstate">
                  {#if d.sameFile}<b class="bad">Same file</b>{:else}Name <span class="adm-num">{Math.round(d.similarity * 100)}%</span>{/if}
                  · <bdi>{clean(d.emote.owner?.displayName) || 'Unknown'}</bdi>
                </span>
              </a>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="adm-sec lower">
      <Segmented label="Show" size="s" value={tab} onchange={(v) => (tab = v)}
        options={[
          { value: 'activity', label: history ? `Activity ${history.length + 1}` : 'Activity', icon: 'activity' },
          { value: 'notes', label: noteCount === null ? 'Notes' : `Notes ${noteCount}`, icon: 'message-square' },
          ...(approvalTab ? [{ value: 'approvals' as const, label: 'Approvals', icon: 'check' as const }] : []),
        ]} />
      <div class="pane">
        <div hidden={tab !== 'activity'}>
          {#if historyProblem}
            <State bare kind="error" title="That didn’t load">
              {#snippet actions()}<button class="ui-btn secondary small" type="button" onclick={() => loadHistory(e.id, seq)}>Try again</button>{/snippet}
            </State>
          {:else if !history}
            <p class="hint" aria-busy="true">Loading activity…</p>
          {:else}
            <ol class="log">
              {#each history as h (h.id)}
                {@const d = describe(h)}
                <li>
                  <span class="line"><span>{d.text} · <AccountLink id={h.actorId} /></span>{#if d.detail}<span class="more">{d.detail}</span>{/if}</span>
                  <time class="adm-num" datetime={h.createdAt}>{dateTime(h.createdAt)}</time>
                </li>
              {/each}
              <li>
                <span class="line"><span>Uploaded · <AccountLink id={e.owner?.id} name={e.owner?.displayName} /></span></span>
                <time class="adm-num" datetime={e.createdAt}>{dateTime(e.createdAt)}</time>
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
        {#if approvalTab}
          <div hidden={tab !== 'approvals'}>
            <ApprovalsPanel emote={e} id="ri-approvals" onsaved={(a) => (onapprovals?.(e.id, a), loadHistory(e.id, seq))} />
          </div>
        {/if}
      </div>
    </section>
  </div>

  {#if !resolved && actions.length}
    {@const singles = actions.filter((a) => a !== 'reject' && a !== primary)}
    <footer class="verdicts" aria-label="Verdict">
      {#if actions.includes('reject')}<button class="ui-btn danger small" type="button" disabled={busy} onclick={() => onact('reject')}><Icon name="x" size={15} />Reject</button>{/if}
      <span class="mid">
        <IconButton label="Skip to the next, no verdict" icon="arrow-right" variant="outline" tip="top" disabled={!hasNext} onclick={onskip} />
        {#each singles as a (a)}
          <IconButton label={actionLabel(a, reviewer, mode)} icon={a === 'listing' ? 'eye' : a === 'personal' ? 'user' : a === 'deny-listing' ? 'eye-off' : 'user-x'} variant="outline" tip="top" disabled={busy || (!a.startsWith('deny') && !ready(e))} onclick={() => onact(a)} />
        {/each}
      </span>
      <button class="ui-btn primary small" type="button" disabled={busy || !ready(e)} onclick={() => onact(primary)}><Icon name="check" size={15} />{actionLabel(primary, reviewer, mode)}</button>
    </footer>
  {/if}
</div>

<style>
  p, h3 { margin: 0; }
  .insp { display: flex; flex-direction: column; min-height: calc(100svh - var(--nav-h)); }
  .insp .adm-ip-body { flex: 1; }
  .place { margin-left: 8px; letter-spacing: 0; text-transform: none; color: var(--muted); }
  .top { padding-top: 16px; }

  /* Every size on one baseline: cells sharing hairlines, the emote at its real size. */
  .sizes { display: grid; grid-auto-flow: column; grid-auto-columns: minmax(0, 1fr); border: 1px solid var(--border); background: var(--bg); }
  .sizes.wait { display: grid; place-items: center; min-height: 160px; }
  .size { display: grid; grid-template-rows: 1fr auto; justify-items: center; gap: 8px; min-width: 0; min-height: 168px; margin: 0; padding: 12px 6px 8px;
    box-shadow: -1px 0 0 var(--border); }
  .size .pic { display: grid; place-items: end center; min-width: 0; max-width: 100%; }
  .size img { display: block; max-width: 100%; height: auto; object-fit: contain; }
  figcaption { display: grid; justify-items: center; gap: 1px; font-size: var(--t-fine); color: var(--muted); white-space: nowrap; }
  figcaption b { color: var(--text); font-weight: 700; }
  figcaption .adm-num { font-size: 11px; color: var(--dim); }
  .chatline { display: flex; flex-wrap: wrap; align-items: center; gap: 4px 6px; min-height: 48px; padding: 10px 12px; border: 1px solid var(--border);
    border-top: 0; font-size: var(--t-small); }
  .viewer { font-weight: 700; color: var(--accent-text); }
  .viewer::after { content: ':'; color: var(--muted); font-weight: 400; }
  .inline { display: block; height: 28px; width: auto; }

  .big { font-size: 24px; letter-spacing: -.02em; }
  .by { display: flex; flex-wrap: wrap; align-items: center; gap: 4px 8px; font-size: var(--t-small); color: var(--muted); min-width: 0; }
  .owner { font-weight: 700; text-decoration: none; min-width: 0; }
  .when { white-space: nowrap; }
  .labels { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 14px; }
  .tags { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 6px; }
  .tag { display: inline-flex; align-items: center; height: 24px; padding: 0 8px; border-radius: var(--radius); background: var(--neutral-chip); font-size: var(--t-fine); font-weight: 600; }
  .hash { color: var(--dim); }
  .adm-facts { margin-top: 4px; }
  .adm-facts .id { grid-column: 1 / -1; }
  .adm-facts .id dd { display: flex; align-items: center; justify-content: space-between; gap: 6px; }
  .idv { min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  dd.ok, .ok { color: var(--green); }
  dd.bad, .bad { color: var(--red); }

  .adm-label { display: flex; align-items: center; gap: 8px; }
  .adm-label .n { margin-left: auto; font-family: var(--font-mono); font-weight: 500; letter-spacing: 0; text-transform: none; color: var(--dim); }
  .same { color: var(--red); letter-spacing: 0; text-transform: none; font-weight: 600; }
  .hint { min-height: 20px; font-size: var(--t-small); color: var(--muted); }
  .retry { padding: 0; border: 0; background: none; font: inherit; color: var(--accent-text); cursor: pointer; }
  .tally { gap: 2px 12px; }
  .adm-strip { grid-auto-columns: minmax(140px, 170px); }
  .adm-strip.small { grid-auto-columns: minmax(118px, 136px); }
  .mini { display: grid; grid-template-columns: 34px minmax(0, 1fr); grid-template-rows: auto auto; column-gap: 8px; align-items: center; min-height: 52px; padding: 6px 8px;
    color: var(--text); text-decoration: none; transition: background-color var(--fast); }
  .mpic { grid-row: 1 / 3; display: grid; place-items: center; width: 34px; height: 34px; }
  .mname, .mstate { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .mname { font-size: var(--t-fine); font-weight: 600; }
  .mstate { font-size: 12px; color: var(--muted); }
  .mini:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  @media (hover: hover) and (pointer: fine) { .mini:hover { background: var(--row-hover); } }

  .pane { min-height: 120px; }
  .log { list-style: none; margin: 0; padding: 0; }
  .log > li { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; padding: 8px 0; border-bottom: 1px solid var(--border); }
  .log > li:last-child { border-bottom: 0; }
  .line { display: grid; gap: 2px; min-width: 0; font-size: var(--t-small); }
  .more { color: var(--muted); white-space: pre-wrap; overflow-wrap: anywhere; }
  .log time { flex: none; font-size: 12px; color: var(--dim); }

  .verdicts { position: sticky; bottom: 0; display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 8px; padding: 10px 16px;
    border-top: 1px solid var(--border); background: var(--surface); }
  .mid { display: flex; gap: 6px; }
</style>
