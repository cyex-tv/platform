<script lang="ts">
  // The profile's rail: navigation only (the person is in the ProfileHeader band above it), the
  // same on /users/<id> and on the /me pages that belong to it, so managing your emotes never feels
  // like another area. The rows are public and the same for every viewer (rows.ts): Uploaded
  // emotes, Emote sets, Cosmetics and Activity. What only the owner (or an editor) opens, the
  // dashboard, Active emotes, Favourites, Channels and Admin, is reached from the account menu, the
  // band's buttons and the dashboard, never from here. It runs the full height of the left side and
  // never scrolls inside itself (SideRail `fill`). Under 900px the rows are a tab strip under the
  // band, plus a tab for the page you are on when it has no row.
  import { page } from '$app/state';
  import { clean } from '$lib/format';
  import SideRail from '$lib/shell/SideRail.svelte';
  import { profileRail, stripExtra, stripTabs, type RailKey, type SectionKey } from './rows';

  type Props = {
    account: { id: string; displayName: string };
    current: RailKey;
    activity?: boolean;
  };
  let { account, current, activity = false }: Props = $props();

  const groups = $derived(profileRail(account.id, current, activity));
  const section = $derived(current && current !== 'channels' && current !== 'connections' && current !== 'editors' ? (current as SectionKey) : null);
  const tabs = $derived(stripTabs(groups, stripExtra(page.url.pathname, section)));
</script>

<SideRail label="{clean(account.displayName)}'s profile" {groups} {tabs} collapse="tabs" fill />
