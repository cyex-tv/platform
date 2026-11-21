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
  import Row from '$lib/components/Row.svelte';
  import Section from '$lib/components/Section.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
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

  async function refresh() {
    forgetProfile();
    forgetLooks();
    await loadMe({ fresh: true });
  }

  const currentIdentity = $derived(me.identities[0] ?? null);

  async function choose(p: Platform) {
    if (p === current || busy) return;
    busy = 'name';
    try {
      await gql(SET_DISPLAY_PLATFORM, { platform: p });
      await refresh();
      const id = me.identities.find((i) => i.platform === p);
      notify(`You go by your ${nameOf(p)} name${id ? `, ${id.displayName}` : ''}, now.`);
    } catch (err) {
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

  <Section title="Display name" lead={canChooseName(me.identities) ? 'Pick which connection is your name and picture on cyex.tv. People still find you by every login.' : undefined}>
    {#if currentIdentity}
      <Row label={currentIdentity.displayName} hint={canChooseName(me.identities) ? `${nameOf(currentIdentity.platform)} · ${currentIdentity.login}` : `Your ${nameOf(currentIdentity.platform)} name. Connect another platform below to choose between them.`}>
        {#snippet media()}<Avatar src={currentIdentity.avatar} name={currentIdentity.displayName} size={24} tone="tool" />{/snippet}
        {#if canChooseName(me.identities)}
          <Segmented label="Display name" options={me.identities.map((i) => ({ value: i.platform, label: nameOf(i.platform), disabled: busy !== null }))} value={current ?? ''} onchange={(v) => choose(v as Platform)} />
        {/if}
      </Row>
    {/if}
  </Section>

  <Section title="Connections" count={me.identities.length} lead="Connect both to sign in with either. You always keep at least one: the last connection can’t be disconnected. cyex.tv keeps the user ID, login, display name and picture.">
    {#each PLATFORMS as p (p.id)}
      {@const id = me.identities.find((i) => i.platform === p.id)}
      {#if id}
        <Row label={id.login} icon={p.slug}>
          {#snippet hint()}{p.name} · <span class="ui-mono">{id.platformUserId}</span>{/snippet}
          {#if canDisconnect(me.identities)}
            <Confirm small label="Disconnect" confirm="Click again to disconnect" busy={busy !== null} onconfirm={() => disconnect(p.id)} />
          {:else}
            <span class="keep" title="You always keep at least one connection.">Your only connection</span>
          {/if}
        </Row>
      {:else}
        <Row label={p.name} hint="Not connected" icon={p.slug} dim>
          <a class="ui-btn secondary small" href={linkUrl(p.slug)}>Connect {p.name}</a>
        </Row>
      {/if}
    {/each}
  </Section>

  <Section title="Security">
    <Row label="Sign out everywhere" hint="Ends every session: this browser, other browsers and the cyex.tv apps. You sign in again after.">
      <Confirm small label="Sign out everywhere" confirm="Click again to sign out" busy={busy !== null} onconfirm={everywhere} />
    </Row>
  </Section>

  <SessionsPanel />

  <DataPanel />

  <DeleteAccount />
</div>

<style>
  .keep { font-size: var(--t-fine); color: var(--muted); }
</style>
