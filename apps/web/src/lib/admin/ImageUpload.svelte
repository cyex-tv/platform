<script lang="ts">
  // A badge or paint picture: pick a file (or drop or paste one), check it against the API's
  // rules here, upload it (POST /v1/cosmetics/images), then read it back through GraphQL until its
  // files are made (cosmeticImage.ready). `imageId` is bound: set when the picture is ready,
  // cleared when a new one is picked. `busy` is bound too, so the form can wait for the picture
  // instead of refusing to save.
  import { onDestroy } from 'svelte';
  import { gql, uploadCosmeticImage } from '$lib/api';
  import type { CosmeticImageFieldsFragment } from '$lib/gql/graphql';
  import { COSMETIC_IMAGE } from '$lib/queries';
  import { safeUrl } from '$lib/cosmetics/paint';
  import { checkFile, checkSize, PICTURE_ACCEPT, waitReady } from '$lib/cosmetics/upload';

  type Props = {
    kind: 'badge' | 'paint';
    imageId?: string;
    busy?: boolean;
    current?: CosmeticImageFieldsFragment | null;
    label: string;
  };
  let { kind, imageId = $bindable(''), busy = $bindable(false), current = null, label }: Props = $props();

  let phase = $state<'idle' | 'checking' | 'uploading' | 'making' | 'ready' | 'failed'>('idle');
  let problem = $state('');
  let image = $state<CosmeticImageFieldsFragment | null>(null);
  let local = $state<string | null>(null);
  let over = $state(false);
  let stop: AbortController | null = null;
  onDestroy(() => {
    stop?.abort();
    if (local) URL.revokeObjectURL(local);
  });

  $effect(() => {
    busy = phase === 'checking' || phase === 'uploading' || phase === 'making';
  });

  const shown = $derived(image ?? current);
  const remote = $derived(shown && shown.ready ? safeUrl(`${shown.host.replace(/\/+$/, '')}/${kind === 'badge' ? '2x' : '1x'}.webp`) : null);
  // The picked file shows at once; the made file replaces it when it's ready.
  const preview = $derived(phase === 'ready' ? (remote ?? local) : (local ?? remote));

  function setLocal(file: File | null) {
    if (local) URL.revokeObjectURL(local);
    local = file ? URL.createObjectURL(file) : null;
  }

  async function size(file: File): Promise<{ width: number; height: number } | null> {
    try {
      const bmp = await createImageBitmap(file);
      const out = { width: bmp.width, height: bmp.height };
      bmp.close();
      return out;
    } catch {
      return null;
    }
  }

  async function use(file: File | undefined | null) {
    if (!file) return;
    stop?.abort();
    stop = new AbortController();
    const signal = stop.signal;
    imageId = '';
    image = null;
    problem = '';
    // Reading the picture's size takes a moment; the form waits for it, and a newer pick wins.
    phase = 'checking';
    const bad = checkFile(file);
    const dims = bad ? null : await size(file);
    if (signal.aborted) return;
    const why = bad ?? (dims ? checkSize(kind, dims.width, dims.height) : 'That file isn’t a picture this browser can open. Try another one.');
    if (why) {
      setLocal(null);
      phase = 'failed';
      problem = why;
      return;
    }
    setLocal(file);
    phase = 'uploading';
    try {
      const up = await uploadCosmeticImage(file, kind, signal);
      phase = 'making';
      const img = await waitReady(async () => (await gql(COSMETIC_IMAGE, { id: up.id })).cosmeticImage, { signal });
      if (signal.aborted) return;
      image = img;
      imageId = img.id;
      phase = 'ready';
    } catch (err) {
      if (signal.aborted) return;
      phase = 'failed';
      problem = err instanceof Error ? err.message : 'The upload didn’t work. Try again.';
    }
  }

  function picked(e: Event & { currentTarget: HTMLInputElement }) {
    const file = e.currentTarget.files?.[0];
    // Cleared, so picking the same file again (after a failure, say) is a change too.
    e.currentTarget.value = '';
    void use(file);
  }

  function dropped(e: DragEvent) {
    e.preventDefault();
    over = false;
    void use(e.dataTransfer?.files?.[0]);
  }

  function pasted(e: ClipboardEvent) {
    const file = [...(e.clipboardData?.files ?? [])].find((f) => f.type.startsWith('image/'));
    if (!file) return;
    e.preventDefault();
    void use(file);
  }
</script>

<div class="ui-field">
  <span class="ui-label">{label}</span>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="pick"
    class:over
    ondragover={(e) => { if (e.dataTransfer?.types.includes('Files')) { e.preventDefault(); over = true; } }}
    ondragleave={() => (over = false)}
    ondrop={dropped}
    onpaste={pasted}
  >
    <span class="prev {kind}" aria-hidden="true">
      {#if preview}<img src={preview} alt="" />{/if}
      {#if phase === 'uploading' || phase === 'making'}<span class="ui-spinner"></span>{/if}
    </span>
    <label class="ui-btn secondary">
      {shown || local ? 'Replace picture' : 'Choose a picture'}
      <input class="file" type="file" accept={PICTURE_ACCEPT} onchange={picked} />
    </label>
    <span class="ui-hint" role="status">
      {#if phase === 'checking'}Checking the file…{:else if phase === 'uploading'}Uploading…{:else if phase === 'making'}Making the files…{:else if phase === 'ready'}Ready.{:else if kind === 'badge'}Square, at least 72 × 72 px. PNG, GIF or WebP, up to 5 MB. Or drop it here.{:else}At least 64 px tall, up to 8:1. PNG, GIF or WebP, up to 5 MB.{/if}
    </span>
  </div>
  {#if problem}<span class="ui-error" role="alert">{problem}</span>{/if}
</div>

<style>
  .pick { display: flex; flex-wrap: wrap; align-items: center; gap: 12px; border-radius: var(--radius); outline: 1px dashed transparent; outline-offset: 6px; }
  .pick.over { outline-color: var(--accent); }
  .prev { position: relative; display: grid; place-items: center; flex: none; height: 48px; border-radius: var(--radius); background: var(--elevated-2); overflow: hidden; }
  .prev > * { grid-area: 1 / 1; }
  .prev.badge { width: 48px; }
  .prev.paint { width: 120px; }
  .prev img { max-width: 100%; max-height: 100%; object-fit: contain; }
  .prev.paint img { width: 100%; height: 100%; object-fit: cover; }
  label.ui-btn { position: relative; cursor: pointer; }
  .file { position: absolute; inset: 0; opacity: 0; cursor: pointer; }
  label.ui-btn:has(.file:focus-visible) { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
</style>
