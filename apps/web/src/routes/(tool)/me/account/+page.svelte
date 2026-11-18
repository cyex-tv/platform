<script lang="ts">
  import { gql, linkUrl } from '$lib/api';
  import DataPanel from '$lib/account/DataPanel.svelte';
  import DeleteAccount from '$lib/account/DeleteAccount.svelte';
  import SessionsPanel from '$lib/account/SessionsPanel.svelte';
  import OrdersPanel from '$lib/shop/OrdersPanel.svelte';
  import { date } from '$lib/format';
  import Name from '$lib/components/Name.svelte';
  import type { Platform } from '$lib/gql/graphql';
  import { UNLINK } from '$lib/queries';
  import Avatar from '$lib/components/Avatar.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import { leave } from '$lib/shell/signout';
  import { loadMe, session } from '$lib/tool/session.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';

  const PLATFORMS: { id: Platform; name: string; slug: 'twitch' | 'kick' }[] = [
    { id: 'TWITCH', name: 'Twitch', slug: 'twitch' },
    { id: 'KICK', name: 'Kick', slug: 'kick' },
  ];

  const me = $derived(session.me!);

  async function unlink(p: Platform, name: string) {
    try {
      await gql(UNLINK, { platform: p });
      await loadMe({ fresh: true });
      notify(`${name} unlinked.`);
    } catch (err) {
      fail(err);
    }
  }
</script>

<svelte:head><title>Account - cyex.tv</title></svelte:head>

<div class="narrow">
  <PageHead title="Account">
    {#snippet line()}Member since <span class="ui-num">{date(me.createdAt)}</span>{/snippet}
  </PageHead>

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

    <OrdersPanel />

    <SessionsPanel />

    {#if me.permissions.length}
      <section class="ui-panel">
        <header><h2>Permissions</h2></header>
        <p class="pad perms">{#each me.permissions as p}<span class="ui-chip">{p}</span>{/each}</p>
      </section>
    {/if}

    <DataPanel />

    <section class="ui-card">
      <h2>Sign out</h2>
      <p class="ui-hint">Ends this session on this device. Your sets and channels stay as they are.</p>
      <div class="ui-actions gap"><button class="ui-btn secondary" type="button" onclick={leave}>Sign out</button></div>
    </section>

    <DeleteAccount />
  </div>
</div>

<style>
  .what { display: grid; flex: 1; min-width: 0; }
  .what strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .blank { width: 28px; height: 28px; border-radius: 50%; border: 1px dashed var(--border-strong); flex: none; }
  .pad { padding: 10px 14px 12px; }
  .perms { display: flex; flex-wrap: wrap; gap: 6px; text-transform: none; }
  .perms .ui-chip { text-transform: none; letter-spacing: 0; font-family: var(--font-mono); font-weight: 600; }
  .gap { margin-top: 12px; }
</style>
