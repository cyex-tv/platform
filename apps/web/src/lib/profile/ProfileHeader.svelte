<script lang="ts">
  // The profile's identity band, cyex's own take on the 7TV profile: 7TV centres the person in a
  // tall side card above the tabs; here the person heads the page instead, left-aligned like a
  // cyex tool header. One surface band across the content column: the paint-ringed avatar, the
  // name (the page's h1, drawn in the paint, with the badge) and the roles as flags right after
  // it; under them one facts line (connections linking out, the channels and the set each shows,
  // the editors by name, member since), each fact an icon and a word, never a box. The rail beside
  // it is navigation only, so the rail reads the same on every profile and /me page.
  //
  // Roles: everyone sees the public ones; on your own profile your hidden ones show too, marked
  // with an eye-off icon and "Hidden" (only you see them). Editors are names only, never what
  // they may do.
  import type { Snippet } from 'svelte';
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { paintRing } from '$lib/cosmetics/paint';
  import { looks, NO_LOOK, seedLook, wantLooks, type Look } from '$lib/cosmetics/store.svelte';
  import { clean, date } from '$lib/format';
  import { platformName } from '$lib/shell/account';
  import { myRoles, type PublicProfile } from './data.svelte';
  import { connectionUrl } from './public';
  import type { Relation } from './rows';

  type Props = {
    account: { id: string; displayName: string; avatar?: string | null };
    relation: Relation;
    profile: PublicProfile | null;
    seed?: Look | null;
    actions?: Snippet;
  };
  let { account, relation, profile, seed = null, actions }: Props = $props();

  const name = $derived(clean(account.displayName));
  const look = $derived(looks[account.id] ?? seed ?? NO_LOOK);
  const connections = $derived(profile?.identities ?? []);
  const active = $derived(profile?.activeSets ?? []);
  const editors = $derived(profile?.editors ?? []);

  let own = $state<{ id: string; roles: { id: string; name: string; public: boolean }[] } | null>(null);
  const roles = $derived<{ id: string; name: string; public: boolean }[]>(
    relation.mine && own?.id === account.id ? own.roles : (profile?.roles ?? []).map((r) => ({ ...r, public: true })),
  );

  $effect(() => {
    if (seed) seedLook(account.id, seed);
    else if (!relation.mine) wantLooks([account.id]);
  });
  $effect(() => wantLooks(editors.map((e) => e.id)));
  $effect(() => {
    const id = account.id;
    if (!relation.mine) return;
    myRoles()
      .then((list) => {
        if (id === account.id) own = { id, roles: list };
      })
      .catch(() => {});
  });
</script>

<header class="band">
  <span class="ring" style={paintRing(look.paint)}><span class="gap"><Avatar src={account.avatar} {name} size={64} /></span></span>
  <div class="id">
    <div class="top">
      <h1 class="name"><PaintedName {name} paint={look.paint} badge={look.badge} badgeSize={22} /></h1>
      {#if roles.length}
        <ul class="flags" aria-label="Roles">
          {#each roles as r (r.id)}
            <li class="flag" class:hidden={!r.public}>
              {#if !r.public}<Icon name="eye-off" size={12} />{/if}<bdi>{clean(r.name)}</bdi>{#if !r.public}<span class="hid">Hidden</span>{/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>
    <ul class="facts">
      {#each connections as c (c.platform + c.login)}
        {@const url = connectionUrl(c.platform, c.login)}
        <li>
          <Icon name={c.platform === 'KICK' ? 'kick' : 'twitch'} size={14} />
          {#if url}
            <a href={url} target="_blank" rel="noopener"><span class="sr-only">{`${platformName(c.platform)}: `}</span><bdi>{clean(c.login)}</bdi><span class="sr-only">{' (opens in a new tab)'}</span></a>
          {:else}<span class="sr-only">{`${platformName(c.platform)}: `}</span><bdi>{clean(c.login)}</bdi>{/if}
        </li>
      {/each}
      {#each active as a (a.platform + a.login)}
        <li>
          <Icon name="zap" size={14} />
          <span><a href="/sets/{encodeURIComponent(a.setId)}"><bdi>{clean(a.setName)}</bdi></a><span class="dim">{' on '}<bdi>{clean(a.login)}</bdi><span class="sr-only">{` (${platformName(a.platform)})`}</span></span></span>
        </li>
      {/each}
      {#if editors.length}
        <li>
          <Icon name="circle-user" size={14} />
          <span><span class="dim">{'Editors '}</span>{#each editors.slice(0, 4) as e, i (e.id)}{#if i}<span class="dim">{', '}</span>{/if}<a href="/users/{encodeURIComponent(e.id)}"><PaintedName name={e.displayName} paint={looks[e.id]?.paint} /></a>{/each}{#if editors.length > 4}<span class="dim">{` and ${editors.length - 4} more`}</span>{/if}</span>
        </li>
      {/if}
      {#if profile?.createdAt}<li class="dim"><Icon name="calendar" size={14} />{'Member since '}<time datetime={profile.createdAt}>{date(profile.createdAt)}</time></li>{/if}
    </ul>
  </div>
  {#if actions}<div class="acts">{@render actions()}</div>{/if}
</header>

<style>
  .band { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; align-items: center; gap: 18px;
    padding: 18px 20px; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); }
  .ring { display: inline-grid; padding: 2px; border-radius: 50%; background: var(--border); }
  .gap { display: inline-grid; padding: 2px; border-radius: 50%; background: var(--surface); }
  .id { display: grid; gap: 8px; min-width: 0; }
  .top { display: flex; flex-wrap: wrap; align-items: center; gap: 6px 12px; min-width: 0; }
  .name { margin: 0; min-width: 0; max-width: 100%; font-size: 24px; font-weight: 600; letter-spacing: -.015em; line-height: 1.2; overflow-wrap: anywhere; }
  .flags { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 4px; }
  .flag { display: inline-flex; align-items: center; gap: 4px; height: 22px; padding: 0 7px; border-radius: var(--radius);
    background: var(--neutral-chip); color: var(--muted); font-size: 12px; font-weight: 600; white-space: nowrap; }
  .flag.hidden { padding-left: 5px; }
  .flag .hid { font-weight: 500; color: var(--dim); }
  .flag .hid::before { content: '·'; margin-right: 4px; }
  .facts { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 4px 18px; font-size: var(--t-fine); color: var(--text); }
  .facts li { display: inline-flex; align-items: center; gap: 6px; min-width: 0; max-width: 100%; }
  .facts li > :global(svg) { flex: none; color: var(--dim); }
  .facts li > span, .facts li > a { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .facts a { color: var(--text); font-weight: 500; text-decoration: underline; text-decoration-color: var(--border-strong); text-underline-offset: 3px; }
  .dim, .facts .dim { color: var(--muted); font-weight: 400; }
  .acts { display: flex; gap: 8px; align-self: start; }
  @media (hover: hover) and (pointer: fine) { .facts a:hover { text-decoration-color: currentColor; } }
  @media (max-width: 899.98px) {
    .band { grid-template-columns: auto minmax(0, 1fr); gap: 12px 14px; padding: 14px; }
    .name { font-size: 20px; }
    .acts { grid-column: 1 / -1; }
  }
</style>
