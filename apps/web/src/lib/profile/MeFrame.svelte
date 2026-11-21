<script lang="ts">
  // The frame of the /me pages that belong to your profile (Channels, Notifications, Subscription,
  // Upload): the same Inspector as /users/<id> (your profile is your dashboard), so moving between
  // them never moves the person, the tabs or the panel. The head is you (ProfileHeader, flat, with
  // the Dashboard and Settings buttons), the tabs are the profile's public rows plus the page you are
  // on, and the panel is the page's own (facts, actions, related). The page's PageHead is the h1.
  import type { Snippet } from 'svelte';
  import { page } from '$app/state';
  import Icon from '$lib/components/Icon.svelte';
  import LinkTabs from '$lib/components/LinkTabs.svelte';
  import { quiet } from '$lib/quiet';
  import InspectorLayout from '$lib/shell/InspectorLayout.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { ACTIVITY_READY } from './activity';
  import { profileOf, relationTo, type PublicProfile } from './data.svelte';
  import ProfileHeader from './ProfileHeader.svelte';
  import { profileRail, sectionHref, stripExtra, stripTabs } from './rows';

  type Props = {
    /** The panel's label. */
    title: string;
    label?: string;
    panel: Snippet;
    children: Snippet;
    narrow?: 'before' | 'after';
    fold?: boolean;
  };
  let { title, label, panel, children, narrow = 'before', fold = false }: Props = $props();

  const me = $derived(session.me);
  const who = $derived(me?.identities[0]);
  const account = $derived(me ? { id: me.id, displayName: who?.displayName ?? '', avatar: who?.avatar } : null);
  const relation = $derived(relationTo(me?.id ?? '', me ?? null, null));
  let profile = $state<PublicProfile | null>(null);
  $effect(() => {
    const id = me?.id;
    if (!id) return;
    void quiet(profileOf(id).then((p) => (profile = p)), 'your profile');
  });
  const tabList = $derived(account ? stripTabs(profileRail(account.id, null, ACTIVITY_READY), stripExtra(page.url.pathname)) : []);
</script>

<InspectorLayout {title} {label} {panel} {narrow} {fold} headPad={false} testid="me-panel">
  {#snippet head()}
    {#if account}
      <ProfileHeader {account} {relation} {profile} heading={false} facts={false} flat>
        {#snippet actions()}
          <a class="ui-btn secondary" href={sectionHref(account.id, 'overview')}><Icon name="gauge" size={15} />Dashboard</a>
          <a class="ui-btn secondary" href="/me/settings"><Icon name="settings" size={15} />Settings</a>
        {/snippet}
      </ProfileHeader>
    {:else}
      <ProfileHeader account={{ id: '', displayName: '' }} {relation} profile={null} loading flat />
    {/if}
  {/snippet}
  {#snippet tabs()}
    {#if account}<LinkTabs label="Your profile" items={tabList} />{/if}
  {/snippet}
  {@render children()}
</InspectorLayout>
