<script lang="ts">
  // An account's sets as cards (7TV's emote sets view): two rows of emotes, the name, how full it
  // is, and a mark on the set a channel shows. The owner and editors see all their sets, open them
  // in the editor and get "New set" (with SETS); everyone else sees the listed ones, which open
  // the public set page (/sets/<id>).
  import EmoteThumbs from './EmoteThumbs.svelte';
  import Capacity from '$lib/components/Capacity.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import { clean, count } from '$lib/format';
  import { fillPercent } from '$lib/sets/public';
  import { setMark } from '$lib/sets/visibility';
  import type { SetVisibility } from '$lib/gql/graphql';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { libraryOf, publicSetsOf } from '../data.svelte';
  import NewSetDialog from '../NewSetDialog.svelte';
  import type { Picture } from '$lib/emote';

  let { accountId, mine, canCreate, name = '' }: { accountId: string; mine: boolean; canCreate: boolean; name?: string } = $props();

  type Card = { id: string; name: string; kind: string; used: number; capacity: number; thumbs: Picture[]; chat: string | null; visibility?: SetVisibility };
  type Shown = { cards: Card[]; hasPersonal: boolean };

  const managed = $derived(mine || canCreate);
  const platformLabel = (p: string) => (p === 'KICK' ? 'Kick' : 'Twitch');
  const L = loader<Shown>(async () => {
    if (!(mine || canCreate)) {
      const sets = await publicSetsOf(accountId);
      return { hasPersonal: false, cards: sets.map((s) => ({ id: s.id, name: s.name, kind: s.kind, used: s.slotsUsed, capacity: s.capacity, thumbs: s.preview.map((m) => m.emote), chat: s.active ? 'chat' : null })) };
    }
    const lib = await libraryOf(accountId, mine);
    const active = new Map(lib.channels.filter((c) => c.activeSetId).map((c) => [c.activeSetId!, c.platform]));
    return {
      hasPersonal: lib.sets.some((s) => s.kind === 'PERSONAL'),
      cards: lib.sets.map((s) => ({ id: s.id, name: s.name, kind: s.kind, used: s.emotes.length, capacity: s.capacity, thumbs: s.emotes.map((m) => m.emote), visibility: s.visibility,
        chat: active.has(s.id) ? platformLabel(active.get(s.id)!) : null })),
    };
  });
  let dlg = $state<NewSetDialog>();
  const kindLabel = (k: string) => (k === 'PERSONAL' ? 'Personal' : k === 'GLOBAL' ? 'Global' : 'Channel');
</script>

<LoadGate source={L} label="Loading the sets…">
  {#snippet children(lib)}
    <PageHead title="Emote sets" line={managed
      ? `${count(lib.cards.length)} ${lib.cards.length === 1 ? 'set' : 'sets'}. A channel shows one set; a personal set goes with ${mine ? 'you' : 'them'} to every channel that allows it.`
      : `${count(lib.cards.length)} ${lib.cards.length === 1 ? 'set' : 'sets'} anyone can open.`}>
      {#snippet actions()}
        {#if canCreate}<button class="btn btn-s btn-primary" type="button" onclick={() => dlg?.open()}><Icon name="plus" size={15} />New set</button>{/if}
      {/snippet}
    </PageHead>
    {#if !lib.cards.length}
      <State title="No sets yet">
        {#if managed}Make one for {mine ? 'your' : 'the'} channel, then pick it in Channels.{:else}<bdi>{name || 'This account'}</bdi> hasn’t listed a set.{/if}
        {#snippet actions()}
          {#if canCreate}<button class="btn btn-s btn-primary" type="button" onclick={() => dlg?.open()}><Icon name="plus" size={15} />New set</button>{/if}
        {/snippet}
      </State>
    {:else}
      <ul class="cards">
        {#each lib.cards as s (s.id)}
          {@const p = fillPercent(s.used, s.capacity)}
          {@const mark = setMark(s.visibility)}
          <li>
            <a class="card" href="{managed ? '/me/sets' : '/sets'}/{s.id}">
              <EmoteThumbs emotes={s.thumbs} />
              <span class="row">
                <strong class="nm"><bdi>{clean(s.name)}</bdi></strong>
                {#if mark}<span class="vis" title={mark.label}><Icon name={mark.icon} size={14} /><span class="sr-only">{mark.label}</span></span>{/if}
                {#if s.chat}<span class="on" title={s.chat === 'chat' ? 'Shown in chat' : `Active in ${s.chat} chat`}><Icon name="zap" size={15} /><span class="sr-only">{s.chat === 'chat' ? 'Shown in chat' : `Active in ${s.chat} chat`}</span></span>{/if}
              </span>
              <span class="meta">{kindLabel(s.kind)} · <Capacity used={s.used} capacity={s.capacity} /></span>
              <span class="cap" role="meter" aria-label="How full" aria-valuemin={0} aria-valuemax={100} aria-valuenow={p}>
                <span class="fill" class:full={p >= 90} style:width="{p}%"></span>
                <span class="pc">{p}%</span>
              </span>
            </a>
          </li>
        {/each}
      </ul>
    {/if}
    <NewSetDialog bind:this={dlg} ownerId={accountId} {mine} hasPersonal={lib.hasPersonal} />
  {/snippet}
</LoadGate>

<style>
  .cards { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(min(250px, 100%), 1fr)); gap: 12px; }
  .card { display: grid; gap: 8px; height: 100%; padding: 12px; color: var(--text); text-decoration: none;
    background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card);
    transition: background-color var(--fast), border-color var(--fast); }
  .card:active { opacity: .7; transition: none; }
  .card:focus-visible { outline-offset: 2px; }
  @media (hover: hover) and (pointer: fine) { .card:hover { background: var(--surface-hover); border-color: var(--border-strong); } }
  .row { display: flex; align-items: center; gap: 8px; min-width: 0; margin-top: 2px; }
  .nm { flex: 1; min-width: 0; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .vis { display: inline-grid; color: var(--dim); }
  .on { display: inline-grid; color: var(--accent-text); }
  .meta { font-size: var(--t-fine); color: var(--dim); font-variant-numeric: tabular-nums; }
  .cap { position: relative; display: block; height: 18px; border-radius: var(--radius); background: var(--elevated-2); overflow: hidden; }
  .fill { position: absolute; inset: 0 auto 0 0; background: var(--accent-wash); }
  .fill.full { background: color-mix(in srgb, var(--yellow) 30%, transparent); }
  .pc { position: relative; display: grid; place-items: center; height: 100%; font-size: 12px; font-weight: 600; color: var(--text);
    font-variant-numeric: tabular-nums; }
  @media (forced-colors: active) { .fill { background: Highlight; } }
</style>
