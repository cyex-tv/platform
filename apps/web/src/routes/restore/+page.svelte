<script lang="ts">
  // Where signing in to a deleted account lands within its 30 days (ADR 0019). The session it got
  // can only see the deletion and undo it: "Restore account" brings everything back and opens
  // your profile; "Keep it deleted" signs out again. Without such a session (you came here
  // another way, or the window has passed) the one empty state says there's nothing to restore.
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { ApiError, gql } from '$lib/api';
  import Avatar from '$lib/components/Avatar.svelte';
  import State from '$lib/components/State.svelte';
  import { clean, date } from '$lib/format';
  import { PENDING_DELETION, RESTORE_ACCOUNT } from '$lib/queries';
  import type { PendingDeletionQuery } from '$lib/gql/graphql';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import { leave } from '$lib/shell/signout';
  import { notify } from '$lib/tool/notices.svelte';
  import { loadMe } from '$lib/tool/session.svelte';

  type Pending = NonNullable<PendingDeletionQuery['pendingDeletion']>;

  let status = $state<'loading' | 'ready' | 'none' | 'failed'>('loading');
  let pending = $state<Pending | null>(null);
  let busy = $state<'restore' | 'leave' | null>(null);
  let problem = $state('');

  const name = $derived(clean(pending?.displayName ?? '') || 'Your account');
  const days = $derived(pending ? Math.max(0, Math.ceil((Date.parse(pending.eraseAfter) - Date.now()) / 86_400_000)) : 0);

  async function load() {
    status = 'loading';
    try {
      const r = await gql(PENDING_DELETION, {});
      pending = r.pendingDeletion ?? null;
      status = pending ? 'ready' : 'none';
    } catch {
      status = 'failed';
    }
  }

  onMount(load);

  async function restore() {
    if (busy) return;
    busy = 'restore';
    problem = '';
    try {
      await gql(RESTORE_ACCOUNT, {});
    } catch (err) {
      busy = null;
      if (err instanceof ApiError && (err.code === 'nothing_to_restore' || err.code === 'signed_out')) {
        status = 'none';
        return;
      }
      problem = 'Restoring didn’t work. Try again in a moment.';
      return;
    }
    const me = await loadMe({ fresh: true }).catch(() => null);
    notify('Your account is back.');
    await goto(me ? `/users/${me.id}` : '/', { invalidateAll: true });
    busy = null;
  }

  async function keepDeleted() {
    if (busy) return;
    busy = 'leave';
    await leave();
    busy = null;
  }
</script>

<svelte:head>
  <title>Restore your account - cyex.tv</title>
  <meta name="robots" content="noindex" />
</svelte:head>

<ColumnLayout top>
  <div class="center">
    {#if status === 'loading'}
      <div class="wide"><State kind="loading" title="Loading" /></div>
    {:else if status === 'failed'}
      <div class="wide">
        <State kind="error" title="This page didn’t load" level={1}>
          Nothing was changed. Try again in a moment.
          {#snippet actions()}<button class="ui-btn secondary" type="button" onclick={load}>Try again</button>{/snippet}
        </State>
      </div>
    {:else if status === 'none' || !pending}
      <div class="wide">
        <State title="Nothing to restore" level={1} role="status">
          There’s no deleted account waiting here. Accounts can be restored for 30 days after they’re deleted, by signing in with them.
          {#snippet actions()}<a class="ui-btn secondary" href="/">Go to the front page</a>{/snippet}
        </State>
      </div>
    {:else}
      <section class="card" aria-labelledby="restore-h">
        <div class="who">
          <Avatar src={pending.avatar} {name} size={48} />
          <div>
            <h1 class="page-title" id="restore-h">Restore your account</h1>
            <p class="name"><bdi>{name}</bdi></p>
          </div>
        </div>
        <p class="lead">
          You deleted this account on {date(pending.requestedAt)}. It’s hidden now, and it will be deleted for good on
          <strong>{date(pending.eraseAfter)}</strong>{#if days > 0}{' '}({days === 1 ? 'tomorrow' : `in ${days} days`}){/if}.
        </p>
        <p class="lead">Restore it to bring back your profile, sets, emotes and channels as they were.</p>
        {#if problem}<p class="ui-error" role="alert">{problem}</p>{/if}
        <div class="choices">
          <button class="btn btn-s btn-primary big" type="button" onclick={restore} disabled={busy !== null}>
            {busy === 'restore' ? 'Restoring…' : 'Restore account'}
          </button>
          <button class="btn btn-s big" type="button" onclick={keepDeleted} disabled={busy !== null}>
            {busy === 'leave' ? 'Signing out…' : 'Keep it deleted'}
          </button>
        </div>
        <p class="fine note">Keeping it deleted signs you out. You can still change your mind by signing in again before {date(pending.eraseAfter)}.</p>
      </section>
    {/if}
  </div>
</ColumnLayout>

<style>
  /* The sign-in page's measure: one centred card. */
  .center { display: grid; justify-items: center; padding-block: clamp(24px, 6vw, 72px); }
  .wide { width: 100%; max-width: var(--measure-form); }
  .card { width: 100%; max-width: var(--measure-form); padding: clamp(20px, 4vw, 32px); border: 1px solid var(--border);
    border-radius: var(--radius-card); background: var(--surface); }
  .who { display: flex; align-items: center; gap: 14px; min-width: 0; }
  .who > div { min-width: 0; }
  .who h1 { margin: 0; }
  .name { margin: 2px 0 0; color: var(--muted); font-size: var(--t-ui); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .lead { margin: 16px 0 0; font-size: var(--t-ui); line-height: 1.55; color: var(--muted); max-width: none; text-wrap: pretty; }
  .lead strong { color: var(--text); font-weight: 600; }
  .lead + .lead { margin-top: 8px; }
  .ui-error { margin: 12px 0 0; }
  .choices { display: grid; gap: 10px; margin-top: 24px; }
  .big { width: 100%; height: 44px; justify-content: center; font-size: var(--t-ui); }
  .btn.big:not(.btn-primary) { background: var(--bg); }
  .note { margin: 16px 0 0; }
</style>
