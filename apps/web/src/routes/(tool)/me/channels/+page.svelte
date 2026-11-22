<script lang="ts">
  // Channels: your channel on each platform you signed in with, and the channels of accounts you
  // edit, each with the set its chat shows and whether chatters' personal emotes show. In your
  // profile's frame (MeFrame); the panel sums them up and links to what they depend on.
  import Icon from '$lib/components/Icon.svelte';
  import FactRows from '$lib/components/FactRows.svelte';
  import LinkRows from '$lib/components/LinkRows.svelte';
  import PanelBlock from '$lib/components/PanelBlock.svelte';
  import Row from '$lib/components/Row.svelte';
  import Section from '$lib/components/Section.svelte';
  import MeFrame from '$lib/profile/MeFrame.svelte';
  import { count } from '$lib/format';
  import { sectionHref } from '$lib/profile/rows';
  import Name from '$lib/components/Name.svelte';
  import { clean, iso } from '$lib/format';
  import { ownerSetsHref, permissionLabels } from '$lib/tool/editors';
  import { gql } from '$lib/api';
  import type { MyChannelsQuery } from '$lib/gql/graphql';
  import { MY_CHANNELS, UPDATE_CHANNEL } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import State from '$lib/components/State.svelte';

  type Channel = MyChannelsQuery['myChannels'][number];
  const L = loader(({ fetch }) => gql(MY_CHANNELS, {}, fetch));
  const sets = $derived(L.data?.mySets ?? []);
  /** Channels with a save on its way: one at a time per channel, however fast the controls are used. */
  let saving = $state(new Set<string>());

  const channelSets = $derived(sets.filter((s) => s.kind === 'CHANNEL' && s.owner?.id === session.me?.id));
  // Account editing (ADR 0014): channels of accounts you edit with the channels permission (which
  // the sets permission brings), by owner.
  const managed = $derived(
    (L.data?.editorOf ?? [])
      .filter((g) => g.permissions.includes('CHANNELS') || g.permissions.includes('SETS'))
      .map((g) => ({
        ownerId: g.account.id,
        permissions: g.permissions,
        channels: (L.data?.editorChannels ?? []).filter((c) => c.account.id === g.account.id),
        sets: (L.data?.editorSets ?? []).filter((s) => s.kind === 'CHANNEL' && s.owner?.id === g.account.id),
      }))
      .filter((g) => g.channels.length),
  );

  async function update(ch: Channel, input: { activeSetId?: string; allowPersonalEmotes?: boolean }, revert: () => void) {
    const key = `${ch.platform}:${ch.platformChannelId}`;
    if (saving.has(key)) return revert();
    saving = new Set([...saving, key]);
    try {
      const r = await gql(UPDATE_CHANNEL, { platform: ch.platform, id: ch.platformChannelId, input });
      ch.activeSetId = r.updateChannelSettings.activeSetId;
      ch.allowPersonalEmotes = r.updateChannelSettings.allowPersonalEmotes;
      notify('Saved. Chat picks it up within seconds.');
    } catch (err) {
      // The control goes back to what is saved, not the value that failed.
      revert();
      fail(err);
    } finally {
      const left = new Set(saving);
      left.delete(key);
      saving = left;
    }
  }

  const platformName = (p: string) => (p === 'TWITCH' ? 'Twitch' : 'Kick');
  const own = $derived(L.data?.myChannels ?? []);
  const setName = (id: string | null | undefined) => (id ? [...sets, ...(L.data?.editorSets ?? [])].find((s) => s.id === id)?.name : undefined);
  const showing = $derived([...own, ...managed.flatMap((g) => g.channels)]);
</script>

<svelte:head><title>Channels - cyex.tv</title></svelte:head>

{#snippet block(ch: Channel, options: { id: string; name: string }[], owner?: string)}
  {@const busy = saving.has(`${ch.platform}:${ch.platformChannelId}`)}
  {@const key = `${ch.platform}-${ch.platformChannelId}`.toLowerCase()}
  <div class="ch" data-platform={ch.platform.toLowerCase()}>
    <div class="ch-head">
      <span class="dot" aria-hidden="true"></span>
      <h3><Name value={ch.login} /></h3>
      <span class="plat" title={owner ? `${platformName(ch.platform)} · ${clean(owner)}’s channel` : undefined}>{platformName(ch.platform)}{#if owner}{' '}· <bdi>{clean(owner)}</bdi>’s channel{/if}</span>
      <a class="ch-link" href="/channels/{ch.platform.toLowerCase()}/{encodeURIComponent(ch.login)}">Public page<Icon name="chevron-right" size={14} /></a>
    </div>
    <Row label="Active set" for="set-{key}" hint="What everyone in {owner ? 'its' : 'your'} chat sees.">
      <select id="set-{key}" class="ui-input pick" aria-disabled={busy} value={ch.activeSetId ?? ''}
        onchange={(e) => { const el = e.currentTarget; update(ch, { activeSetId: el.value }, () => (el.value = ch.activeSetId ?? '')); }}>
        <option value="">None</option>
        {#each options as s}<option value={s.id}>{iso(s.name)}</option>{/each}
      </select>
    </Row>
    {#if options.length === 0}
      <p class="ui-hint none">{owner ? `${iso(owner)} has` : 'You have'} no channel sets yet. <a href={owner ? ownerSetsHref(ch.account.id) : '/me/sets'}>Make one</a>.</p>
    {/if}
    <Row label="Personal emotes" for="pe-{key}" inline hint="Show chatters’ personal sets in {owner ? 'its' : 'your'} chat.">
      <input id="pe-{key}" class="ui-switch" type="checkbox" aria-disabled={busy} checked={ch.allowPersonalEmotes}
        onchange={(e) => { const el = e.currentTarget; update(ch, { allowPersonalEmotes: el.checked }, () => (el.checked = ch.allowPersonalEmotes)); }} />
    </Row>
  </div>
{/snippet}

{#snippet side()}
  <PanelBlock title="In numbers">
    <FactRows items={[
      { label: 'Your channels', value: L.data ? count(own.length) : '–', mono: true },
      { label: 'Channels you edit', value: L.data ? count(managed.reduce((n, g) => n + g.channels.length, 0)) : '–', mono: true },
      { label: 'Your channel sets', value: L.data ? count(channelSets.length) : '–', mono: true },
    ]} />
  </PanelBlock>
  {#if showing.length}
    <PanelBlock title="Chat shows">
      <FactRows items={showing.map((ch) => ({ label: clean(ch.login), key: ch.platform + ch.platformChannelId }))}>
        {#snippet value(k)}
          {@const ch = showing.find((c) => c.platform + c.platformChannelId === k)}
          {@const set = ch ? setName(ch.activeSetId) : undefined}
          {#if ch && set && ch.activeSetId}<a href="/sets/{encodeURIComponent(ch.activeSetId)}"><bdi>{clean(set)}</bdi></a>{:else}<span class="none-set">No set</span>{/if}
        {/snippet}
      </FactRows>
    </PanelBlock>
  {/if}
  <PanelBlock title="Related">
    <LinkRows items={[
      { href: '/me/account', label: 'Connections', hint: 'Connect another platform to add its channel', icon: 'link' },
      { href: sectionHref(session.me?.id ?? '', 'sets'), label: 'Your sets', icon: 'folder' },
      { href: '/me/editors', label: 'Editors', icon: 'circle-user' },
    ]} />
    <p class="pb-hint">A change reaches chat within seconds.</p>
  </PanelBlock>
{/snippet}

{#snippet head()}
  <PageHead title="Channels" line="Your channel on each platform you signed in with. Pick the set its chat shows." />
{/snippet}

<MeFrame title="Channels" label="About your channels" panel={side} {head}>
  <LoadGate source={L} label="Loading your channels…">
  {#snippet children(data)}
  {@const channels = data.myChannels}
  {#if channels.length === 0 && managed.length === 0}
    <State title="No channels yet">Sign in with Twitch or Kick and yours appears here.</State>
  {:else}
    {#if channels.length}
      <Section title="Your channels" count={count(channels.length)}>
        {#each channels as ch (ch.platform + ch.platformChannelId)}
          {@render block(ch, channelSets)}
        {/each}
      </Section>
    {/if}
    {#each managed as g (g.ownerId)}
      <Section title="{clean(g.channels[0].account.displayName)} · Editor" id="of-{g.ownerId}" lead="You edit this account: {permissionLabels(g.permissions)}">
        {#each g.channels as ch (ch.platform + ch.platformChannelId)}
          {@render block(ch, g.sets, g.channels[0].account.displayName)}
        {/each}
      </Section>
    {/each}
  {/if}
  {/snippet}
  </LoadGate>
</MeFrame>

<style>
  /* Busy keeps focus (a disabled control would drop it): dim it instead. */
  [aria-disabled='true'] { opacity: .55; cursor: progress; }
  .ch + .ch { margin-top: 20px; }
  .ch-head { display: flex; align-items: center; gap: 10px; min-height: 40px; margin-top: 8px; border-bottom: 1px solid var(--border); }
  .ch-head h3 { flex: 0 1 auto; min-width: 0; margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--t-ui); font-weight: 600; }
  .plat { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--t-fine); color: var(--muted); }
  .ch-link { display: inline-flex; align-items: center; gap: 2px; flex: none; min-height: 32px; font-size: var(--t-fine); font-weight: 500; color: var(--muted); white-space: nowrap; }
  @media (hover: hover) and (pointer: fine) { .ch-link:hover { color: var(--text); } }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--twitch-logo); flex: none; }
  [data-platform='kick'] .dot { background: var(--kick); }
  .pick { width: auto; min-width: 0; max-width: min(100%, 260px); text-overflow: ellipsis; }
  .none { margin: 0; padding: 8px 0; border-bottom: 1px solid var(--border); }
  .none a { color: var(--text); text-decoration: underline; text-decoration-color: var(--border-strong); text-underline-offset: 3px; }
  .none-set { color: var(--muted); }
</style>
