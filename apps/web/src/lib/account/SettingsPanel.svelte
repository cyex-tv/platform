<script lang="ts">
  // Settings' right panel (redesign round 2, wide screens): what the section you are on does, and
  // your account in facts (name, id, since when, connections, how you sign in), with the places
  // the section leads to. Under 1100px it lies flat after the section (WorkspaceLayout).
  import { page } from '$app/state';
  import FactRows from '$lib/components/FactRows.svelte';
  import LinkRows from '$lib/components/LinkRows.svelte';
  import PanelBlock from '$lib/components/PanelBlock.svelte';
  import { clean, count, date } from '$lib/format';
  import { platformName } from '$lib/shell/account';
  import { session } from '$lib/tool/session.svelte';
  import { sectionHref } from '$lib/profile/rows';

  type About = { title: string; text: string; links?: { href: string; label: string; hint?: string }[] };
  const ABOUT: Record<string, About> = {
    appearance: {
      title: 'Appearance',
      text: 'How cyex.tv looks on this device: the theme and how much moves. Each browser keeps its own choice.',
    },
    notifications: {
      title: 'Notifications',
      text: 'Which events reach your bell. Messages about your account and its safety always arrive.',
      links: [{ href: '/me/notifications', label: 'Your notifications', hint: 'Everything that reached you' }],
    },
    hidden: {
      title: 'Hidden',
      text: 'Emotes and creators you hid. They stay out of browse, search and suggestions for you; nobody else is told.',
      links: [{ href: '/search', label: 'Browse emotes', hint: 'Hide from a tile’s menu' }],
    },
    billing: {
      title: 'Billing',
      text: 'What you bought in the shop: receipts, gifts and codes. Payments go through our payment provider; cyex.tv never sees your card.',
      links: [{ href: '/shop', label: 'Shop', hint: 'Paints and badges' }],
    },
  };
  const key = $derived.by(() => {
    const m = page.url.pathname.match(/^\/me\/settings\/([a-z]+)/);
    return m && ABOUT[m[1]] ? m[1] : 'appearance';
  });
  const about = $derived(ABOUT[key]);
  const me = $derived(session.me);
  const who = $derived(me?.identities[0]);
  const facts = $derived(
    me
      ? [
          { label: 'Name', value: clean(who?.displayName ?? ''), avatar: { src: who?.avatar, name: clean(who?.displayName ?? '') } },
          { label: 'Account id', value: me.id, mono: true },
          ...(me.createdAt ? [{ label: 'Member since', value: date(me.createdAt), mono: true }] : []),
          { label: 'Connections', value: count(me.identities.length), mono: true },
          { label: 'Signs in with', value: me.identities.map((i) => platformName(i.platform)).join(', ') },
        ]
      : [],
  );
</script>

<PanelBlock title="This section">
  <p class="pb-hint">{about.text}</p>
</PanelBlock>
{#if me}
  <PanelBlock title="Your account">
    <FactRows label="Your account" items={facts} />
  </PanelBlock>
  <PanelBlock title="Related">
    <LinkRows label="Related" items={[
      { href: sectionHref(me.id, 'overview'), label: 'Your profile', hint: 'What everyone sees', icon: 'circle-user' },
      { href: sectionHref(me.id, 'account'), label: 'Account', hint: 'Name, connections, sign out everywhere', icon: 'user' },
      { href: sectionHref(me.id, 'editors'), label: 'Editors', hint: 'Who works on your account', icon: 'users' },
      ...(about.links ?? []),
    ]} />
  </PanelBlock>
{/if}
