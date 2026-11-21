<script lang="ts">
  // Accounts as a dense list: avatar, painted name, the logins they signed in with. A row opens
  // the account's admin page, or with `onpick` picks it (a filter's lookup, compact). One Tab
  // stop; the arrow keys move over the rows.
  import Avatar from '$lib/components/Avatar.svelte';
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
        <span class="logins ui-hint">
          {#each h.identities as id (id.platform + id.login)}<span class="login"><bdi>{clean(id.login)}</bdi> on {where(id.platform)}</span>{/each}
        </span>
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
  .row { display: flex; align-items: center; gap: 12px; width: 100%; min-height: 52px; padding: 0 16px; border: 0; background: none;
    color: var(--text); font: inherit; text-align: left; text-decoration: none; cursor: pointer; }
  .row:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  @media (hover: hover) and (pointer: fine) { .row:hover { background: var(--row-hover); } }
  .nm { flex: none; max-width: 45%; min-width: 0; font-weight: 600; font-size: var(--t-small); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .logins { display: flex; flex: 1; min-width: 0; flex-wrap: wrap; gap: 0 12px; justify-content: flex-end; overflow: hidden; max-height: 20px; }
  .login { white-space: nowrap; }
  .compact { max-height: 260px; overflow-y: auto; }
  .compact .row { min-height: 44px; }
  @media (max-width: 640px) {
    .row { min-height: 56px; flex-wrap: wrap; row-gap: 0; padding: 6px 16px; }
    .nm { max-width: calc(100% - 52px); }
    .logins { flex-basis: 100%; justify-content: flex-start; margin-left: 44px; max-height: 20px; }
  }
</style>
