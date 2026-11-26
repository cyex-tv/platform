<script lang="ts">
  // Accounts as a dense list: avatar, painted name, the logins they signed in with. A row opens
  // the account's admin page, or with `onpick` picks it (a filter's lookup, compact). One Tab
  // stop; the arrow keys move over the rows.
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { looks, wantLooks } from '$lib/cosmetics/store.svelte';
  import { clean } from '$lib/format';
  import { gridKeys } from './ui/pick';

  type Item = { account: { id: string; displayName: string; avatar?: string | null }; identities: { platform: string; login: string }[] };
  type Props = { items: Item[]; onpick?: (account: { id: string; displayName: string }) => void; compact?: boolean; label?: string };
  let { items, onpick, compact = false, label = 'Accounts' }: Props = $props();

  $effect(() => wantLooks(items.map((i) => i.account.id)));
  const where = (p: string) => (p === 'KICK' ? 'Kick' : 'Twitch');
  const open = (id: string) => `/admin/users/${encodeURIComponent(id)}`;
</script>

<ul class="adm-rows" class:compact aria-label={label} use:gridKeys>
  {#each items as h, i (h.account.id)}
    {@const l = looks[h.account.id]}
    <li>
      {#snippet body()}
        <Avatar src={h.account.avatar} name={h.account.displayName} size={32} tone="tool" />
        <span class="nm"><PaintedName name={h.account.displayName} paint={l?.paint} badge={l?.badge} /></span>
        <span class="logins">
          {#each h.identities as id (id.platform + id.login)}<span class="login"><Icon name={id.platform === 'KICK' ? 'kick' : 'twitch'} size={13} /><bdi>{clean(id.login)}</bdi><span class="sr-only">{' on ' + where(id.platform)}</span></span>{/each}
        </span>
        {#if !compact}<span class="aid adm-num">{h.account.id}</span>{/if}
        {#if !onpick}<span class="go" aria-hidden="true"><Icon name="chevron-right" size={14} /></span>{/if}
      {/snippet}
      {#if onpick}
        <button class="row" type="button" data-item tabindex={i === 0 ? 0 : -1} onclick={() => onpick(h.account)}>{@render body()}</button>
      {:else}
        <a class="row" href={open(h.account.id)} data-item tabindex={i === 0 ? 0 : -1}>{@render body()}</a>
      {/if}
    </li>
  {/each}
</ul>

<style>
  .row { display: flex; align-items: center; gap: 12px; width: 100%; min-height: 48px; padding: 0 2px; border: 0; background: none;
    color: var(--text); font: inherit; text-align: left; text-decoration: none; cursor: pointer; }
  .row:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  @media (hover: hover) and (pointer: fine) { .row:hover { background: var(--row-hover); } .row:hover .go { color: var(--accent-text); } }
  .nm { flex: none; max-width: 38%; min-width: 0; font-weight: 600; font-size: var(--t-small); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .logins { display: flex; flex: 1; min-width: 0; flex-wrap: wrap; gap: 0 14px; overflow: hidden; max-height: 20px; font-size: var(--t-fine); color: var(--muted); }
  .login { display: inline-flex; align-items: center; gap: 5px; white-space: nowrap; }
  .login > :global(svg) { color: var(--dim); }
  .aid { flex: none; font-size: var(--t-fine); color: var(--dim); }
  .go { flex: none; display: inline-flex; color: var(--dim); }
  .compact { max-height: 260px; overflow-y: auto; }
  .compact .row { min-height: 44px; padding: 0 16px; }
  @media (max-width: 899.98px) { .aid { display: none; } }
  @media (max-width: 639.98px) {
    .row { min-height: 56px; flex-wrap: wrap; row-gap: 0; padding: 6px 2px; }
    .nm { max-width: calc(100% - 80px); }
    .logins { flex-basis: 100%; margin-left: 44px; max-height: 20px; order: 5; }
    .go { margin-left: auto; }
  }
</style>
