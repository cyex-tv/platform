<script lang="ts">
  import BottomSheet from '$lib/components/BottomSheet.svelte';
  import ChatPreview, { type ChatLine } from '$lib/components/ChatPreview.svelte';
  import Chip from '$lib/components/Chip.svelte';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import Eyebrow from '$lib/components/Eyebrow.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import Popover from '$lib/components/Popover.svelte';
  import SectionHead from '$lib/components/SectionHead.svelte';
  import TagInput from '$lib/components/TagInput.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import { focusSearch } from '$lib/shell/search.svelte';

  let { data } = $props();
  let sort = $state<'top' | 'new'>('top');
  let tagList = $state(['cat', 'happy']);
  let kind = $state<'all' | 'animated' | 'static'>('all');
  let zeroWidth = $state(false);
  let sheet = $state(false);
  let chips = $state(['Animated', 'Zero-width', 'Tag: cat']);

  const e = $derived(data.emotes);
  const plain = $derived(e.filter((x) => !x.flags.zeroWidth));
  const zw = $derived(e.find((x) => x.flags.zeroWidth));
  const wide = $derived(e.find((x) => x.files[0] && x.files[0].width > x.files[0].height * 1.5));
  const lines = $derived<ChatLine[]>(
    plain.length < 6
      ? []
      : [
          { user: 'lumen_fox', parts: [{ text: 'first stream on the new set ' }, { emote: plain[0] }] },
          { user: 'quietoak', parts: [{ emote: plain[1] }, { text: ' ' }, { emote: plain[1] }, { text: ' that one is great' }] },
          { user: 'nora_byte', parts: [{ text: 'stacked: ' }, { emote: plain[2] }, ...(zw ? [{ text: ' ' }, { emote: { ...zw, zeroWidth: true } }] : [])] },
          { user: 'pixelmoth', parts: [{ text: 'wide one ' }, ...(wide ? [{ emote: wide }] : [])] },
          { user: 'sunnyfern', parts: [{ emote: plain[3] }, { text: ' gg' }] },
          { user: 'mossgate', parts: [{ text: 'brb getting tea ' }, { emote: plain[4] }] },
          { user: 'tidalwren', parts: [{ text: 'hi chat ' }, { emote: plain[5] }] },
        ],
  );
</script>

<svelte:head><title>Components - cyex.tv</title></svelte:head>

<div class="page">
  <div class="stack">
    <SectionHead level={1} size="page" eyebrow="Development" lit="top" title="Shared components" lead="Every component in src/lib/components with live data from the API." />

    <section>
      <Eyebrow lit="top">Controls</Eyebrow>
      <div class="row">
        <Segmented label="Sort" options={[{ value: 'top', label: 'Top' }, { value: 'new', label: 'New' }]} value={sort} onchange={(v) => (sort = v)} />
        <Segmented
          label="Kind"
          size="s"
          options={[{ value: 'all', label: 'All' }, { value: 'animated', label: 'Animated' }, { value: 'static', label: 'Static' }]}
          value={kind}
          onchange={(v) => (kind = v)}
        />
        <Popover label="Filters" sheet>
          {#snippet trigger(props)}
            <button class="btn btn-s" type="button" {...props}>Filters</button>
          {/snippet}
          <div class="filters">
            <p class="fine">Type</p>
            <Segmented label="Type" size="s" options={[{ value: 'all', label: 'All' }, { value: 'animated', label: 'Animated' }, { value: 'static', label: 'Static' }]} value={kind} onchange={(v) => (kind = v)} />
            <label class="check"><input type="checkbox" bind:checked={zeroWidth} /> Zero-width only <span class="fine">Emotes that sit on top of the one before them.</span></label>
          </div>
        </Popover>
        <IconButton icon="copy" label="Copy link" />
        <button class="btn btn-s" type="button" onclick={() => (sheet = true)}>Open a sheet</button>
        <button class="btn btn-s" type="button" onclick={() => focusSearch('fox')}>Header search with “fox”</button>
      </div>
      <div class="row">
        {#each chips as c (c)}<Chip label={c} onremove={() => (chips = chips.filter((x) => x !== c))} />{/each}
        <Chip label="Static chip" />
        <Chip label="Link chip" href="/dev/components" />
      </div>
    </section>

    <section>
      <Eyebrow lit="middle">Tag field</Eyebrow>
      <div class="tagdemo"><TagInput tags={tagList} onchange={(t) => (tagList = t)} label="Tags" showLabel max={5} hint="Enter adds a tag, Backspace takes the last one off. Up to 5." /></div>
    </section>

    <section>
      <Eyebrow lit="middle">Emote grid</Eyebrow>
      <EmoteGrid items={e} eager={12} label="Emotes" />
    </section>

    <section>
      <Eyebrow lit="middle">Trending grid</Eyebrow>
      <EmoteGrid items={e.slice(0, 8)} ranked label="Trending emotes" />
    </section>

    <section>
      <Eyebrow lit="middle">Loading more</Eyebrow>
      <EmoteGrid items={e.slice(0, 4)} loading skeletons={8} />
    </section>

    <section>
      <Eyebrow lit="bottom">Chat preview</Eyebrow>
      {#if lines.length}
        <div class="chats">
          <ChatPreview platform="twitch" {lines} />
          <ChatPreview platform="kick" {lines} live visible={5} />
        </div>
      {/if}
    </section>
  </div>
</div>

<BottomSheet bind:open={sheet} label="A bottom sheet">
  <p>Drag the handle down, tap outside or press Escape to close.</p>
  {#snippet footer()}
    <button class="btn" type="button" onclick={() => (sheet = false)}>Cancel</button>
    <button class="btn btn-primary" type="button" onclick={() => (sheet = false)}>Done</button>
  {/snippet}
</BottomSheet>

<style>
  .tagdemo { max-width: 360px; }
  .stack { display: grid; gap: 56px; }
  .row { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; margin-bottom: 14px; }
  .filters { display: grid; gap: 12px; padding: 8px; }
  .filters .fine { margin: 0; }
  .check { display: grid; grid-template-columns: auto 1fr; gap: 2px 10px; font-size: var(--t-ui); }
  .check .fine { grid-column: 2; }
  .chats { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 20px; }
  @media (max-width: 899.98px) { .chats { grid-template-columns: minmax(0, 1fr); } }
</style>
