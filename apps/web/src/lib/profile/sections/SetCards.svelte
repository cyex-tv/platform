<script lang="ts">
  // An account's sets as cards (7TV's emote sets view): two rows of emotes, the name, how full it
  // is, and a mark on the set a channel shows. "New set" for the owner and editors with SETS.
  import EmoteThumbs from './EmoteThumbs.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import { clean, count } from '$lib/format';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { libraryOf } from '../data.svelte';
  import NewSetDialog from '../NewSetDialog.svelte';

  let { accountId, mine, canCreate }: { accountId: string; mine: boolean; canCreate: boolean } = $props();

  const L = loader(() => libraryOf(accountId, mine));
  let dlg = $state<NewSetDialog>();
  const pct = (n: number, cap: number) => (cap > 0 ? Math.min(100, Math.round((n / cap) * 100)) : 0);
  const kindLabel = (k: string) => (k === 'PERSONAL' ? 'Personal' : k === 'GLOBAL' ? 'Global' : 'Channel');
</script>

<LoadGate source={L} label="Loading the sets…">
  {#snippet children(lib)}
    {@const active = new Map(lib.channels.filter((c) => c.activeSetId).map((c) => [c.activeSetId!, c.platform]))}
    <PageHead title="Emote sets" line={`${count(lib.sets.length)} ${lib.sets.length === 1 ? 'set' : 'sets'}. A channel shows one set; a personal set goes with ${mine ? 'you' : 'them'} to every channel that allows it.`}>
      {#snippet actions()}
        {#if canCreate}<button class="ui-btn primary" type="button" onclick={() => dlg?.open()}><Icon name="plus" size={15} />New set</button>{/if}
      {/snippet}
    </PageHead>
    {#if !lib.sets.length}
      <State title="No sets yet">
        Make one for {mine ? 'your' : 'the'} channel, then pick it in Channels.
        {#snippet actions()}
          {#if canCreate}<button class="ui-btn primary" type="button" onclick={() => dlg?.open()}><Icon name="plus" size={15} />New set</button>{/if}
        {/snippet}
      </State>
    {:else}
      <ul class="cards">
        {#each lib.sets as s (s.id)}
          {@const p = pct(s.emotes.length, s.capacity)}
          {@const on = active.get(s.id)}
          <li>
            <a class="card" href="/me/sets/{s.id}">
              <EmoteThumbs emotes={s.emotes.map((m) => m.emote)} />
              <span class="row">
                <strong class="nm"><bdi>{clean(s.name)}</bdi></strong>
                {#if on}<span class="on" title="Active in {on === 'KICK' ? 'Kick' : 'Twitch'} chat"><Icon name="zap" size={15} /><span class="sr-only">Active in {on === 'KICK' ? 'Kick' : 'Twitch'} chat</span></span>{/if}
              </span>
              <span class="meta">{kindLabel(s.kind)} · {count(s.emotes.length)} / {count(s.capacity)}</span>
              <span class="cap" role="meter" aria-label="How full" aria-valuemin={0} aria-valuemax={100} aria-valuenow={p}>
                <span class="fill" class:full={p >= 90} style:width="{p}%"></span>
                <span class="pc">{p}%</span>
              </span>
            </a>
          </li>
        {/each}
      </ul>
    {/if}
    <NewSetDialog bind:this={dlg} ownerId={accountId} {mine} hasPersonal={lib.sets.some((s) => s.kind === 'PERSONAL')} />
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
  .on { display: inline-grid; color: var(--accent-text); }
  .meta { font-size: var(--t-fine); color: var(--dim); font-variant-numeric: tabular-nums; }
  .cap { position: relative; display: block; height: 18px; border-radius: var(--radius); background: var(--elevated-2); overflow: hidden; }
  .fill { position: absolute; inset: 0 auto 0 0; background: var(--accent-wash); }
  .fill.full { background: color-mix(in srgb, var(--yellow) 30%, transparent); }
  .pc { position: relative; display: grid; place-items: center; height: 100%; font-size: 12px; font-weight: 600; color: var(--text);
    font-variant-numeric: tabular-nums; }
  @media (forced-colors: active) { .fill { background: Highlight; } }
</style>
