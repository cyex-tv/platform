<script lang="ts">
  // The profile's rail: navigation only (the person is in the ProfileHeader band above it), the
  // same on /users/<id> and on the /me pages that belong to it, so managing your emotes never feels
  // like another area. Rows: Overview (your own profile's dashboard), Active emotes, Uploaded
  // emotes, Emote sets, Channels, then Cosmetics and Activity; "Admin ›" pinned at the bottom for
  // staff on their own profile. It runs the full height of the left side and never scrolls inside
  // itself (SideRail `fill`). Under 900px the rows are a tab strip under the band.
  import { page } from '$app/state';
  import Icon from '$lib/components/Icon.svelte';
  import { clean } from '$lib/format';
  import SideRail from '$lib/shell/SideRail.svelte';
  import { isAdmin, session } from '$lib/tool/session.svelte';
  import { profileRail, stripExtra, stripTabs, type RailKey, type Relation } from './rows';

  type Props = {
    account: { id: string; displayName: string };
    relation: Relation;
    current: RailKey;
    activity?: boolean;
  };
  let { account, relation, current, activity = false }: Props = $props();

  const groups = $derived(profileRail(account.id, relation, current, activity));
  const staff = $derived(relation.mine && isAdmin(session.me));
  // The /me pages without a row of their own (Upload, Notifications) add their tab to the phone strip.
  const tabs = $derived(relation.mine ? stripTabs(groups, stripExtra(page.url.pathname)) : undefined);
</script>

<SideRail label="{clean(account.displayName)}'s profile" {groups} {tabs} collapse="tabs" fill>
  {#snippet foot()}
    {#if staff}<a class="rail-foot" href="/admin"><Icon name="wrench" /><span class="lbl">Admin</span><Icon name="chevron-right" size={14} /></a>{/if}
  {/snippet}
</SideRail>
