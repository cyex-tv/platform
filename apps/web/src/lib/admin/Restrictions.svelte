<script lang="ts">
  // An account's restrictions on its staff page: uploads, reports and tickets, the whole account.
  // Each row says what holds now and until when, and places one (a reason and a length: 1 hour,
  // 1 day, 7 days, 30 days, until a day you pick, or for good) or lifts it. The API ends a timed
  // one on its own; below, every restriction placed, with who, why and when it ended.
  import { gql } from '$lib/api';
  import type { AccountRestrictionsQuery } from '$lib/gql/graphql';
  import { ACCOUNT_RESTRICTIONS, LIFT_RESTRICTION, PLACE_RESTRICTION } from '$lib/queries';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import { dateTime, toLocalInput } from '$lib/format';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { can } from '$lib/tool/session.svelte';
  import AccountLink from './AccountLink.svelte';
  import Box from './ui/Box.svelte';
  import More from './ui/More.svelte';
  import { DURATIONS, endFor, HOLDS, type Duration, type HoldKind } from './restrictions';

  export type Status = {
    uploadsBlocked: boolean;
    ticketsBlocked: boolean;
    blocked: boolean;
    uploadsBlockedUntil?: string | null;
    ticketsBlockedUntil?: string | null;
    blockedUntil?: string | null;
  };

  let {
    accountId,
    status = $bindable(),
    self = false,
  }: { accountId: string; status: Status; self?: boolean } = $props();

  type Row = AccountRestrictionsQuery['accountRestrictions'][number];
  let history = $state<Row[] | null>(null);
  let problem = $state(false);
  let showAll = $state(false);
  const FIRST = 8;
  let seq = 0;

  async function loadHistory() {
    const mine = ++seq;
    problem = false;
    try {
      const r = (await gql(ACCOUNT_RESTRICTIONS, { accountId })).accountRestrictions;
      if (mine === seq) history = r;
    } catch {
      if (mine === seq) problem = true;
    }
  }
  $effect(() => {
    void accountId;
    history = null;
    showAll = false;
    loadHistory();
  });

  const on = (k: HoldKind) => (k === 'UPLOADS' ? status.uploadsBlocked : k === 'TICKETS' ? status.ticketsBlocked : status.blocked);
  const until = (k: HoldKind) => (k === 'UPLOADS' ? status.uploadsBlockedUntil : k === 'TICKETS' ? status.ticketsBlockedUntil : status.blockedUntil);
  const allowed = (perm: string) => can('users.restrict') || can(perm);
  const label = (k: HoldKind) => HOLDS.find((h) => h.kind === k)?.label ?? k;

  // The dialog.
  let dialog: HTMLDialogElement;
  let kind = $state<HoldKind>('UPLOADS');
  let length = $state<Duration>('1d');
  let custom = $state('');
  let reason = $state('');
  let error = $state('');
  let busy = $state(false);
  let minLocal = $state('');

  function open(k: HoldKind) {
    kind = k;
    length = k === 'ACCOUNT' ? '7d' : '1d';
    const now = new Date();
    minLocal = toLocalInput(new Date(now.getTime() + 60_000));
    custom = toLocalInput(new Date(now.getTime() + 14 * 24 * 3_600_000));
    reason = '';
    error = '';
    dialog.showModal();
  }

  async function place(e: SubmitEvent) {
    e.preventDefault();
    if (busy) return;
    error = '';
    if (!reason.trim()) {
      error = 'Give a reason. Staff see it in the history; the account doesn’t.';
      return;
    }
    let ends: string | null;
    try {
      ends = endFor(length, new Date(), custom);
    } catch (err) {
      error = (err as Error).message;
      return;
    }
    busy = true;
    try {
      status = (await gql(PLACE_RESTRICTION, { accountId, input: { kind, until: ends, reason: reason.trim() } })).placeRestriction;
      notify(`${label(kind)}: restricted ${ends ? `until ${dateTime(ends)}` : 'for good'}.`);
      dialog.close();
      loadHistory();
    } catch (err) {
      fail(err);
    } finally {
      busy = false;
    }
  }

  async function lift(k: HoldKind) {
    if (busy) return;
    busy = true;
    try {
      status = (await gql(LIFT_RESTRICTION, { accountId, kind: k })).liftRestriction;
      notify(`${label(k)}: lifted.`);
      loadHistory();
    } catch (err) {
      fail(err);
    } finally {
      busy = false;
    }
  }

  const ended = (r: Row) => (r.liftedAt ? `Lifted ${dateTime(r.liftedAt)}` : r.endsAt ? `Ended ${dateTime(r.endsAt)}` : 'Lifted');
</script>

<Box title="Restrictions">
  <ul class="holds">
    {#each HOLDS as h (h.kind)}
      {@const active = on(h.kind)}
      {@const end = until(h.kind)}
      {@const mayChange = allowed(h.permission) && !self}
      <li class="hold">
        <div class="what">
          <strong>{h.label}</strong>
          <span class="ui-hint" class:hot={active}>
            {#if !active}{h.hint}{:else if end}{h.stopped} until <span class="ui-num">{dateTime(end)}</span>{:else}{h.stopped} for good{/if}
          </span>
        </div>
        {#if mayChange && active}
          <button class="ui-btn ghost small" type="button" disabled={busy} onclick={() => open(h.kind)}>Change</button>
        {/if}
        <input class="ui-switch" type="checkbox" role="switch" aria-label={h.label} checked={active} disabled={!mayChange || busy}
          onchange={(e) => { e.currentTarget.checked = active; if (active) lift(h.kind); else open(h.kind); }} />
      </li>
    {/each}
  </ul>
  {#if self}<p class="box-none">You can’t restrict your own account.</p>{/if}

  <h3 class="box-sub">History</h3>
  {#if problem}
    <State bare kind="error" title="That didn’t load">
      {#snippet actions()}<button class="ui-btn secondary small" type="button" onclick={loadHistory}>Try again</button>{/snippet}
    </State>
  {:else if !history}
    <p class="box-none">Loading…</p>
  {:else if history.length === 0}
    <p class="box-none">Never restricted.</p>
  {:else}
    <ul class="rows">
      {#each showAll ? history : history.slice(0, FIRST) as r (r.id)}
        <li>
          <span class="top">
            <strong>{label(r.kind)}</strong>
            {#if r.active}<span class="ui-chip pending">Running</span>{/if}
            <span class="reason">{r.reason ?? 'No reason given.'}</span>
          </span>
          <span class="meta ui-hint">
            <span class="ui-num">{dateTime(r.createdAt)}</span> by <AccountLink id={r.createdBy} />
            · {r.endsAt ? `until ${dateTime(r.endsAt)}` : 'for good'}
            {#if !r.active}· {ended(r)}{#if r.liftedBy}{' '}by <AccountLink id={r.liftedBy} />{/if}{/if}
          </span>
        </li>
      {/each}
    </ul>
    <More total={history.length} limit={FIRST} bind:expanded={showAll} />
  {/if}
</Box>

<dialog class="ui-dialog restrict" bind:this={dialog} aria-labelledby="rd-title">
  <form onsubmit={place} novalidate>
    <div class="body">
      <h2 id="rd-title">Restrict: {label(kind)}</h2>
      <div class="ui-field">
        <span class="ui-label" aria-hidden="true">How long</span>
        <!-- Six choices don't fit a phone's row: a menu there, the switch on wider screens. -->
        <div class="len wide-only"><Segmented label="How long" size="s" options={DURATIONS} value={length} onchange={(v) => (length = v)} /></div>
        <select class="ui-input narrow-only" aria-label="How long" bind:value={length}>
          {#each DURATIONS as d (d.value)}<option value={d.value}>{d.label}</option>{/each}
        </select>
        {#if length === 'custom'}
          <label class="ui-field until">
            <span class="ui-label">Ends</span>
            <input class="ui-input" type="datetime-local" bind:value={custom} min={minLocal} />
          </label>
        {/if}
      </div>
      <label class="ui-field">
        <span class="ui-label">Reason</span>
        <textarea class="ui-input" rows="3" maxlength="500" bind:value={reason} aria-describedby="rd-reason-hint"></textarea>
        <span class="ui-hint" id="rd-reason-hint">Staff see it in the history; the account doesn’t.{#if kind === 'ACCOUNT'} Blocking signs them out everywhere.{/if}</span>
      </label>
      {#if error}<p class="ui-error" role="alert">{error}</p>{/if}
    </div>
    <footer>
      <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
      <button class="ui-btn {kind === 'ACCOUNT' ? 'danger' : 'primary'}" type="submit" disabled={busy}>{kind === 'ACCOUNT' ? 'Block' : 'Restrict'}</button>
    </footer>
  </form>
</dialog>

<style>
  ul { list-style: none; margin: 0; padding: 0; }
  .hold { display: flex; align-items: center; gap: 8px; min-height: 56px; padding: 6px 8px 6px 16px; border-bottom: 1px solid var(--border); }
  .hold:last-child { border-bottom: 0; }
  .what { display: grid; flex: 1; min-width: 0; }
  .what .ui-hint.hot { color: var(--text); font-weight: 600; }
  .rows li { display: grid; gap: 1px; min-height: 52px; padding: 6px 16px; border-top: 1px solid var(--border); align-content: center; min-width: 0; }
  .rows li:first-child { border-top: 0; }
  .top { display: flex; align-items: center; gap: 8px; min-width: 0; font-size: var(--t-small); }
  .top strong { flex: none; }
  .top .ui-chip { flex: none; }
  .reason { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--muted); }
  .meta { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .restrict { width: min(560px, calc(100vw - 24px)); }
  .narrow-only { display: none; }
  @media (max-width: 599.98px) { .wide-only { display: none; } .narrow-only { display: block; } }
  .until { margin-top: 8px; }
  textarea { resize: vertical; min-height: 72px; }
</style>
