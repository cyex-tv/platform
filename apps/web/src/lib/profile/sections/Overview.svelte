<script lang="ts">
  // Your profile's dashboard (Overview, ?s=overview; the account menu and the band's Dashboard
  // button open it, never the rail, which is public). It links on to what only you open: Active
  // emotes, Favourites, Channels and Notifications. The crucial
  // things at a glance, each tile a short list that links on to the detailed tab. Channels and the
  // set each shows, notifications, your latest uploads with their state, the latest activity,
  // editors (names), cosmetics (what you show, how many you own) and your sets. Every tile loads on
  // its own and fails on its own (one tile's error never empties the page).
  // Laid out as a dashboard for the full-width site (2026/11/22): a strip of numbers on top (each a
  // link to its tab), then the tiles in balanced columns (CSS columns: as many 360px columns as
  // fit, at most four, hairlines between), so no tile leaves a gap beside a taller one.
  import { onMount } from 'svelte';
  import { gql } from '$lib/api';
  import FactRows from '$lib/components/FactRows.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Section from '$lib/components/Section.svelte';
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

{#snippet more(href: string, label: string)}<a class="more" {href}>{label}<Icon name="chevron-right" size={14} /></a>{/snippet}

<PageHead title="Overview" line="Your channels, uploads, editors and what happened lately, at a glance. Only you see this page.">
  {#snippet actions()}
    <a class="btn btn-s" href={sectionHref(accountId, 'active')}><Icon name="zap" size={15} />Active emotes</a>
    <a class="btn btn-s" href={sectionHref(accountId, 'favourites')}><Icon name="heart" size={15} />Favourites</a>
  {/snippet}
</PageHead>

{#snippet stat(label: string, value: number | null | undefined, href: string, failed = false)}
  <li><a {href}><span class="st-l">{label}</span><span class="st-n ui-mono">{#if value != null}{count(value)}{:else if failed}–{:else}<span class="st-sk ui-skeleton" aria-hidden="true"></span><span class="sr-only">Loading</span>{/if}</span></a></li>
{/snippet}

<ul class="stats" aria-label="In numbers">
  {@render stat('Uploads', uploads.data?.total, sectionHref(accountId, 'uploads'), uploads.failed)}
  {@render stat('Channels', library.data?.channels.length, '/me/channels', library.failed)}
  {@render stat('Emote sets', library.data?.sets.length, sectionHref(accountId, 'sets'), library.failed)}
  {@render stat('Editors', editors.data?.length, sectionHref(accountId, 'editors'), editors.failed)}
  {@render stat('Unread', inbox.items ? inbox.count : null, '/me/notifications', !!inbox.error)}
  {@render stat('Paints owned', owned.data?.paints, sectionHref(accountId, 'cosmetics'), owned.failed)}
</ul>

<div class="tiles">
    <Section title="Channels" count={library.data ? count(library.data.channels.length) : undefined} id="ov-ch">
      {#snippet actions()}{@render more('/me/channels', 'Manage')}{/snippet}
      {#if library.data}
        {#if library.data.channels.length}
          <ul class="rows">
            {#each library.data.channels as c (c.platform + c.platformChannelId)}
              {@const set = setName(c.activeSetId)}
              <li>
                <Icon name={c.platform === 'KICK' ? 'kick' : 'twitch'} size={16} />
                <strong class="nm"><bdi>{clean(c.login)}</bdi></strong>
                <span class="sub">{#if set}<a href="/sets/{encodeURIComponent(c.activeSetId ?? '')}"><bdi>{clean(set)}</bdi></a>{:else}No active set{/if}</span>
              </li>
            {/each}
          </ul>
        {:else}<p class="ui-hint pad">No channels yet. Link Twitch or Kick under Account to add yours.</p>{/if}
      {:else}{@render waiting(library)}{/if}
    </Section>

    <Section title="Recent uploads" count={uploads.data ? count(uploads.data.total) : undefined} id="ov-up">
      {#snippet actions()}{@render more(sectionHref(accountId, 'uploads'), 'All emotes')}{/snippet}
      {#if uploads.data}
        {#if uploads.data.items.length}
          <ul class="rows">
            {#each uploads.data.items as e (e.id)}
              {@const pic = { host: e.host, animated: e.animated, files: e.files }}
              <li>
                <span class="thumb">{#if ready(pic)}<img src={src(pic, 1)} alt="" loading="lazy" />{/if}</span>
                <a class="nm" href="/me/emotes/{encodeURIComponent(e.id)}"><bdi>{clean(e.name)}</bdi></a>
                <span class="sub ui-mono">{dateTime(e.createdAt)}</span>
                <span class="ui-chip {e.state.toLowerCase()}">{STATE_LABEL[e.state]}</span>
              </li>
            {/each}
          </ul>
        {:else}<p class="ui-hint pad">No uploads yet. <a href="/me/upload">Upload your first emote</a>.</p>{/if}
      {:else}{@render waiting(uploads)}{/if}
    </Section>

    <Section title="Editors" count={editors.data ? count(editors.data.length) : undefined} id="ov-ed">
      {#snippet actions()}{@render more(sectionHref(accountId, 'editors'), 'Manage')}{/snippet}
      {#if editors.data}
        {#if editors.data.length}
          <ul class="rows">
            {#each editors.data.slice(0, 5) as ed (ed.account.id)}
              <li><a class="nm" href="/users/{encodeURIComponent(ed.account.id)}"><PaintedName name={ed.account.displayName} paint={looks[ed.account.id]?.paint} badge={looks[ed.account.id]?.badge} /></a></li>
            {/each}
          </ul>
        {:else}<p class="ui-hint pad">Nobody yet. Editors can manage your sets and emotes for you.</p>{/if}
      {:else}{@render waiting(editors)}{/if}
    </Section>
    <Section title="Notifications" count={inbox.count ? `${count(inbox.count)} unread` : undefined} id="ov-no">
      {#snippet actions()}{@render more('/me/notifications', 'All')}{/snippet}
      {#if inbox.items}
        {#if latestNotes.length}
          <ul class="notes">{#each latestNotes as n (n.id)}<li><NotificationRow note={n} variant="dense" /></li>{/each}</ul>
        {:else}<p class="ui-hint pad">Nothing new. Reviews, editor changes and cosmetics you get show up here.</p>{/if}
      {:else}{@render waiting({ failed: !!inbox.error })}{/if}
    </Section>

    <Section title="Recent activity" id="ov-ac">
      {#snippet actions()}{@render more(sectionHref(accountId, 'activity'), 'See all')}{/snippet}
      {#if activity.data}
        {#if activity.data.length}<div class="feed"><ActivityRows items={activity.data} subject={accountId} /></div>
        {:else}<p class="ui-hint pad">Nothing yet. Changes to your sets, emotes, cosmetics and editors show up here.</p>{/if}
      {:else}{@render waiting(activity)}{/if}
    </Section>

    <Section title="Cosmetics" id="ov-co">
      {#snippet actions()}{@render more(sectionHref(accountId, 'cosmetics'), 'Change')}{/snippet}
      {#if owned.data}
        <FactRows items={[
          { label: 'Paint', key: 'paint' },
          { label: 'Badge', value: look.badge ? clean(look.badge.name) : 'None' },
          { label: 'Owned', value: `${count(owned.data.paints)} paints, ${count(owned.data.badges)} badges`, mono: true },
        ]}>
          {#snippet value(k)}{#if k === 'paint'}{#if look.paint}<PaintedName name={look.paint.name} paint={look.paint} />{:else}None{/if}{/if}{/snippet}
        </FactRows>
      {:else}{@render waiting(owned)}{/if}
    </Section>

    <Section title="Emote sets" count={library.data ? count(library.data.sets.length) : undefined} id="ov-se">
      {#snippet actions()}{@render more(sectionHref(accountId, 'sets'), 'Sets')}{/snippet}
      {#if library.data}
        {#if library.data.sets.length}
          <ul class="rows">
            {#each library.data.sets.slice(0, 5) as s (s.id)}
              <li><a class="nm" href="/sets/{encodeURIComponent(s.id)}"><bdi>{clean(s.name)}</bdi></a></li>
            {/each}
          </ul>
        {:else}<p class="ui-hint pad">No sets yet.</p>{/if}
      {:else}{@render waiting(library)}{/if}
    </Section>
</div>

<style>
  /* The numbers: one sheet of cells sharing hairlines, as many per row as fit (two on a phone). */
  .stats { list-style: none; margin: 0 0 24px; padding: 0; display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    border-top: 1px solid var(--border); border-left: 1px solid var(--border); }
  .stats li { min-width: 0; border-right: 1px solid var(--border); border-bottom: 1px solid var(--border); }
  .stats a { display: grid; gap: 6px; padding: 12px 14px; color: var(--text); }
  @media (hover: hover) and (pointer: fine) { .stats a:hover { background: var(--surface-hover); } }
  .stats a:focus-visible { outline-offset: -2px; }
  .st-l { font-size: var(--t-label, 12px); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .st-n { font-size: 22px; font-weight: 600; line-height: 1; font-variant-numeric: tabular-nums; min-height: 22px; }
  .st-sk { display: block; width: 48px; height: 18px; border-radius: var(--radius); }
  @media (max-width: 479.98px) { .stats { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
  /* The tiles: balanced columns of flat Sections, each opened by a hairline (the tops line up). */
  /* 24px of air each side of the rule, as the two columns of Account and Editors. */
  .tiles { columns: 4 360px; column-gap: 48px; column-rule: 1px solid var(--border); }
  .tiles > :global(.sec) { break-inside: avoid; border-top: 1px solid var(--border); }
  .more { display: inline-flex; align-items: center; gap: 2px; min-height: 32px; padding: 0 2px 0 8px; border-radius: var(--radius);
    font-size: var(--t-fine); font-weight: 500; color: var(--muted); white-space: nowrap; }
  @media (hover: hover) and (pointer: fine) { .more:hover { color: var(--text); } }
  .rows, .notes { list-style: none; margin: 0; padding: 0; }
  .rows li { display: flex; align-items: center; gap: 12px; min-height: 44px; padding: 4px 0; border-bottom: 1px solid var(--border); font-size: var(--t-small); }
  .rows li > :global(svg) { color: var(--dim); flex: none; }
  .nm { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; color: var(--text); }
  .sub { flex: none; max-width: 50%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 12px; color: var(--muted); }
  .sub a { color: var(--muted); text-decoration: underline; text-decoration-color: var(--border-strong); text-underline-offset: 3px; }
  .thumb { flex: none; display: grid; place-items: center; width: 32px; height: 32px; }
  .thumb img { max-width: 32px; max-height: 32px; width: auto; height: auto; }
  .pad { margin: 0; padding: 12px 0 14px; }
  .pad a { color: var(--text); text-decoration: underline; text-decoration-color: var(--border-strong); text-underline-offset: 3px; }
  .line { display: block; height: 12px; width: 70%; border-radius: var(--radius); }
  .line + .line { margin-top: 8px; }
  .line.short { width: 40%; }
  .notes { border-top: 1px solid var(--border); }
  .feed :global(.feed) { border-top: 0; }
  .feed :global(.item:last-child) { border-bottom: 0; }
</style>
