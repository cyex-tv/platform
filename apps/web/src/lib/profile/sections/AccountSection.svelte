<script lang="ts">
  // Your profile → Account (?s=account, owner only; it was Settings → Account until 2026/11/22):
  // the name you go by (with more than one connection, pick which login is your name and picture
  // on cyex.tv; every login still finds you), your connections (Disconnect while more than one is
  // linked; the last one always stays, and the API refuses it too), Sign out everywhere (the one
  // session action: every browser and app, this one too; the account menu's Sign out ends only
  // this one), your API token (its expiry, and a new one shown with Copy), Developers (your
  // read-only API keys with their usage, ADR 0033), your data, and
  // deleting the account. Two flat columns on wide screens.
  import { gql, linkUrl } from '$lib/api';
  import { unlinkMessage } from '$lib/account/errors';
  import ApiTokenPanel from '$lib/account/ApiTokenPanel.svelte';
  import DataPanel from '$lib/account/DataPanel.svelte';
  import DeveloperKeysPanel from '$lib/account/DeveloperKeysPanel.svelte';
  import DeleteAccount from '$lib/account/DeleteAccount.svelte';
  import { canChooseName, canDisconnect } from '$lib/account/settings';
  import Avatar from '$lib/components/Avatar.svelte';
  import Row from '$lib/components/Row.svelte';
  import Section from '$lib/components/Section.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import type { Platform } from '$lib/gql/graphql';
  import { forgetProfile } from '$lib/profile/data.svelte';
  import { sectionHref } from '$lib/profile/rows';
  import { SET_DISPLAY_PLATFORM, SIGN_OUT_EVERYWHERE, UNLINK } from '$lib/queries';
  import { leftSession } from '$lib/shell/signout';
  import { forgetLooks } from '$lib/cosmetics/store.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { useToolButtons } from '$lib/components/btn';
  import { loadMe, session } from '$lib/tool/session.svelte';

  useToolButtons(); // the account controls are tool buttons, as in Settings

  const PLATFORMS: { id: Platform; name: string; slug: 'twitch' | 'kick' }[] = [
    { id: 'TWITCH', name: 'Twitch', slug: 'twitch' },
    { id: 'KICK', name: 'Kick', slug: 'kick' },
  ];
  const nameOf = (p: Platform) => PLATFORMS.find((x) => x.id === p)?.name ?? p;

  // The last account seen: signing out everywhere forgets the session before the page goes, and
  // the tab must not break in that moment.
  let kept = session.me!;
  const me = $derived.by(() => (kept = session.me ?? kept));
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

<PageHead title="Account" line="The name you go by, the platforms you sign in with, and your data. Only you see this tab." />

<div class="cq"><div class="cols">
  <div class="col">
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
          <a class="ui-btn secondary small" href={linkUrl(p.slug, sectionHref(me.id, 'account'))}>Connect {p.name}</a>
        </Row>
      {/if}
    {/each}
  </Section>

  </div>
  <div class="col">
  <Section title="Sign out" id="signout-h">
    <Row label="Sign out everywhere" hint="Ends every session: this browser, other browsers and the cyex.tv apps. You sign in again after. To sign out of this browser only, use Sign out in the account menu.">
      <Confirm small label="Sign out everywhere" confirm="Click again to sign out" busy={busy !== null} onconfirm={everywhere} />
    </Row>
  </Section>

  <ApiTokenPanel />

  <DeveloperKeysPanel />

  <DataPanel />

  <DeleteAccount />
  </div>
</div></div>

<style>
  /* Two flat columns from 1000px of content (who you are | sign out, data, delete), one below. */
  .cq { container-type: inline-size; }
  .cols { display: grid; grid-template-columns: minmax(0, 1fr); }
  .col { min-width: 0; }
  .col + .col > :global(.sec:first-child) { border-top: 1px solid var(--border); }
  @container (min-width: 1000px) {
    /* The hairline sits in the middle of the gutter: 24px of air on each side of it. */
    .cols { grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); column-gap: 0; }
    .col:first-child { padding-right: 24px; }
    .col + .col { padding-left: 24px; border-left: 1px solid var(--border); }
    .col + .col > :global(.sec:first-child) { border-top: 0; }
  }
  .keep { font-size: var(--t-fine); color: var(--muted); }
</style>
