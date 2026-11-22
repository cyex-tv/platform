<script lang="ts">
  // The profile's identity band, cyex's own take on the 7TV profile: the person heads the page,
  // left-aligned like a cyex tool header. One surface band across the content column: the
  // paint-ringed avatar, the name (the page's h1, drawn in the paint, with the badge) and the first
  // roles as flags right after it; under them one line of facts. Lists that can grow (connections,
  // with the set each channel shows; editors; roles past the first three) are collapsed behind
  // count chips that open a panel on purpose (FactPopover), so the band keeps its height for an
  // account at its 50 editors as for a new one (owner, 2026/11/20). Editors show ten at a time.
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
  import { session } from '$lib/tool/session.svelte';
  import { myRoles, type PublicProfile } from './data.svelte';
  import FactPopover from './FactPopover.svelte';
  import { connectionUrl } from './public';
  import type { Relation } from './rows';
  import { EDITORS_STEP, fold, moreText, ROLES_SHOWN, shown, showing } from './scale';

  type Props = {
    account: { id: string; displayName: string; avatar?: string | null };
    relation: Relation;
    profile: PublicProfile | null;
    seed?: Look | null;
    actions?: Snippet;
    /** The page has its own h1 (the /me pages): the name is then plain text with the same look, not a second h1. */
    heading?: boolean;
    /** Who this is isn't known yet: the band's shape with blank parts, so nothing moves when it arrives. */
    loading?: boolean;
    /** The Connections and Editors chips (off on the profile page, whose About panel lists them). */
    facts?: boolean;
    /** Flat on the ground (the profile page's Inspector head), not a surface band. */
    flat?: boolean;
  };
  let { account, relation, profile, seed = null, actions, heading = true, loading = false, facts = true, flat = false }: Props = $props();

  type Role = { id: string; name: string; public: boolean; invisible?: boolean };
  /** localStorage: '1' when your roles show only to you (app.html holds their line on a phone). */
  const OWN_FLAGS_KEY = 'cyexOwnFlags';
  const name = $derived(clean(account.displayName));
  const look = $derived(looks[account.id] ?? seed ?? NO_LOOK);
  const connections = $derived(profile?.identities ?? []);
  const active = $derived(profile?.activeSets ?? []);
  const editors = $derived(profile?.editors ?? []);

  let own = $state<{ id: string; roles: Role[] } | null>(null);
  const roles = $derived<Role[]>(
    relation.mine && own?.id === account.id ? own.roles : (profile?.roles ?? []).map((r) => ({ ...r, public: true })),
  );
  const folded = $derived(fold(roles, ROLES_SHOWN));

  const same = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();
  const setOn = (platform: string, login: string) => active.find((a) => a.platform === platform && same(a.login, login));
  // A channel showing a set whose login isn't among the connections still gets its row.
  const orphans = $derived(active.filter((a) => !connections.some((c) => c.platform === a.platform && same(c.login, a.login))));

  // Editors in the panel: a step at a time; only those shown are asked for their look.
  let steps = $state(1);
  $effect(() => {
    void account.id;
    steps = 1;
  });
  const editorsShown = $derived(editors.slice(0, shown(editors.length, steps, EDITORS_STEP)));

  $effect(() => {
    if (loading) return;
    if (seed) seedLook(account.id, seed);
    else if (!relation.mine) wantLooks([account.id]);
  });
  $effect(() => wantLooks(editorsShown.map((e) => e.id)));
  $effect(() => {
    const id = account.id;
    if (loading || !relation.mine) return;
    myRoles()
      .then((list) => {
        if (id !== account.id) return;
        own = { id, roles: list };
        // app.html's hint for the next visit: your roles (hidden ones too) show only to you, so
        // a phone keeps their line from the first paint.
        try {
          localStorage.setItem(OWN_FLAGS_KEY, list.length && !(profile?.roles ?? []).length ? '1' : '0');
        } catch {
          /* no hint */
        }
      })
      .catch(() => {});
  });
  // The hint is for the first paint of your own profile only.
  $effect(() => {
    if (loading || !session.known) return;
    if (!relation.mine || own?.id === account.id) document.documentElement.removeAttribute('data-own-flags');
  });
</script>

{#snippet flag(r: Role)}
  {@const hid = !r.public || !!r.invisible}
  <li class="flag" class:hidden={hid}>
    {#if hid}<Icon name="eye-off" size={12} />{/if}<bdi>{clean(r.name)}</bdi>{#if hid}<span class="hid">{r.invisible ? 'Invisible' : 'Hidden'}</span>{/if}
  </li>
{/snippet}

{#if loading}
  <header class="band" class:flat aria-hidden="true">
    <span class="ring"><span class="gap"><span class="ph-av ui-skeleton"></span></span></span>
    <div class="id">
      <div class="top"><span class="ph-name ui-skeleton"></span></div>
      <ul class="facts"><li><span class="ph-fact ui-skeleton"></span></li></ul>
    </div>
    <div class="acts"><span class="ph-btn ui-skeleton"></span></div>
  </header>
{:else}
<header class="band" class:flat>
  <span class="ring" style={paintRing(look.paint)}><span class="gap"><Avatar src={account.avatar} {name} size={64} /></span></span>
  <div class="id">
    <div class="top">
      <svelte:element this={heading ? 'h1' : 'p'} class="name"><PaintedName {name} paint={look.paint} badge={look.badge} badgeSize={22} /></svelte:element>
      {#if roles.length}
        <ul class="flags" aria-label="Roles">
          {#each folded.head as r (r.id)}{@render flag(r)}{/each}
          {#if folded.rest}
            <li class="more-roles">
              <FactPopover label="All roles" count={folded.rest} width="280px">
                <ul class="plist roles" aria-label="All roles">{#each roles as r (r.id)}{@render flag(r)}{/each}</ul>
              </FactPopover>
            </li>
          {/if}
        </ul>
      {/if}
    </div>
    <ul class="facts">
      {#if facts && (connections.length || orphans.length)}
        <li>
          <FactPopover label="Connections" word="Connections" icon="link" count={connections.length + orphans.length}>
            <ul class="plist" aria-label="Connections">
              {#each connections as c (c.platform + c.login)}
                {@const url = connectionUrl(c.platform, c.login)}
                {@const on = setOn(c.platform, c.login)}
                <li class="prow">
                  <Icon name={c.platform === 'KICK' ? 'kick' : 'twitch'} size={16} />
                  <span class="pmain">
                    {#if url}
                      <a href={url} target="_blank" rel="noopener"><span class="sr-only">{`${platformName(c.platform)}: `}</span><bdi>{clean(c.login)}</bdi><span class="sr-only">{' (opens in a new tab)'}</span></a>
                    {:else}
                      <span><span class="sr-only">{`${platformName(c.platform)}: `}</span><bdi>{clean(c.login)}</bdi></span>
                    {/if}
                    {#if on}<span class="psub">Shows <a href="/sets/{encodeURIComponent(on.setId)}"><bdi>{clean(on.setName)}</bdi></a></span>{/if}
                  </span>
                </li>
              {/each}
              {#each orphans as a (a.platform + a.login)}
                <li class="prow">
                  <Icon name={a.platform === 'KICK' ? 'kick' : 'twitch'} size={16} />
                  <span class="pmain">
                    <span><span class="sr-only">{`${platformName(a.platform)}: `}</span><bdi>{clean(a.login)}</bdi></span>
                    <span class="psub">Shows <a href="/sets/{encodeURIComponent(a.setId)}"><bdi>{clean(a.setName)}</bdi></a></span>
                  </span>
                </li>
              {/each}
            </ul>
          </FactPopover>
        </li>
      {/if}
      {#if facts && editors.length}
        <li>
          <FactPopover label="Editors" word="Editors" icon="circle-user" count={editors.length}>
            <ul class="plist" aria-label="Editors">
              {#each editorsShown as e (e.id)}
                <li class="prow">
                  <Avatar src={e.avatar} name={clean(e.displayName)} size={24} />
                  <span class="pmain"><a href="/users/{encodeURIComponent(e.id)}"><PaintedName name={e.displayName} paint={looks[e.id]?.paint} /></a></span>
                </li>
              {/each}
            </ul>
            {#if editorsShown.length < editors.length}
              <div class="pmore">
                <span class="psub ui-num" aria-live="polite">{showing(editorsShown.length, editors.length)}</span>
                <button class="btn btn-s" type="button" onclick={() => steps++}>{moreText(editorsShown.length, editors.length, EDITORS_STEP)}</button>
              </div>
            {/if}
          </FactPopover>
        </li>
      {/if}
      {#if profile?.createdAt}<li class="dim since"><Icon name="calendar" size={14} />{'Member since '}<time datetime={profile.createdAt}>{date(profile.createdAt)}</time></li>{/if}
    </ul>
  </div>
  {#if actions}<div class="acts">{@render actions()}</div>{/if}
</header>
{/if}

<style>
  .band { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; align-items: center; gap: 18px;
    padding: 18px 20px; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); }
  .band.flat { padding: 24px 0 18px; background: none; border: 0; border-radius: 0; }
  .band.flat .gap { background: var(--bg); }
  .ring { display: inline-grid; padding: 2px; border-radius: 50%; background: var(--border); }
  .gap { display: inline-grid; padding: 2px; border-radius: 50%; background: var(--surface); }
  .id { display: grid; grid-template-columns: minmax(0, 1fr); gap: 8px; min-width: 0; }
  .ph-av { display: block; width: 64px; height: 64px; border-radius: 50%; }
  .ph-name { display: block; width: 160px; max-width: 100%; height: 24px; border-radius: var(--radius); }
  .ph-fact { display: block; width: 220px; max-width: 100%; height: 28px; border-radius: var(--radius); }
  .ph-btn { display: block; width: 104px; height: 40px; border-radius: var(--radius-btn); }
  .top { min-height: 29px; display: flex; flex-wrap: wrap; align-items: center; gap: 6px 12px; min-width: 0; }
  .name { margin: 0; min-width: 0; max-width: 100%; font-size: 24px; font-weight: 600; letter-spacing: -.015em; line-height: 1.2; overflow-wrap: anywhere; }
  .flags { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 4px; min-width: 0; max-width: 100%; }
  .flag { display: inline-flex; align-items: center; gap: 4px; height: 22px; max-width: 100%; padding: 0 7px; border-radius: var(--radius);
    background: var(--neutral-chip); color: var(--muted); font-size: 12px; font-weight: 600; white-space: nowrap; }
  .flag > bdi { min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .flag.hidden { padding-left: 5px; }
  .flag .hid { font-weight: 500; color: var(--dim); }
  .flag .hid::before { content: '·'; margin-right: 4px; }
  .more-roles { display: inline-flex; }
  .more-roles :global(.fact) { height: 22px; padding: 0 6px; font-size: 12px; font-weight: 600; color: var(--muted); }
  .facts { min-height: 28px; list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; align-items: center; gap: 6px 10px;
    font-size: var(--t-fine); color: var(--text); }
  .facts > li { display: inline-flex; align-items: center; gap: 6px; min-width: 0; max-width: 100%; }
  .facts > li > :global(svg) { flex: none; color: var(--dim); }
  .since { margin-left: 4px; }
  .flat .since:first-child { margin-left: 0; }
  .dim { color: var(--muted); font-weight: 400; }
  .acts { display: flex; flex-wrap: wrap; gap: 8px; align-self: start; }

  /* The panels: fixed-height rows; a long list scrolls inside the panel (the Popover caps its height). */
  .plist { list-style: none; margin: 0; padding: 0; display: grid; }
  .plist.roles { display: flex; flex-wrap: wrap; gap: 4px; padding: 4px; }
  .prow { display: flex; align-items: center; gap: 10px; height: 44px; padding: 0 8px; min-width: 0; }
  .prow + .prow { border-top: 1px solid var(--border); }
  .prow > :global(svg) { flex: none; color: var(--dim); }
  .pmain { display: grid; min-width: 0; line-height: 1.3; font-size: var(--t-small); }
  .pmain > a, .pmain > span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pmain a { color: var(--text); font-weight: 500; text-decoration: underline; text-decoration-color: var(--border-strong); text-underline-offset: 3px; }
  .psub { font-size: var(--t-fine); color: var(--muted); }
  .pmore { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 8px 8px 4px; border-top: 1px solid var(--border); }
  @media (hover: hover) and (pointer: fine) { .pmain a:hover { text-decoration-color: currentColor; } }
  @media (max-width: 899.98px) {
    .band { grid-template-columns: auto minmax(0, 1fr); gap: 12px 14px; padding: 14px; }
    .band.flat { padding: 16px 0 12px; }
    .name { font-size: 20px; }
    .acts { grid-column: 1 / -1; }
    /* Your own profile before the session is known (app.html's hint): the buttons' row is kept. */
    :global(html[data-own]) .acts:empty { min-height: 40px; }
  }
  /* Phones: the roles get their own line under the name, so its height never depends on the name;
     on your own profile (app.html's hint) that line is kept until your roles arrive. */
  @media (max-width: 599.98px) {
    .top { flex-direction: column; align-items: flex-start; }
    :global(html[data-own-flags]) .top:not(:has(.flags)) { min-height: 52px; }
  }
</style>
