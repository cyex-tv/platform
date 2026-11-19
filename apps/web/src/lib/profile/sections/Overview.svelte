<script lang="ts">
  // Your profile's dashboard (Overview, ?s=overview; the account menu opens it): the crucial
  // things at a glance, each tile a short list that links on to the detailed tab. Channels and the
  // set each shows, notifications, your latest uploads with their state, the latest activity,
  // editors (names), cosmetics (what you show, how many you own) and your sets. Every tile loads on
  // its own and fails on its own (one tile's error never empties the page).
  import { onMount } from 'svelte';
  import { gql } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { loadOwned, looks, NO_LOOK } from '$lib/cosmetics/store.svelte';
  import { inbox, loadInbox } from '$lib/account/inbox.svelte';
  import NotificationRow from '$lib/account/NotificationRow.svelte';
  import { ready, src } from '$lib/emote';
  import { clean, count, dateTime } from '$lib/format';
  import type { MyEmotesQuery } from '$lib/gql/graphql';
  import { MY_EMOTES } from '$lib/queries';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { STATE_LABEL } from '$lib/tool/state';
  import { loadActivity, type ActivityItem } from '../activity';
  import ActivityRows from '../ActivityRows.svelte';
  import { editorsOf, libraryOf, type Channel, type Editor, type ProfileSet } from '../data.svelte';
  import { sectionHref } from '../rows';

  let { accountId }: { accountId: string } = $props();

  type Tile<T> = { data: T | null; failed: boolean };
  const tile = <T,>(): Tile<T> => ({ data: null, failed: false });

  let library = $state<Tile<{ channels: Channel[]; sets: ProfileSet[] }>>(tile());
  let uploads = $state<Tile<MyEmotesQuery['myEmotes']>>(tile());
  let activity = $state<Tile<ActivityItem[]>>(tile());
  let editors = $state<Tile<Editor[]>>(tile());
  let owned = $state<Tile<{ badges: number; paints: number }>>(tile());

  const look = $derived(looks[accountId] ?? NO_LOOK);
  const setName = (id: string | null | undefined) => library.data?.sets.find((s) => s.id === id)?.name ?? null;
  const latestNotes = $derived((inbox.items ?? []).slice(0, 4));

  function load<T>(t: Tile<T>, p: Promise<T>) {
    p.then((d) => {
      t.data = d;
    }).catch(() => {
      t.failed = true;
    });
  }
  onMount(() => {
    load(library, libraryOf(accountId, true));
    load(uploads, gql(MY_EMOTES, { page: 1, perPage: 6 }).then((r) => r.myEmotes));
    load(activity, loadActivity(accountId, null, fetch, {}).then((p) => p.items.slice(0, 5)));
    load(editors, editorsOf(accountId, true));
    load(owned, loadOwned(accountId, true).then((o) => ({ badges: o.badges.length, paints: o.paints.length })));
    if (!inbox.items && !inbox.loading) loadInbox();
  });
</script>

{#snippet waiting(t: { failed: boolean })}
  {#if t.failed}<p class="ui-hint pad">That didn’t load. Open the tab to try again.</p>
  {:else}<div class="pad" aria-busy="true"><span class="line ui-skeleton"></span><span class="line short ui-skeleton"></span></div>{/if}
{/snippet}

<PageHead title="Overview" line="Your channels, uploads, editors and what happened lately, at a glance." />

<div class="grid">
  <section class="ui-panel span6" aria-labelledby="ov-ch">
    <header>
      <Icon name="tv" />
      <h2 id="ov-ch">Channels</h2>
      <a class="more" href="/me/channels">Manage<Icon name="chevron-right" size={14} /></a>
    </header>
    {#if library.data}
      {#if library.data.channels.length}
        <ul class="ui-list">
          {#each library.data.channels as c (c.platform + c.platformChannelId)}
            {@const set = setName(c.activeSetId)}
            <li>
              <Icon name={c.platform === 'KICK' ? 'kick' : 'twitch'} />
              <span class="what"><strong><bdi>{clean(c.login)}</bdi></strong>
                <span class="ui-hint">{#if set}Shows <a href="/sets/{encodeURIComponent(c.activeSetId ?? '')}"><bdi>{clean(set)}</bdi></a>{:else}No active set{/if}</span></span>
            </li>
          {/each}
        </ul>
      {:else}<p class="ui-hint pad">No channels yet. Link Twitch or Kick in Settings to add yours.</p>{/if}
    {:else}{@render waiting(library)}{/if}
  </section>

  <section class="ui-panel span6" aria-labelledby="ov-no">
    <header>
      <Icon name="bell" />
      <h2 id="ov-no">Notifications{#if inbox.count}<span class="n ui-num">{count(inbox.count)} unread</span>{/if}</h2>
      <a class="more" href="/me/notifications">All<Icon name="chevron-right" size={14} /></a>
    </header>
    {#if inbox.items}
      {#if latestNotes.length}
        <ul class="notes">{#each latestNotes as n (n.id)}<li><NotificationRow note={n} /></li>{/each}</ul>
      {:else}<p class="ui-hint pad">Nothing new. Reviews, editor changes and cosmetics you get show up here.</p>{/if}
    {:else}{@render waiting({ failed: !!inbox.error })}{/if}
  </section>

  <section class="ui-panel span6" aria-labelledby="ov-up">
    <header>
      <Icon name="upload" />
      <h2 id="ov-up">Recent uploads{#if uploads.data}<span class="n ui-num">{count(uploads.data.total)}</span>{/if}</h2>
      <a class="more" href={sectionHref(accountId, 'uploads')}>All emotes<Icon name="chevron-right" size={14} /></a>
    </header>
    {#if uploads.data}
      {#if uploads.data.items.length}
        <ul class="ui-list">
          {#each uploads.data.items as e (e.id)}
            {@const pic = { host: e.host, animated: e.animated, files: e.files }}
            <li>
              <span class="thumb">{#if ready(pic)}<img src={src(pic, 1)} alt="" loading="lazy" />{/if}</span>
              <a class="what" href="/me/emotes/{encodeURIComponent(e.id)}"><strong><bdi>{clean(e.name)}</bdi></strong>
                <span class="ui-hint ui-num">{dateTime(e.createdAt)}</span></a>
              <span class="ui-chip {e.state.toLowerCase()}">{STATE_LABEL[e.state]}</span>
            </li>
          {/each}
        </ul>
      {:else}<p class="ui-hint pad">No uploads yet. <a href="/me/upload">Upload your first emote</a>.</p>{/if}
    {:else}{@render waiting(uploads)}{/if}
  </section>

  <section class="ui-panel span6" aria-labelledby="ov-ac">
    <header>
      <Icon name="activity" />
      <h2 id="ov-ac">Recent activity</h2>
      <a class="more" href={sectionHref(accountId, 'activity')}>See all<Icon name="chevron-right" size={14} /></a>
    </header>
    {#if activity.data}
      {#if activity.data.length}<div class="feed"><ActivityRows items={activity.data} subject={accountId} /></div>
      {:else}<p class="ui-hint pad">Nothing yet. Changes to your sets, emotes, cosmetics and editors show up here.</p>{/if}
    {:else}{@render waiting(activity)}{/if}
  </section>

  <section class="ui-panel span4" aria-labelledby="ov-ed">
    <header>
      <Icon name="circle-user" />
      <h2 id="ov-ed">Editors{#if editors.data}<span class="n ui-num">{count(editors.data.length)}</span>{/if}</h2>
      <a class="more" href="/me/editors">Manage<Icon name="chevron-right" size={14} /></a>
    </header>
    {#if editors.data}
      {#if editors.data.length}
        <ul class="ui-list">
          {#each editors.data.slice(0, 5) as ed (ed.account.id)}
            <li><a class="what" href="/users/{encodeURIComponent(ed.account.id)}"><PaintedName name={ed.account.displayName} paint={looks[ed.account.id]?.paint} badge={looks[ed.account.id]?.badge} /></a></li>
          {/each}
        </ul>
      {:else}<p class="ui-hint pad">Nobody yet. Editors can manage your sets and emotes for you.</p>{/if}
    {:else}{@render waiting(editors)}{/if}
  </section>

  <section class="ui-panel span4" aria-labelledby="ov-co">
    <header>
      <Icon name="brush" />
      <h2 id="ov-co">Cosmetics</h2>
      <a class="more" href={sectionHref(accountId, 'cosmetics')}>Change<Icon name="chevron-right" size={14} /></a>
    </header>
    {#if owned.data}
      <dl class="facts">
        <div><dt>Paint</dt><dd>{#if look.paint}<PaintedName name={look.paint.name} paint={look.paint} />{:else}None{/if}</dd></div>
        <div><dt>Badge</dt><dd>{look.badge ? clean(look.badge.name) : 'None'}</dd></div>
        <div><dt>Owned</dt><dd class="ui-num">{count(owned.data.paints)} paints, {count(owned.data.badges)} badges</dd></div>
      </dl>
    {:else}{@render waiting(owned)}{/if}
  </section>

  <section class="ui-panel span4" aria-labelledby="ov-se">
    <header>
      <Icon name="folder" />
      <h2 id="ov-se">Emote sets{#if library.data}<span class="n ui-num">{count(library.data.sets.length)}</span>{/if}</h2>
      <a class="more" href={sectionHref(accountId, 'sets')}>Sets<Icon name="chevron-right" size={14} /></a>
    </header>
    {#if library.data}
      {#if library.data.sets.length}
        <ul class="ui-list">
          {#each library.data.sets.slice(0, 5) as s (s.id)}
            <li><a class="what" href="/sets/{encodeURIComponent(s.id)}"><strong><bdi>{clean(s.name)}</bdi></strong></a></li>
          {/each}
        </ul>
      {:else}<p class="ui-hint pad">No sets yet.</p>{/if}
    {:else}{@render waiting(library)}{/if}
  </section>
</div>

<style>
  .grid { display: grid; grid-template-columns: repeat(12, minmax(0, 1fr)); gap: 16px; align-items: start; }
  .span6 { grid-column: span 6; }
  .span4 { grid-column: span 4; }
  @media (max-width: 1199.98px) { .span4 { grid-column: span 6; } .span4:last-child { grid-column: span 12; } }
  @media (max-width: 760px) { .span6, .span4, .span4:last-child { grid-column: span 12; } }
  /* A tile's head: icon, title with its number, and where it leads. */
  .ui-panel > header > :global(svg) { color: var(--dim); flex: none; }
  .ui-panel > header h2 { display: flex; align-items: baseline; gap: 8px; margin: 0; font-weight: 600; }
  .n { font-size: 12px; font-weight: 500; color: var(--dim); }
  .more { display: inline-flex; align-items: center; gap: 2px; min-height: 32px; padding: 0 4px 0 8px; border-radius: var(--radius);
    font-size: var(--t-fine); font-weight: 500; color: var(--muted); white-space: nowrap; }
  @media (hover: hover) and (pointer: fine) { .more:hover { color: var(--text); background: var(--row-hover); } }
  .what { display: grid; flex: 1; min-width: 0; color: var(--text); }
  .what strong { font-size: var(--t-ui); font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .what .ui-hint a, .pad a { color: var(--text); text-decoration: underline; text-decoration-color: var(--border-strong); text-underline-offset: 3px; }
  .ui-list > li { font-size: var(--t-ui); }
  .ui-list > li > :global(svg) { color: var(--muted); flex: none; }
  .thumb { flex: none; display: grid; place-items: center; width: 32px; height: 32px; }
  .thumb img { max-width: 32px; max-height: 32px; width: auto; height: auto; }
  .pad { margin: 0; padding: 12px 16px 14px; }
  .line { display: block; height: 12px; width: 70%; border-radius: var(--radius); }
  .line + .line { margin-top: 8px; }
  .line.short { width: 40%; }
  .notes { list-style: none; margin: 0; padding: 4px 0; }
  .feed { padding: 0 12px; }
  .feed :global(.feed) { border-top: 0; }
  .feed :global(.item:last-child) { border-bottom: 0; }
  .facts { margin: 0; padding: 8px 16px 12px; display: grid; gap: 8px; }
  .facts div { display: flex; gap: 12px; align-items: baseline; }
  .facts dt { width: 64px; flex: none; font-size: var(--t-fine); color: var(--muted); }
  .facts dd { margin: 0; min-width: 0; font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
