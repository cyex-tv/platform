<script lang="ts">
  // The paint on names: a large name and several sample names on the dark and light grounds,
  // then Twitch- and Kick-style chat frames with the painted name in a conversation.
  import { floorFilter } from '$lib/cosmetics/paint';
  import { like, paintStyleText } from '../css';
  import type { StudioPaint } from '../format';
  import Seg from './Seg.svelte';

  type Props = { paint: StudioPaint; imageUrl?: string | null; name: string; onname: (n: string) => void };
  let { paint, imageUrl = null, name, onname }: Props = $props();

  const SAMPLES = ['lumen_fox', 'quietoak', 'nora_byte', 'pixelmoth', 'Ab', 'sunnyfern_streams_daily'];
  const style = $derived(paintStyleText(paint, imageUrl));
  // The site's readability floor on each ground, so a pale paint looks here as it will on cyex.tv.
  const floor = $derived({ dark: floorFilter(like(paint), 'dark'), light: floorFilter(like(paint), 'light') });
  const shown = $derived(name.trim() || 'lumen_fox');
  let ground = $state<'dark' | 'light'>('dark');
  const others = [
    { user: 'quietoak', colour: '#4FD6FF', ink: '#0B6D8A' },
    { user: 'nora_byte', colour: '#FF7BD5', ink: '#A82277' },
  ];
</script>

<section class="ps-panel" aria-labelledby="ps-preview-h">
  <header><h2 id="ps-preview-h">Preview</h2></header>
  <div class="ps-body">
    <div class="ps-field">
      <label for="ps-tryname">Name to preview</label>
      <input id="ps-tryname" class="ps-text" type="text" maxlength="25" autocomplete="off" spellcheck="false" value={name} placeholder="lumen_fox" oninput={(e) => onname(e.currentTarget.value)} />
    </div>
    <div class="ps-stages">
      {#each ['dark', 'light'] as g (g)}
        <div class="ps-stage {g}">
          <span class="ps-stagelabel">{g === 'dark' ? 'Dark' : 'Light'}</span>
          <span class="ps-floor" style:filter={floor[g as 'dark' | 'light'] || null}><span class="ps-painted ps-big" {style}>{shown}</span></span>
          <div class="ps-names" style:filter={floor[g as 'dark' | 'light'] || null}>
            {#each SAMPLES as s (s)}<span class="ps-painted ps-mid" {style}>{s}</span>{/each}
          </div>
        </div>
      {/each}
    </div>
    <div class="ps-row" style:justify-content="space-between">
      <span class="ps-flabel">In chat</span>
      <Seg label="Chat ground" value={ground} onchange={(v) => (ground = v)} options={[{ value: 'dark', label: 'Dark', icon: 'moon' }, { value: 'light', label: 'Light', icon: 'sun' }]} />
    </div>
    {#each ['twitch', 'kick'] as platform (platform)}
      <figure class="ps-chat {platform} {ground}" aria-label="{platform === 'twitch' ? 'Twitch' : 'Kick'}-style chat preview">
        <figcaption class="top"><b>{platform === 'twitch' ? 'Twitch' : 'Kick'}</b><span>Chat</span></figcaption>
        <ol>
          {#each [{ who: others[0], text: 'is the new set live yet' }, { who: null, text: 'it is, try the new emotes' }, { who: others[1], text: 'that name looks great' }, { who: null, text: 'thanks, made it today' }] as line, i (i)}
            <li>
              {#if platform === 'twitch'}<span class="ts">12:0{1 + (i >> 1)}</span>{/if}
              {#if line.who}
                <span class="who" style:color={ground === 'dark' ? line.who.colour : line.who.ink}>{line.who.user}</span>
              {:else}
                <span class="ps-floor" style:filter={floor[ground] || null}><span class="who ps-painted" {style}>{shown}</span></span>
              {/if}<span class="colon">:</span>{line.text}
            </li>
          {/each}
        </ol>
      </figure>
    {/each}
  </div>
</section>
