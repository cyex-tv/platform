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
  import { animationOf, checkAnimation, checkFile, checkSize, pictureAccept, waitReady } from '$lib/cosmetics/upload';
  import { badgeOriginal, enlarge, pickBadgeFile } from '$lib/cosmetics/badgeFile';
  import { pickPaintFile } from '$lib/cosmetics/paintFile';
  import { display } from '$lib/dpr.svelte';

  // A badge's preview shows its original at its real size (up to 512 px, smaller ones in a 72 px
  // box at least; a picture made before originals existed: the ladder file for 72 px), a paint's
  // in a 120 × 48 px box (cover), the file picked from its size ladder for the screen.
  const BADGE_PREVIEW = 72;
  const PAINT_PREVIEW = { height: 48, width: 120 } as const;

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
  // The made file wouldn't load (cdn.cyex.tv refused it or it's missing): the picture is saved,
  // but it shows nowhere, so say so instead of leaving an empty preview.
  let refused = $state<string | null>(null);
  let stop: AbortController | null = null;
  onDestroy(() => {
    stop?.abort();
    if (local) URL.revokeObjectURL(local);
  });

  $effect(() => {
    busy = phase === 'checking' || phase === 'uploading' || phase === 'making';
  });

  const shown = $derived(image ?? current);
  const madeName = $derived(
    !shown || !shown.ready
      ? null
      : kind === 'badge'
        ? (badgeOriginal(shown.files)?.name ?? pickBadgeFile(shown.files.map((f) => f.name), BADGE_PREVIEW, display.dpr)?.name ?? '4x.webp')
        : (pickPaintFile(shown, false, { ...PAINT_PREVIEW, dpr: display.dpr }) ?? '1x.webp'),
  );
  const real = $derived(kind === 'badge' && shown?.ready ? badgeOriginal(shown.files) : null);
  const remote = $derived(shown && madeName ? safeUrl(`${shown.host.replace(/\/+$/, '')}/${madeName}`) : null);
  // The picked file shows at once; the made file replaces it when it's ready (unless it won't load).
  const preview = $derived(remote && refused === remote ? local : phase === 'ready' ? (remote ?? local) : (local ?? remote));
  const cdnHost = $derived(remote ? new URL(remote).host : '');

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

  async function animation(file: File) {
    try {
      const bytes =
        typeof file.arrayBuffer === 'function'
          ? await file.arrayBuffer()
          : await new Promise<ArrayBuffer>((ok, no) => {
              const r = new FileReader();
              r.onload = () => ok(r.result as ArrayBuffer);
              r.onerror = () => no(r.error);
              r.readAsArrayBuffer(file);
            });
      return animationOf(new Uint8Array(bytes));
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
    refused = null;
    // Reading the picture's size takes a moment; the form waits for it, and a newer pick wins.
    phase = 'checking';
    const bad = checkFile(file, kind);
    const dims = bad ? null : await size(file);
    const anim = !bad && dims ? await animation(file) : null;
    if (signal.aborted) return;
    const why =
      bad ??
      (dims
        ? (checkSize(kind, dims.width, dims.height) ?? checkAnimation(kind, anim, dims.width, dims.height))
        : 'That file isn’t a picture this browser can open. Try another one.');
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
    <span class="prev {kind}" class:real aria-hidden="true">
      {#if preview}<img src={preview} alt="" width={real && preview === remote ? real.width : undefined} height={real && preview === remote ? real.height : undefined} style:image-rendering={real && preview === remote ? enlarge(Math.max(real.width, real.height)) : undefined} onerror={() => { if (preview === remote) refused = remote; }} />{/if}
      {#if phase === 'uploading' || phase === 'making'}<span class="ui-spinner"></span>{/if}
    </span>
    <label class="ui-btn secondary">
      {shown || local ? 'Replace picture' : 'Choose a picture'}
      <input class="file" type="file" accept={pictureAccept(kind)} onchange={picked} />
    </label>
    <span class="ui-hint" role="status">
      {#if phase === 'checking'}Checking the file…{:else if phase === 'uploading'}Uploading…{:else if phase === 'making'}Making the files…{:else if phase === 'ready'}Ready.{:else if kind === 'badge'}Square, at least 72 × 72 px. PNG, JPEG, GIF or WebP, up to 8 MB; animated up to 120 frames and 10 seconds. Or drop it here.{:else}At least 64 px tall, up to 8:1. PNG, JPEG, GIF or WebP, up to 8 MB; animated up to 120 frames and 10 seconds. Or drop it here.{/if}
    </span>
  </div>
  {#if problem}<span class="ui-error" role="alert">{problem}</span>
  {:else if remote && refused === remote}<span class="ui-error" role="alert">The picture is made, but {cdnHost} won’t send it, so it shows nowhere yet. If every picture does this, the CDN’s firewall rule is blocking /cosmetics/.</span>{/if}
</div>

<style>
  .pick { display: flex; flex-wrap: wrap; align-items: center; gap: 12px; border-radius: var(--radius); outline: 1px dashed transparent; outline-offset: 6px; }
  .pick.over { outline-color: var(--accent); }
  .prev { position: relative; display: grid; place-items: center; flex: none; height: 48px; border-radius: var(--radius); background: var(--elevated-2); overflow: hidden; }
  .prev > * { grid-area: 1 / 1; }
  .prev.badge { width: 72px; height: 72px; }
  .prev.badge img { width: 72px; height: 72px; }
  /* The original at its real pixel size, up to 512 px, never wider than the field. */
  .prev.badge.real { width: auto; min-width: 72px; max-width: 100%; height: auto; min-height: 72px; }
  .prev.badge.real img { width: auto; height: auto; max-width: 100%; max-height: none; }
  .prev.paint { width: 120px; }
  .prev img { max-width: 100%; max-height: 100%; object-fit: contain; }
  .prev.paint img { width: 100%; height: 100%; object-fit: cover; }
  label.ui-btn { position: relative; cursor: pointer; }
  .file { position: absolute; inset: 0; opacity: 0; cursor: pointer; }
  label.ui-btn:has(.file:focus-visible) { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
</style>
