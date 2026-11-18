<script lang="ts">
  // Download an emote's picture: a popover (a bottom sheet on phones) with the choices the emote's
  // file list has (still or moving for animated emotes, size 1x to 4x, WebP or AVIF) and one button.
  // No sign-in. The chosen option is bold with an accent check, never a filled box. Files come from
  // the CDN, so they are fetched and saved from a Blob (see `save`). `variant="bar"` is the phone's
  // action bar: just the icon.
  import Icon from '$lib/components/Icon.svelte';
  import Popover from '$lib/components/Popover.svelte';
  import { bytes, fileName, fileUrl, formatsAt, hasBoth, pick, save, settle, sizes, type Choice } from './download';
  import type { PageEmote } from './emote-page';

  type Props = { emote: Pick<PageEmote, 'host' | 'animated' | 'files'> & { name: string }; variant?: 'head' | 'bar' };
  let { emote, variant = 'head' }: Props = $props();

  const id = $props.id();
  const both = $derived(hasBoth(emote.files));
  let open = $state(false);
  let saying = $state<'' | 'saving' | 'saved' | 'opened'>('');

  // svelte-ignore state_referenced_locally
  let choice = $state<Choice>(settle(emote.files, { still: false, scale: 4, format: 'WEBP' }));
  const file = $derived(pick(emote.files, choice));
  const scales = $derived(sizes(emote.files, choice.still));
  const fmts = $derived(formatsAt(emote.files, choice.still, choice.scale));
  const dims = (scale: number) => {
    const f = emote.files.find((x) => x.scale === scale && x.isStatic === (both ? choice.still : x.isStatic));
    return f ? `${f.width} × ${f.height}` : '';
  };
  const label = (f: string) => (f === 'WEBP' ? 'WebP' : f === 'AVIF' ? 'AVIF' : f);

  function set(next: Partial<Choice>) {
    choice = settle(emote.files, { ...choice, ...next });
    saying = '';
  }

  async function go() {
    if (!file || saying === 'saving') return;
    saying = 'saving';
    const ok = await save(fileUrl(emote.host, file), fileName(emote.name, file, emote.animated));
    saying = ok ? 'saved' : 'opened';
    if (ok) setTimeout(() => (open = false), 500);
  }

  $effect(() => {
    if (!open) saying = '';
  });
</script>

<Popover label="Download emote" bind:open sheet width="300px" align={variant === 'head' ? 'end' : 'start'}>
  {#snippet trigger(props)}
    {#if variant === 'bar'}
      <button class="btn btn-s icon-only" type="button" aria-label="Download" {...props}><Icon name="download" /></button>
    {:else}
      <button class="btn btn-s" type="button" {...props}><Icon name="download" />Download</button>
    {/if}
  {/snippet}

  <div class="dl">
    {#if both}
      <fieldset>
        <legend class="fine">Type</legend>
        {#each [{ v: false, l: 'Animated' }, { v: true, l: 'Still frame' }] as o (o.l)}
          <label class="opt" class:on={choice.still === o.v}>
            <input type="radio" name="{id}-kind" checked={choice.still === o.v} onchange={() => set({ still: o.v })} />
            <span class="ck" aria-hidden="true">{#if choice.still === o.v}<Icon name="check" size={16} />{/if}</span>
            <span class="t">{o.l}</span>
          </label>
        {/each}
      </fieldset>
    {/if}

    <fieldset>
      <legend class="fine">Size</legend>
      {#each scales as s (s)}
        <label class="opt" class:on={choice.scale === s}>
          <input type="radio" name="{id}-size" checked={choice.scale === s} onchange={() => set({ scale: s })} />
          <span class="ck" aria-hidden="true">{#if choice.scale === s}<Icon name="check" size={16} />{/if}</span>
          <span class="t">{s}x</span>
          <span class="d fine">{dims(s)}</span>
        </label>
      {/each}
    </fieldset>

    <fieldset>
      <legend class="fine">Format</legend>
      {#each fmts as f (f)}
        <label class="opt" class:on={choice.format === f}>
          <input type="radio" name="{id}-format" checked={choice.format === f} onchange={() => set({ format: f })} />
          <span class="ck" aria-hidden="true">{#if choice.format === f}<Icon name="check" size={16} />{/if}</span>
          <span class="t">{label(f)}</span>
        </label>
      {/each}
    </fieldset>

    <button class="btn btn-primary btn-s go" type="button" onclick={go} disabled={!file || saying === 'saving'}>
      <Icon name="download" />{saying === 'saving' ? 'Saving…' : 'Download'}{#if file && saying !== 'saving' && bytes(file.size)}<span class="sz">{bytes(file.size)}</span>{/if}
    </button>
    {#if saying === 'saved' || saying === 'opened'}
      <p class="fine say">{saying === 'saved' ? 'Saved.' : 'Opened in a new tab: save it from there.'}</p>
    {/if}
    <span class="sr-only" role="status">{saying === 'saved' ? 'Saved' : saying === 'opened' ? 'Opened in a new tab' : ''}</span>
  </div>
</Popover>

<style>
  .dl { display: grid; gap: 10px; }
  fieldset { border: 0; margin: 0; padding: 0; min-width: 0; display: grid; gap: 2px; }
  legend { padding: 4px 10px 2px; color: var(--dim); }
  .opt { position: relative; display: flex; align-items: center; gap: 10px; min-height: 40px; padding: 0 10px; border-radius: var(--radius-btn);
    color: var(--muted); font-size: var(--t-ui); cursor: pointer; touch-action: manipulation; transition: background-color var(--dur-color); }
  .opt input { position: absolute; opacity: 0; inset: 0; margin: 0; cursor: pointer; }
  .opt.on { color: var(--text); font-weight: 600; }
  .ck { flex: none; width: 16px; display: grid; place-items: center; color: var(--accent); }
  .d { margin-left: auto; color: var(--dim); font-variant-numeric: tabular-nums; font-weight: 400; }
  .opt:has(input:focus-visible) { outline: 2px solid var(--accent); outline-offset: -2px; }
  .go { width: 100%; }
  .sz { color: inherit; opacity: .75; font-weight: 400; margin-left: 4px; font-variant-numeric: tabular-nums; }
  .say { margin: 0; text-align: center; color: var(--muted); }
  .icon-only { width: 44px; height: 44px; padding: 0; }
  @media (hover: hover) and (pointer: fine) { .opt:hover { background: var(--surface-2); } }
</style>
