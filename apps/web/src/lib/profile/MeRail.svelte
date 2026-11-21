<script lang="ts">
  // The rail of your area (redesign round 2, the Workspace): flush with the window's left edge,
  // your places with their counts (your emotes, favourites, channels, unread notifications,
  // upload), then your sets with how full each is (the browse rail's meters, LIVE for the one a
  // channel shows) and the sets you edit for others, grouped by whose they are. Settings at the
  // foot. Under 900px it is SideRail's tab strip: your places, plus the set you are on.
  import { page } from '$app/state';
  import Icon from '$lib/components/Icon.svelte';
  import { count as fmt, clean } from '$lib/format';
  import { inbox, refreshCount } from '$lib/account/inbox.svelte';
  import { target } from '$lib/components/browse/target.svelte';
  import { personal } from '$lib/personal/lists.svelte';
  import SideRail from '$lib/shell/SideRail.svelte';
  import type { RailGroup, RailItem } from '$lib/shell/rail';
  import { session } from '$lib/tool/session.svelte';
  import { sectionHref } from './rows';

  const me = $derived(session.me);
  $effect(() => {
    if (!me) return;
    target.ensure();
    personal.ensure();
    if (!inbox.known) void refreshCount().catch(() => {});
  });

  const path = $derived(page.url.pathname);
  const at = (base: string) => path === base || path.startsWith(`${base}/`);
  const setShown = (id: string) => path === `/me/sets/${id}` || path === `/me/sets/${encodeURIComponent(id)}`;

  const places = $derived.by<RailItem[]>(() => {
    if (!me) return [];
    return [
      { href: sectionHref(me.id, 'overview'), label: 'Dashboard', icon: 'gauge' },
      { href: sectionHref(me.id, 'uploads'), label: 'Your emotes', icon: 'smile', current: at('/me/emotes') },
      { href: sectionHref(me.id, 'favourites'), label: 'Favourites', icon: 'heart', count: personal.loaded ? personal.favourites.size : undefined, current: at('/me/favourites') },
      { href: '/me/channels', label: 'Channels', icon: 'tv', current: at('/me/channels') },
      { href: '/me/notifications', label: 'Notifications', icon: 'bell', count: inbox.known && inbox.count > 0 ? inbox.count : undefined, current: at('/me/notifications') },
      { href: '/me/upload', label: 'Upload', icon: 'upload', current: at('/me/upload') },
      // Subscriptions don't exist yet: the row shows only while you are on its page.
      ...(at('/me/subscription') ? [{ href: '/me/subscription', label: 'Subscription', icon: 'star' as const, current: true }] : []),
    ];
  });

  const setRow = (s: NonNullable<typeof target.sets>[number]): RailItem => ({
    href: `/me/sets/${encodeURIComponent(s.id)}`,
    label: clean(s.name),
    icon: 'folder',
    count: `${fmt(s.held.size)}/${fmt(s.capacity)}`,
    meter: { used: s.held.size, capacity: s.capacity },
    live: s.active,
    current: setShown(s.id),
  });

  const groups = $derived.by<RailGroup[]>(() => {
    const out: RailGroup[] = [{ items: places }];
    const sets = target.sets ?? [];
    const mine = sets.filter((s) => s.mine);
    if (mine.length) out.push({ caption: `Your sets · ${fmt(mine.length)}`, items: mine.map(setRow) });
    const others = new Map<string, typeof sets>();
    for (const s of sets.filter((x) => !x.mine)) others.set(s.ownerName, [...(others.get(s.ownerName) ?? []), s]);
    for (const [owner, list] of others) out.push({ caption: `${clean(owner)} · editor`, items: list.map(setRow) });
    return out;
  });
  // The phone strip: your places, plus the set you are on.
  const tabs = $derived([...places, ...groups.slice(1).flatMap((g) => g.items).filter((i) => i.current)]);
  // Dashboard is current on none of the /me pages (it is the profile's Overview).
  const known = $derived(!!me && target.sets !== null);
</script>

<SideRail label="Your area" {groups} {tabs} fill wait={!known}>
  {#snippet head()}
    <div class="title"><strong>Your area</strong>{#if target.sets === null && me}<span class="ph ui-skeleton" aria-hidden="true"></span>{/if}</div>
  {/snippet}
  {#snippet foot()}<a class="rail-foot" href="/me/settings"><Icon name="settings" /><span class="lbl">Settings</span><Icon name="chevron-right" size={14} /></a>{/snippet}
</SideRail>

<style>
  .title { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 28px; padding-left: 10px; }
  .title strong { font-size: var(--text-label); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .ph { width: 48px; height: 10px; border-radius: var(--radius); }
</style>
