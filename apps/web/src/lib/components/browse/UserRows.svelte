<script lang="ts">
  // People found by name, as profile rows (the Users tab on /search): avatar, name, the logins
  // they signed in with, each row one link to their page. The name is drawn in their paint when
  // this browser knows it (`looks`, the shared cosmetics cache holds yours; the API shows nobody
  // else's by account). The API gives no emote count, so none is shown.
  import Avatar from '$lib/components/Avatar.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { NO_LOOK, type Look } from '$lib/cosmetics/store.svelte';
  import { clean } from '$lib/format';
  import type { FindPeopleQuery } from '$lib/gql/graphql';

  type Hit = FindPeopleQuery['searchAccounts']['items'][number];
  type Props = {
    items: Hit[];
    looks: Record<string, Look>;
  };
  let { items, looks }: Props = $props();

  const where = (p: string) => (p === 'KICK' ? 'Kick' : 'Twitch');
</script>

<ul class="people" aria-label="People">
  {#each items as h (h.account.id)}
    {@const look = looks[h.account.id] ?? NO_LOOK}
    <li>
      <a href="/users/{encodeURIComponent(h.account.id)}">
        <Avatar src={h.account.avatar} name={h.account.displayName} size={44} />
        <span class="who">
          <span class="name"><PaintedName name={h.account.displayName} paint={look.paint} badge={look.badge} /></span>
          {#if h.identities.length}
            <span class="logins">
              {#each h.identities as id, k (id.platform + id.login)}
                {#if k > 0}<span class="sep" aria-hidden="true">·</span>{/if}
                <span class="login"><span class="plat">{where(id.platform)}</span> <bdi>{clean(id.login)}</bdi></span>
              {/each}
            </span>
          {/if}
        </span>
      </a>
    </li>
  {/each}
</ul>

<style>
  .people { list-style: none; margin: 0; padding: 0; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); overflow: hidden; }
  li + li { border-top: 1px solid var(--border); }
  a { display: flex; align-items: center; gap: 14px; min-height: 64px; padding: 10px 16px; color: var(--text); }
  a:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .who { display: grid; gap: 2px; min-width: 0; }
  .name { font-size: var(--t-ui); font-weight: 600; min-width: 0; display: flex; }
  .logins { display: flex; flex-wrap: wrap; align-items: baseline; gap: 0 8px; min-width: 0; font-size: var(--t-small); color: var(--muted); }
  .login { min-width: 0; overflow-wrap: anywhere; }
  .plat { color: var(--dim); }
  .sep { color: var(--dim); }
  @media (hover: hover) and (pointer: fine) { a:hover { background: var(--surface-2); } }
  @media (max-width: 480px) { a { padding: 10px 12px; gap: 12px; } }
</style>
