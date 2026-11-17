<script lang="ts">
  import { goto } from '$app/navigation';
  import { gql, linkUrl, signOut } from '$lib/api';
  import { date, iso } from '$lib/format';
  import Name from '$lib/components/Name.svelte';
  import type { Platform } from '$lib/gql/graphql';
  import { MARK_READ, MY_NOTIFICATIONS, UNLINK } from '$lib/queries';
  import Avatar from '$lib/components/Avatar.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import { forgetMe, loadMe, session } from '$lib/tool/session.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';

  const PLATFORMS: { id: Platform; name: string; slug: 'twitch' | 'kick' }[] = [
    { id: 'TWITCH', name: 'Twitch', slug: 'twitch' },
    { id: 'KICK', name: 'Kick', slug: 'kick' },
  ];

  const L = loader(async ({ fetch }) => (await gql(MY_NOTIFICATIONS, {}, fetch)).myNotifications);
  const notes = $derived(L.data);
  const me = $derived(session.me!);
  const unread = $derived(notes?.filter((n) => !n.read).length ?? 0);

  async function unlink(p: Platform, name: string) {
    try {
      await gql(UNLINK, { platform: p });
      await loadMe({ fresh: true });
      notify(`${name} unlinked.`);
    } catch (err) {
      fail(err);
    }
  }

  async function markRead() {
    try {
      await gql(MARK_READ, {});
      L.set(notes?.map((n) => ({ ...n, read: true })) ?? null);
    } catch (err) {
      fail(err);
    }
  }

  async function leave() {
    try {
      await signOut();
      forgetMe();
      await goto('/');
    } catch (err) {
      fail(err);
    }
  }

  /** A notification in words. Payloads are JSON; unknown kinds show their kind. */
  function describe(kind: string, payload: string): string {
    let data: Record<string, unknown> = {};
    try {
      data = JSON.parse(payload);
    } catch {}
    const name = typeof data.name === 'string' ? iso(data.name) : 'An emote';
    const reason = typeof data.reason === 'string' ? ` Reason: ${iso(data.reason)}` : '';
    if (kind === 'emote.taken_down') return `${name} was taken down.${reason}`;
    if (kind === 'emote.rejected') return `${name} was rejected.${reason}`;
    if (kind === 'emote.approved') return `${name} was approved.`;
    return kind;
  }
</script>

<svelte:head><title>Account - cyex.tv</title></svelte:head>

<div class="narrow">
  <div class="ui-head"><div><h1>Account</h1><p>Member since <span class="ui-mono">{date(me.createdAt)}</span></p></div></div>

  <div class="ui-stack">
    <section class="ui-panel">
      <header><h2>Linked accounts</h2></header>
      <ul class="ui-list">
        {#each PLATFORMS as p}
          {@const id = me.identities.find((i) => i.platform === p.id)}
          <li>
            {#if id}
              <Avatar src={id.avatar} name={id.displayName} size={28} tone="tool" />
              <span class="what"><strong><Name value={id.displayName} /></strong><span class="ui-hint">{p.name} · <span class="ui-mono">{id.platformUserId}</span></span></span>
              {#if me.identities.length > 1}
                <Confirm small label="Unlink" confirm="Click again to unlink" onconfirm={() => unlink(p.id, p.name)} />
              {:else}
                <span class="ui-hint">Your only sign-in</span>
              {/if}
            {:else}
              <span class="blank" aria-hidden="true"></span>
              <span class="what"><strong>{p.name}</strong><span class="ui-hint">Not linked</span></span>
              <a class="ui-btn secondary small" href={linkUrl(p.slug)}>Link {p.name}</a>
            {/if}
          </li>
        {/each}
      </ul>
      <p class="ui-hint pad">Link both to sign in with either. cyex.tv keeps the user ID, login, display name and picture.</p>
    </section>

    <section class="ui-panel">
      <header>
        <h2>Notifications</h2>
        {#if unread}<button class="ui-btn ghost small" type="button" onclick={markRead}>Mark {unread} read</button>{/if}
      </header>
      <LoadGate source={L} label="Loading notifications…">
      {#snippet children(notes)}
      {#if notes.length === 0}
        <p class="ui-hint pad">Nothing yet. Moderation decisions about your emotes show up here.</p>
      {:else}
        <ul class="ui-list">
          {#each notes as n (n.id)}
            <li class:unread={!n.read}>
              <span class="what"><span>{describe(n.kind, n.payload)}</span><span class="ui-hint ui-mono">{date(n.createdAt)}</span></span>
            </li>
          {/each}
        </ul>
      {/if}
      {/snippet}
      </LoadGate>
    </section>

    {#if me.permissions.length}
      <section class="ui-panel">
        <header><h2>Permissions</h2></header>
        <p class="pad perms">{#each me.permissions as p}<span class="ui-chip">{p}</span>{/each}</p>
      </section>
    {/if}

    <section class="ui-card">
      <h2>Sign out</h2>
      <p class="ui-hint">Ends this session. Your sets and channels stay as they are.</p>
      <div class="ui-actions gap"><button class="ui-btn secondary" type="button" onclick={leave}>Sign out</button></div>
    </section>
  </div>
</div>

<style>
  .what { display: grid; flex: 1; min-width: 0; }
  .what strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .blank { width: 28px; height: 28px; border-radius: 50%; border: 1px dashed var(--border-strong); flex: none; }
  .pad { padding: 10px 14px 12px; }
  .unread { box-shadow: inset 2px 0 0 var(--accent); }
  .perms { display: flex; flex-wrap: wrap; gap: 6px; text-transform: none; }
  .perms .ui-chip { text-transform: none; letter-spacing: 0; font-family: var(--font-mono); font-weight: 600; }
  .gap { margin-top: 12px; }
</style>
