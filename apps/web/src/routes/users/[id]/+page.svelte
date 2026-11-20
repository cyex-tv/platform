<script lang="ts">
  // The one profile page, for everyone. The identity band (ProfileHeader: name, roles as flags,
  // and the connections and editors behind count chips) heads it; the ProfileRail beside the
  // content is navigation, the same public rows for every viewer. The server renders the public
  // page (Uploaded emotes; Emote sets, Cosmetics and Activity are a tab away). In the browser the
  // owner also opens Overview (?s=overview, the dashboard: the account menu and the band's
  // Dashboard button), the owner and the account's editors Active emotes (from the dashboard, or
  // the band's button for editors), and the Cosmetics tab becomes the picker for the owner and
  // editors with PROFILE. Sections are ?s= tabs; the old ?s=collection opens Cosmetics.
  import { setContext } from 'svelte';
  import { invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import Problem from '$lib/components/Problem.svelte';
  import { problemTitle } from '$lib/components/problem';
  import State from '$lib/components/State.svelte';
  import { clean } from '$lib/format';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import RailLayout from '$lib/shell/RailLayout.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { ACTIVITY_READY } from '$lib/profile/activity';
  import { relationState } from '$lib/profile/data.svelte';
  import ProfileHeader from '$lib/profile/ProfileHeader.svelte';
  import ProfileRail from '$lib/profile/ProfileRail.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import AdminLink from '$lib/profile/AdminLink.svelte';
  import Overview from '$lib/profile/sections/Overview.svelte';
  import { canProfile, canSets, manages, sectionFrom, sectionHref, sectionsFor, type SectionKey } from '$lib/profile/rows';
  import ActiveEmotes from '$lib/profile/sections/ActiveEmotes.svelte';
  import ActivityFeed from '$lib/profile/sections/ActivityFeed.svelte';
  import CosmeticsPicker from '$lib/profile/sections/CosmeticsPicker.svelte';
  import ShownCosmetics from '$lib/profile/sections/ShownCosmetics.svelte';
  import Favourites from '$lib/profile/sections/Favourites.svelte';
  import MyUploads from '$lib/profile/sections/MyUploads.svelte';
  import SetCards from '$lib/profile/sections/SetCards.svelte';
  import Uploads from './Uploads.svelte';

  let { data } = $props();
  // The page's h1 is the person's name (below); the sections are h2s.
  setContext('heading-level', 2);
  const name = $derived(clean(data.creator?.displayName ?? ''));
  const rel = relationState(() => data.id);
  const relation = $derived(rel.value);
  const allowed = $derived(sectionsFor(relation, ACTIVITY_READY));
  const asked = $derived(page.url.searchParams.get('s'));
  const section = $derived<SectionKey>(sectionFrom(page.url.searchParams, allowed));
  // Grants load after the session; give them a moment before falling back to Uploaded emotes.
  let granted = $state(false);
  $effect(() => {
    void data.id;
    granted = false;
    const t = setTimeout(() => (granted = true), 1500);
    return () => clearTimeout(t);
  });
  // A section only the owner or an editor has, asked for before the session is known: wait for it.
  const waiting = $derived(!!asked && asked !== section && (!session.known || (!!session.me && !granted)));

  const TITLE: Record<SectionKey, string> = { overview: 'Overview', uploads: 'Uploaded emotes', active: 'Active emotes', sets: 'Emote sets', favourites: 'Favourites', cosmetics: 'Cosmetics', activity: 'Activity' };
</script>

{#snippet actions()}
  {#if relation.mine}
    <a class="btn btn-s" href={sectionHref(data.id, 'overview')} aria-current={section === 'overview' ? 'page' : undefined}><Icon name="gauge" size={15} />Dashboard</a>
    <a class="btn btn-s" href="/me/settings"><Icon name="settings" size={15} />Settings</a>
  {:else if manages(relation)}
    <a class="btn btn-s" href={sectionHref(data.id, 'active')} aria-current={section === 'active' ? 'page' : undefined}><Icon name="zap" size={15} />Active emotes</a>
  {/if}
{/snippet}

<svelte:head>
  <title>{data.creator ? `${section === 'uploads' ? name : `${TITLE[section]} · ${name}`} - cyex.tv` : data.error ? problemTitle('server') : 'Nothing here - cyex.tv'}</title>
</svelte:head>

{#if data.error}
  <ColumnLayout>
    <div class="pad">
      <Problem cause="server" level={1} onretry={() => invalidateAll()} />
    </div>
  </ColumnLayout>
{:else if !data.creator}
  <ColumnLayout>
    <div class="pad">
      <State level={1} role="status" title="No profile here.">
        There is no account at this address, or its owner has deleted it. Check the link, or look through what
        everyone has made.
        {#snippet actions()}
          <a class="btn btn-s btn-primary" href="/search">Browse emotes</a>
        {/snippet}
      </State>
    </div>
  </ColumnLayout>
{:else}
  {@const account = { id: data.id, displayName: data.creator.displayName, avatar: data.creator.avatar }}
  <RailLayout>
    {#snippet top()}
      <ProfileHeader {account} {relation} profile={data.profile} seed={data.look} {actions} />
    {/snippet}
    {#snippet rail()}
      <ProfileRail {account} current={section} activity={ACTIVITY_READY} />
    {/snippet}

    {#if waiting}
      <State kind="loading" title="Loading…" />
    {:else if section === 'overview'}
      {#key data.id}<Overview accountId={data.id} />{/key}
    {:else if section === 'active'}
      {#key data.id}<ActiveEmotes accountId={data.id} mine={relation.mine} />{/key}
    {:else if section === 'sets'}
      {#key data.id}<SetCards accountId={data.id} mine={relation.mine} canCreate={canSets(relation)} {name} />{/key}
    {:else if section === 'favourites'}
      <Favourites />
    {:else if section === 'cosmetics' && canProfile(relation)}
      {#key data.id}<CosmeticsPicker accountId={data.id} mine={relation.mine} {name} />{/key}
    {:else if section === 'cosmetics'}
      {#key data.id}<ShownCosmetics accountId={data.id} {name} seed={data.look} />{/key}
    {:else if section === 'activity'}
      {#key data.id}<ActivityFeed accountId={data.id} name={data.creator.displayName} />{/key}
    {:else if relation.mine}
      <MyUploads />
    {:else if relation.perms.has('EMOTE_MANAGE')}
      {#key data.id}<MyUploads accountId={data.id} name={data.creator.displayName} canUpload={relation.perms.has('EMOTE_UPLOAD')} />{/key}
    {:else}
      {#key data.id}
        <Uploads id={data.id} {name} first={data.items} total={data.total} />
      {/key}
    {/if}
    <AdminLink accountId={data.id} />
  </RailLayout>
{/if}

<style>
  .pad { padding-top: var(--rail-gap); }
</style>
