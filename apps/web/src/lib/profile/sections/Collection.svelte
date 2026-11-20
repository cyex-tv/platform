<script lang="ts">
  // The profile's Collection tab: every badge and paint the account owns, when it shows them
  // (Settings → Account, or the switch here for the owner). Paints are drawn on the owner's own
  // name, badges at 32px with their name; each says since when and how it came (shop, code, event,
  // staff), and which ones are worn now. Nothing about who gave it or what it cost: the API
  // doesn't send that. The owner always sees their own, with the switch on top.
  import { gql } from '$lib/api';
  import Icon, { type IconName } from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { loadCatalog, type Look } from '$lib/cosmetics/store.svelte';
  import { clean, date, iso } from '$lib/format';
  import type { GrantSource } from '$lib/gql/graphql';
  import { ACCOUNT_COLLECTION } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import CollectionToggle from '../CollectionToggle.svelte';

  type Props = { accountId: string; mine: boolean; name: string; look?: Look | null };
  let { accountId, mine, name, look = null }: Props = $props();

  const L = loader(async ({ fetch }) => {
    const [r, catalog] = await Promise.all([gql(ACCOUNT_COLLECTION, { accountId }, fetch), loadCatalog()]);
    return { collection: r.accountCollection ?? null, catalog };
  });

  const SOURCE: Record<GrantSource, { label: string; icon: IconName }> = {
    PURCHASE: { label: 'Shop', icon: 'shopping-bag' },
    CODE: { label: 'Code', icon: 'ticket' },
    EVENT: { label: 'Event', icon: 'calendar' },
    STAFF: { label: 'Staff', icon: 'shield' },
    DEFAULT: { label: 'Everyone', icon: 'users' },
    OWNER: { label: 'Staff', icon: 'shield' },
  };
  let show = $state<'all' | 'badges' | 'paints'>('all');
  const who = $derived(iso(clean(name)));
</script>

<PageHead title="Collection" line={mine ? 'Every badge and paint you own.' : `Every badge and paint ${who} owns.`}>
  {#snippet actions()}
    {#if mine}<a class="btn btn-s" href="/shop"><Icon name="shopping-bag" size={15} />Shop</a>{/if}
  {/snippet}
</PageHead>

<LoadGate source={L} label="Loading the collection…">
  {#snippet children(d)}
    {#if mine && d.collection}
      <section class="ui-panel switch" aria-label="Who sees your collection">
        <CollectionToggle value={d.collection.public} onchange={(c) => L.set({ ...d, collection: c })} />
      </section>
    {/if}
    {#if !d.collection}
      <State title="This collection is private.">{who} hasn’t chosen to show their badges and paints.</State>
    {:else}
      {@const badges = d.collection.items.filter((i) => i.kind === 'BADGE' && d.catalog.badges.has(i.cosmeticId))}
      {@const paints = d.collection.items.filter((i) => i.kind === 'PAINT' && d.catalog.paints.has(i.cosmeticId))}
      {#if !badges.length && !paints.length}
        <State title="Nothing here yet.">
          {#if mine}Badges and paints you get from the shop, a code or an event show up here.{:else}{who} doesn’t own any badges or paints yet.{/if}
          {#snippet actions()}{#if mine}<a class="btn btn-s btn-primary" href="/shop">Open the shop</a>{/if}{/snippet}
        </State>
      {:else}
        {#if badges.length && paints.length}
          <div class="filter">
            <Segmented label="Show" size="s" value={show} onchange={(v) => (show = v)}
              options={[{ value: 'all', label: `All ${badges.length + paints.length}` }, { value: 'badges', label: `Badges ${badges.length}` }, { value: 'paints', label: `Paints ${paints.length}` }]} />
          </div>
        {/if}
        {#if paints.length && show !== 'badges'}
          <section class="group" aria-labelledby="col-paints">
            <h3 id="col-paints">Paints <span class="n ui-num">{paints.length}</span></h3>
            <ul class="grid">
              {#each paints as i (i.cosmeticId)}
                {@const p = d.catalog.paints.get(i.cosmeticId)!}
                {@const src = SOURCE[i.source]}
                {@const worn = look?.paint?.id === i.cosmeticId}
                <li class="card">
                  <span class="stage paint"><PaintedName name={name} paint={p} /></span>
                  <span class="meta">
                    <strong><bdi>{clean(i.name)}</bdi></strong>
                    <span class="facts"><Icon name={src.icon} size={13} />{src.label} · since <span class="ui-num">{date(i.ownedSince)}</span></span>
                  </span>
                  {#if worn}<span class="worn"><Icon name="check" size={13} />Wearing</span>{/if}
                </li>
              {/each}
            </ul>
          </section>
        {/if}
        {#if badges.length && show !== 'paints'}
          <section class="group" aria-labelledby="col-badges">
            <h3 id="col-badges">Badges <span class="n ui-num">{badges.length}</span></h3>
            <ul class="grid">
              {#each badges as i (i.cosmeticId)}
                {@const b = d.catalog.badges.get(i.cosmeticId)!}
                {@const src = SOURCE[i.source]}
                {@const worn = look?.badge?.id === i.cosmeticId}
                <li class="card">
                  <span class="stage"><BadgeIcon badge={b} size={32} /></span>
                  <span class="meta">
                    <strong><bdi>{clean(i.name)}</bdi></strong>
                    <span class="facts"><Icon name={src.icon} size={13} />{src.label} · since <span class="ui-num">{date(i.ownedSince)}</span></span>
                  </span>
                  {#if worn}<span class="worn"><Icon name="check" size={13} />Wearing</span>{/if}
                </li>
              {/each}
            </ul>
          </section>
        {/if}
      {/if}
    {/if}
  {/snippet}
</LoadGate>

<style>
  .switch { margin-bottom: 16px; }
  .filter { margin-bottom: 16px; }
  .group + .group { margin-top: 24px; }
  h3 { display: flex; align-items: baseline; gap: 8px; margin: 0 0 10px; font-size: 15px; font-weight: 600; }
  .n { font-size: var(--t-fine); font-weight: 500; color: var(--dim); }
  .grid { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; grid-template-columns: repeat(auto-fill, minmax(min(240px, 100%), 1fr)); }
  .card { position: relative; display: grid; grid-template-rows: 64px auto; gap: 10px; min-width: 0; padding: 12px; background: var(--surface);
    border: 1px solid var(--border); border-radius: var(--radius-card); }
  .stage { display: grid; place-items: center; min-width: 0; padding: 0 8px; border-radius: var(--radius); background: var(--bg); overflow: hidden; }
  .stage.paint { font-size: 20px; font-weight: 700; }
  .meta { display: grid; gap: 2px; min-width: 0; }
  .meta strong { font-size: var(--t-small); font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .facts { display: inline-flex; align-items: center; gap: 5px; font-size: var(--t-fine); color: var(--muted); }
  .facts :global(svg) { flex: none; color: var(--dim); }
  .worn { position: absolute; top: 16px; right: 16px; display: inline-flex; align-items: center; gap: 3px; font-size: var(--t-label); font-weight: 700; color: var(--text); }
  .worn :global(svg) { color: var(--accent-text); }
</style>
