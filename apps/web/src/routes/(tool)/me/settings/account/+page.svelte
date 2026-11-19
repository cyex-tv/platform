<script lang="ts">
  // Settings → Account: the name you go by (with more than one connection, pick which login is
  // your name and picture on cyex.tv; every login still finds you), your connections (Disconnect
  // while more than one is linked; the last one always stays, and the API refuses it too),
  // Security (sign out everywhere, this device too; the sessions list for one device at a time),
  // your data, and deleting the account.
  import { gql, linkUrl } from '$lib/api';
  import { unlinkMessage } from '$lib/account/errors';
  import DataPanel from '$lib/account/DataPanel.svelte';
  import DeleteAccount from '$lib/account/DeleteAccount.svelte';
  import SessionsPanel from '$lib/account/SessionsPanel.svelte';
  import { canChooseName, canDisconnect } from '$lib/account/settings';
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Name from '$lib/components/Name.svelte';
  import { date } from '$lib/format';
  import type { Platform } from '$lib/gql/graphql';
  import { forgetProfile } from '$lib/profile/data.svelte';
  import { SET_DISPLAY_PLATFORM, SIGN_OUT_EVERYWHERE, UNLINK } from '$lib/queries';
  import { leftSession } from '$lib/shell/signout';
  import { forgetLooks } from '$lib/cosmetics/store.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loadMe, session } from '$lib/tool/session.svelte';

  const PLATFORMS: { id: Platform; name: string; slug: 'twitch' | 'kick' }[] = [
    { id: 'TWITCH', name: 'Twitch', slug: 'twitch' },
    { id: 'KICK', name: 'Kick', slug: 'kick' },
  ];
  const nameOf = (p: Platform) => PLATFORMS.find((x) => x.id === p)?.name ?? p;

  const me = $derived(session.me!);
  // The API lists the identity you go by first.
  const current = $derived(me.identities[0]?.platform ?? null);
  let busy = $state<string | null>(null);
  // The radio shows this; a save that fails puts it back on `current`.
  let picked = $derived<Platform | null>(current);

  async function refresh() {
    forgetProfile();
    forgetLooks();
    await loadMe({ fresh: true });
  }

  async function choose(p: Platform) {
    if (p === current || busy) return;
    busy = 'name';
    try {
      await gql(SET_DISPLAY_PLATFORM, { platform: p });
      await refresh();
      const id = me.identities.find((i) => i.platform === p);
      notify(`You go by your ${nameOf(p)} name${id ? `, ${id.displayName}` : ''}, now.`);
    } catch (err) {
      picked = current;
      fail(err);
    } finally {
      busy = null;
    }
  }

  async function disconnect(p: Platform) {
    busy = p;
    try {
      await gql(UNLINK, { platform: p });
      await refresh();
      notify(`${nameOf(p)} disconnected.`);
    } catch (err) {
      notify(unlinkMessage(err), true);
    } finally {
      busy = null;
    }
  }

  async function everywhere() {
    busy = 'all';
    try {
      await gql(SIGN_OUT_EVERYWHERE, {});
      await leftSession();
      notify('Signed out everywhere.');
    } catch (err) {
      fail(err);
      busy = null;
    }
  }
</script>

<svelte:head><title>Account settings - cyex.tv</title></svelte:head>

<div class="narrow">
  <PageHead title="Account">
    {#snippet line()}Member since <span class="ui-num">{date(me.createdAt)}</span>{/snippet}
  </PageHead>

  <div class="ui-stack">
    <section class="ui-panel" aria-labelledby="name-h">
      <header><h2 id="name-h">Display name</h2></header>
      {#if canChooseName(me.identities)}
        <fieldset class="choose" disabled={busy !== null}>
          <legend class="ui-hint pad">Pick which connection is your name and picture on cyex.tv. People still find you by every login.</legend>
          <ul class="ui-list">
            {#each me.identities as id (id.platform)}
              <li>
                <label class="pick">
                  <input type="radio" name="display" value={id.platform} bind:group={picked} onchange={() => choose(id.platform)} />
                  <Avatar src={id.avatar} name={id.displayName} size={28} tone="tool" />
                  <span class="what"><strong><Name value={id.displayName} /></strong><span class="ui-hint">{nameOf(id.platform)} · {id.login}</span></span>
                  {#if id.platform === current}<span class="now"><Icon name="check" size={14} />Your name</span>{/if}
                </label>
              </li>
            {/each}
          </ul>
        </fieldset>
      {:else if me.identities[0]}
        <p class="ui-hint pad">You go by your {nameOf(me.identities[0].platform)} name, <Name value={me.identities[0].displayName} />. Connect another platform below to choose between them.</p>
      {/if}
    </section>

    <section class="ui-panel" aria-labelledby="conn-h">
      <header><h2 id="conn-h">Connections</h2></header>
      <ul class="ui-list">
        {#each PLATFORMS as p (p.id)}
          {@const id = me.identities.find((i) => i.platform === p.id)}
          <li>
            {#if id}
              <Icon name={p.slug} />
              <span class="what"><strong><Name value={id.login} /></strong><span class="ui-hint">{p.name} · <span class="ui-mono">{id.platformUserId}</span></span></span>
              {#if canDisconnect(me.identities)}
                <Confirm small label="Disconnect" confirm="Click again to disconnect" busy={busy !== null} onconfirm={() => disconnect(p.id)} />
              {:else}
                <span class="ui-hint keep" title="You always keep at least one connection.">Your only connection</span>
              {/if}
            {:else}
              <Icon name={p.slug} />
              <span class="what"><strong>{p.name}</strong><span class="ui-hint">Not connected</span></span>
              <a class="ui-btn secondary small" href={linkUrl(p.slug)}>Connect {p.name}</a>
            {/if}
          </li>
        {/each}
      </ul>
      <p class="ui-hint pad">Connect both to sign in with either. You always keep at least one: the last connection can’t be disconnected. cyex.tv keeps the user ID, login, display name and picture.</p>
    </section>

    <section class="ui-panel" aria-labelledby="sec-h">
      <header><h2 id="sec-h">Security</h2></header>
      <div class="row pad">
        <p class="what"><strong>Sign out everywhere</strong><span class="ui-hint">Ends every session: this browser, other browsers and the cyex.tv apps. You sign in again after.</span></p>
        <Confirm small label="Sign out everywhere" confirm="Click again to sign out" busy={busy !== null} onconfirm={everywhere} />
      </div>
    </section>

    <SessionsPanel />

    <DataPanel />

    <DeleteAccount />
  </div>
</div>

<style>
  .what { display: grid; flex: 1; min-width: 0; margin: 0; }
  .what strong { font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pad { padding: 10px 14px 12px; margin: 0; }
  .choose { margin: 0; padding: 0; border: 0; min-width: 0; }
  .choose legend { display: block; width: 100%; }
  .pick { display: flex; align-items: center; gap: 12px; width: 100%; min-height: 44px; cursor: pointer; }
  .pick input { flex: none; width: 16px; height: 16px; margin: 0; accent-color: var(--accent); }
  .choose:disabled .pick { cursor: default; opacity: .7; }
  .now { display: inline-flex; align-items: center; gap: 4px; font-size: var(--t-fine); font-weight: 600; color: var(--text); white-space: nowrap; }
  .now :global(svg) { color: var(--accent-text); }
  li > :global(svg) { color: var(--muted); flex: none; }
  .keep { white-space: nowrap; }
  .row { display: flex; align-items: center; gap: 12px; }
  @media (max-width: 560px) {
    .row { flex-wrap: wrap; }
    .ui-list > li { flex-wrap: wrap; }
  }
</style>
