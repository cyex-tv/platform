<script lang="ts">
  // A badge or paint picture: pick a file, upload it (POST /v1/cosmetics/images), then ask every
  // 1,5 s whether its files are made (cosmeticImage.ready); only a ready picture can be saved.
  // `imageId` is bound: set when the picture is ready, cleared when a new one is picked.
  import { onDestroy } from 'svelte';
  import { gql, uploadCosmeticImage } from '$lib/api';
  import type { CosmeticImageFieldsFragment } from '$lib/gql/graphql';
  import { COSMETIC_IMAGE } from '$lib/queries';
  import { safeUrl } from '$lib/cosmetics/paint';

  type Props = { kind: 'badge' | 'paint'; imageId?: string; current?: CosmeticImageFieldsFragment | null; label: string };
  let { kind, imageId = $bindable(''), current = null, label }: Props = $props();

  let phase = $state<'idle' | 'uploading' | 'making' | 'ready' | 'failed'>('idle');
  let problem = $state('');
  let image = $state<CosmeticImageFieldsFragment | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let tries = 0;
  onDestroy(() => clearTimeout(timer));

  const shown = $derived(image ?? current);
  const preview = $derived(shown && shown.ready ? safeUrl(`${shown.host.replace(/\/+$/, '')}/${kind === 'badge' ? '2x' : '1x'}.webp`) : null);

  async function poll(id: string) {
    tries += 1;
    try {
      const img = (await gql(COSMETIC_IMAGE, { id })).cosmeticImage;
      if (img?.ready) {
        image = img;
        imageId = img.id;
        phase = 'ready';
        return;
      }
    } catch {
      // Ask again; a blip shouldn't lose the upload.
    }
    if (tries > 40) {
      phase = 'failed';
      problem = 'The picture is taking long to process. Try again in a minute.';
      return;
    }
    timer = setTimeout(() => poll(id), 1500);
  }

  async function picked(e: Event & { currentTarget: HTMLInputElement }) {
    const file = e.currentTarget.files?.[0];
    if (!file) return;
    clearTimeout(timer);
    imageId = '';
    image = null;
    problem = '';
    tries = 0;
    phase = 'uploading';
    try {
      const up = await uploadCosmeticImage(file, kind);
      phase = 'making';
      if (up.ready) await poll(up.id);
      else timer = setTimeout(() => poll(up.id), 1500);
    } catch (err) {
      phase = 'failed';
      problem = err instanceof Error ? err.message : 'The upload didn’t work.';
    }
  }
</script>

<div class="ui-field">
  <span class="ui-label">{label}</span>
  <div class="pick">
    <span class="prev {kind}" aria-hidden="true">
      {#if preview}<img src={preview} alt="" />{:else if phase === 'uploading' || phase === 'making'}<span class="ui-spinner"></span>{/if}
    </span>
    <label class="ui-btn secondary">
      {shown ? 'Replace picture' : 'Choose a picture'}
      <input class="file" type="file" accept="image/png,image/apng,image/gif,image/webp" onchange={picked} />
    </label>
    <span class="ui-hint" role="status">
      {#if phase === 'uploading'}Uploading…{:else if phase === 'making'}Making the files…{:else if phase === 'ready'}Ready.{:else if kind === 'badge'}Square, at least 72 × 72 px.{:else}At least 64 px tall, up to 8:1.{/if}
    </span>
  </div>
  {#if problem}<span class="ui-error" role="alert">{problem}</span>{/if}
</div>

<style>
  .pick { display: flex; flex-wrap: wrap; align-items: center; gap: 12px; }
  .prev { display: grid; place-items: center; flex: none; height: 48px; border-radius: var(--radius); background: var(--elevated-2); overflow: hidden; }
  .prev.badge { width: 48px; }
  .prev.paint { width: 120px; }
  .prev img { max-width: 100%; max-height: 100%; object-fit: contain; }
  .prev.paint img { width: 100%; height: 100%; object-fit: cover; }
  label.ui-btn { position: relative; cursor: pointer; }
  .file { position: absolute; inset: 0; opacity: 0; cursor: pointer; }
  label.ui-btn:has(.file:focus-visible) { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
</style>
