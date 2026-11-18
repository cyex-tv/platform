<script lang="ts">
  // The one profile page, for everyone (7TV's model: your profile is your dashboard). The server
  // renders the public page (Uploaded emotes); in the browser, the owner and the account's editors
  // also get Active emotes, Emote sets, Cosmetics (owner, or editors with PROFILE) and Activity
  // (when the API has it), plus edit controls. Sections are ?s= tabs in the ProfileRail.
  import { setContext } from 'svelte';
  import { invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import Icon from '$lib/components/Icon.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import { problemTitle } from '$lib/components/problem';
  import State from '$lib/components/State.svelte';
  import { useCopied } from '$lib/clipboard.svelte';
  import { clean } from '$lib/format';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import RailLayout from '$lib/shell/RailLayout.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { ACTIVITY_READY } from '$lib/profile/activity';
  import { relationState } from '$lib/profile/data.svelte';
  import ProfileRail from '$lib/profile/ProfileRail.svelte';
  import { canSets, sectionFrom, sectionsFor, type SectionKey } from '$lib/profile/rows';
  import ActiveEmotes from '$lib/profile/sections/ActiveEmotes.svelte';
  import ActivityFeed from '$lib/profile/sections/ActivityFeed.svelte';
  import CosmeticsPicker from '$lib/profile/sections/CosmeticsPicker.svelte';
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

  const TITLE: Record<SectionKey, string> = { uploads: 'Uploaded emotes', active: 'Active emotes', sets: 'Emote sets', cosmetics: 'Cosmetics', activity: 'Activity' };
  const link = useCopied();
</script>

{#snippet actions()}
  <button class="btn btn-s copy" type="button" onclick={() => link.copy(location.origin + page.url.pathname)}>
    <Icon name={link.copied ? 'check' : 'copy'} size={16} />
    <span class="labels">
      <span class:off={link.state !== 'idle'}>Copy page link</span>
      <span class:off={!link.copied} aria-hidden="true">Link copied</span>
      <span class:off={!link.failed} aria-hidden="true">Couldn’t copy</span>
    </span>
  </button>
  <span class="sr-only" role="status">{link.copied ? 'Link copied' : link.failed ? 'Couldn’t copy' : ''}</span>
{/snippet}

<svelte:head>
  <title>{data.creator ? `${section === 'uploads' ? name : `${TITLE[section]} · ${name}`} - cyex.tv` : data.error ? problemTitle('server') : 'Nothing here - cyex.tv'}</title>
  {#if name}<meta name="description" content="{name} on cyex.tv: emotes, sets and cosmetics." />{/if}
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
          <a class="ui-btn primary" href="/search">Browse emotes</a>
        {/snippet}
      </State>
    </div>
  </ColumnLayout>
{:else}
  {@const account = { id: data.id, displayName: data.creator.displayName, avatar: data.creator.avatar }}
  <RailLayout>
    {#snippet rail()}
      <ProfileRail {account} {relation} current={section} activity={ACTIVITY_READY} {actions} profile={data.profile} seed={data.look} />
    {/snippet}

    <h1 class="sr-only"><bdi>{name}</bdi></h1>
    {#if waiting}
      <State kind="loading" title="Loading…" />
    {:else if section === 'active'}
      {#key data.id}<ActiveEmotes accountId={data.id} mine={relation.mine} />{/key}
    {:else if section === 'sets'}
      {#key data.id}<SetCards accountId={data.id} mine={relation.mine} canCreate={canSets(relation)} {name} />{/key}
    {:else if section === 'cosmetics'}
      {#key data.id}<CosmeticsPicker accountId={data.id} mine={relation.mine} {name} />{/key}
    {:else if section === 'activity'}
      {#key data.id}<ActivityFeed accountId={data.id} />{/key}
    {:else if relation.mine}
      <MyUploads />
    {:else}
      {#key data.id}
        <Uploads id={data.id} {name} first={data.items} total={data.total} />
      {/key}
    {/if}
  </RailLayout>
{/if}

<style>
  .pad { padding-top: var(--rail-gap); }
  .labels { display: inline-grid; }
  .labels > span { grid-area: 1 / 1; text-align: left; }
  .off { visibility: hidden; }
</style>
