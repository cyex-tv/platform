<script lang="ts">
  // An account's sets as cards (7TV's emote sets view): two rows of emotes, the name, how full it
  // is, and a mark on the set a channel shows. The owner and editors see all their sets, open them
  // in the editor and get "New set" (with SETS); everyone else sees the listed ones, which open
  // the public set page (/sets/<id>). Built for hundreds of sets: the first SETS_STEP cards, then
  // "Show more" with how many show of how many, and a name filter once there are more than a few.
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
  import { byName, moreText, SETS_FILTER_FROM, SETS_STEP, shown, showing } from '../scale';
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
  let steps = $state(1);
  let find = $state('');
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
      {@const matching = byName(lib.cards, find)}
      {@const visible = matching.slice(0, shown(matching.length, steps, SETS_STEP))}
      {#if lib.cards.length > SETS_FILTER_FROM}
        <div class="filter">
          <label class="find">
            <span class="sr-only">Filter sets by name</span>
            <Icon name="search" size={15} />
            <input class="input" type="search" placeholder="Filter sets" autocomplete="off" bind:value={find} oninput={() => (steps = 1)} />
          </label>
          {#if find.trim()}<span class="ui-hint ui-num" aria-live="polite">{count(matching.length)} of {count(lib.cards.length)}</span>{/if}
        </div>
      {/if}
      {#if !matching.length}
        <State bare title="No set matches “{find.trim()}”." />
      {/if}
      <ul class="cards contact-sheet">
        {#each visible as s (s.id)}
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
              <span class="meta">{kindLabel(s.kind)}</span>
              <span class="cap">
                <span class="bar" role="meter" aria-label="How full" aria-valuemin={0} aria-valuemax={100} aria-valuenow={p}>
                  <span class="fill" class:full={p >= 90} style:width="{p}%"></span>
                </span>
                <span class="n"><Capacity used={s.used} capacity={s.capacity} /></span>
              </span>
            </a>
          </li>
        {/each}
      </ul>
      {#if visible.length < matching.length}
        <div class="more">
          <span class="ui-hint ui-num">{showing(visible.length, matching.length)}</span>
          <button class="btn btn-s" type="button" onclick={() => steps++}>{moreText(visible.length, matching.length, SETS_STEP)}</button>
        </div>
      {/if}
    {/if}
    <NewSetDialog bind:this={dlg} ownerId={accountId} {mine} hasPersonal={lib.hasPersonal} />
  {/snippet}
</LoadGate>

<style>
  .filter { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 12px; min-height: 52px; margin-bottom: 16px; border-bottom: 1px solid var(--border); }
  .find { position: relative; display: block; width: min(280px, 100%); }
  .find :global(svg) { position: absolute; left: 10px; top: 50%; translate: 0 -50%; color: var(--muted); pointer-events: none; }
  .find .input { width: 100%; height: 34px; padding-left: 32px; font-size: var(--t-small); }
  .more { display: grid; justify-items: center; gap: 10px; padding: 24px 0 8px; }
  /* A contact sheet (redesign round 2): the sets share hairlines, no cards (app.css .contact-sheet). */
  .cards { list-style: none; margin: 0; padding: 0; grid-template-columns: repeat(auto-fill, minmax(min(250px, 100%), 1fr)); }
  .card { display: grid; gap: 8px; height: 100%; padding: 14px; color: var(--text); text-decoration: none;
    transition: background-color var(--fast); }
  .card:active { opacity: .7; transition: none; }
  .card:focus-visible { outline-offset: -2px; }
  .row { display: flex; align-items: center; gap: 8px; min-width: 0; margin-top: 2px; }
  .nm { flex: 1; min-width: 0; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .vis { display: inline-grid; color: var(--dim); }
  .on { display: inline-grid; color: var(--accent-text); }
  .meta { font-size: var(--t-fine); color: var(--dim); }
  .cap { display: flex; align-items: center; gap: 10px; font-family: var(--font-mono); font-size: 12px; color: var(--muted); font-variant-numeric: tabular-nums; }
  .bar { flex: 1; min-width: 0; height: 3px; border-radius: 2px; background: var(--border); overflow: hidden; }
  .fill { display: block; height: 100%; background: var(--accent); }
  .fill.full { background: var(--yellow); }
  .n { flex: none; }
  @media (forced-colors: active) { .fill { background: Highlight; } }
</style>
