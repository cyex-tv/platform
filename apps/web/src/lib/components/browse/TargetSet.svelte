<script lang="ts">
  // The toolbar's target set: which set "Add" (the tile menu and the selection bar) goes to.
  // Signed in only. Starts as your channel's active set; a pick is remembered on this device.
  // A pick list: the chosen set is bold with an accent tick, each row shows how full it is.
  import Capacity from '$lib/components/Capacity.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Popover from '$lib/components/Popover.svelte';
  import { clean } from '$lib/format';
  import { target } from './target.svelte';

  let open = $state(false);
  $effect(() => {
    target.ensure();
  });
  const set = $derived(target.current);
</script>

<Popover label="Add to which set" sheet align="end" width="320px" bind:open>
  {#snippet trigger(props)}
    <button class="btn btn-s tgt" type="button" title="Where Add puts emotes" {...props}>
      <Icon name="folder" size={15} />
      <span class="sr-only">Adding to </span><bdi>{set ? clean(set.name) : target.sets ? 'No set' : 'Your set'}</bdi>
      <Icon name="chevron-down" size={14} />
    </button>
  {/snippet}
  <div class="pick">
    <p class="cap-line">Add puts emotes in</p>
    {#if target.error}
      <p class="msg">{target.error} <button class="link" type="button" onclick={() => target.refresh()}>Try again</button></p>
    {:else if !target.sets}
      <p class="msg">Loading your sets…</p>
    {:else if target.sets.length === 0}
      <p class="msg">You have no sets to add to yet. <a class="link" href="/me/sets">Make one</a></p>
    {:else}
      <div role="radiogroup" aria-label="Set" class="sets">
        {#each target.sets as t (t.id)}
          <label class="set" class:on={target.chosenId === t.id}>
            <input type="radio" name="target-set" value={t.id} checked={target.chosenId === t.id} onchange={() => { target.choose(t.id); open = false; }} />
            <span class="nm"><bdi>{clean(t.name)}</bdi>{#if t.active}<small>On your channel</small>{/if}</span>
            <span class="cap"><Capacity used={t.held.size} capacity={t.capacity} /></span>
            <span class="tick" aria-hidden="true"><Icon name="check" size={14} /></span>
          </label>
        {/each}
      </div>
    {/if}
  </div>
</Popover>

<style>
  .tgt { height: 34px; max-width: 220px; min-width: 0; }
  .tgt bdi { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tgt :global(svg) { flex: none; color: var(--dim); }
  .pick { display: grid; gap: 6px; }
  .cap-line { margin: 0; padding: 4px 10px 0; font-size: var(--t-label); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .sets { display: grid; gap: 1px; max-height: 320px; overflow-y: auto; }
  .set { position: relative; display: flex; align-items: center; gap: 10px; min-height: 40px; padding: 6px 10px; border-radius: var(--radius);
    font-size: var(--t-nav); color: var(--muted); cursor: pointer; }
  .set input { position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: pointer; }
  .set:has(input:focus-visible) { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .nm { flex: 1; min-width: 0; display: grid; line-height: 1.3; }
  .nm bdi { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .nm small { font-size: 12px; font-weight: 400; color: var(--dim); }
  .cap { font-size: 12px; color: var(--dim); font-variant-numeric: tabular-nums; }
  .tick { display: inline-flex; color: var(--accent-text); opacity: 0; }
  .set.on { color: var(--text); font-weight: 600; }
  .set.on .tick { opacity: 1; }
  .msg { margin: 0; padding: 4px 10px 8px; font-size: var(--t-fine); color: var(--muted); }
  .link { border: 0; background: none; padding: 0; color: var(--accent-text); font: inherit; cursor: pointer; }
  @media (hover: hover) and (pointer: fine) { .set:hover { background: var(--row-hover); color: var(--text); } }
  @media (max-width: 520px) { .tgt { max-width: 160px; } }
</style>
