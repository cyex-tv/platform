<script lang="ts">
  // The upload form, one file or a batch (up to 20, the API's hourly limit): in the site-wide
  // upload dialog (UploadDialog) and on /me/upload as a page. Each file in the queue has its own
  // name (from the file name until typed), tags (TagInput: Enter or a comma adds one), visibility
  // and flags, and a preview at real sizes (1x to 4x). Wide is worked out from the
  // picture, as the API does. Files are checked in the browser first (type, 8 MB, 1000 × 1000 px),
  // then sent one after another with a progress bar; a refusal is explained in words and the file
  // stays in the queue to fix or retry. A file that sends nothing for 30 s stalls (and can be retried);
  // Cancel on the sending row stops it. Uploading needs the guidelines box ticked once per batch.
  // One file that goes through opens its page; a batch stays with a link per emote.
  // "Upload as" (ADR 0014) lists you and the accounts where you hold Upload on every set; you by
  // default, or the account in ?for= when you may upload for it. The account owns what you upload.
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { onDestroy } from 'svelte';
  import { flip } from 'svelte/animate';
  import { fade, fly } from 'svelte/transition';
  import { ApiError, uploadEmote, type EmoteVisibility, type Uploaded } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import TagInput from '$lib/components/TagInput.svelte';
  import { bytes, clean, count, iso } from '$lib/format';
  import { dur } from '$lib/motion';
  import RetryNote from '$lib/components/RetryNote.svelte';
  import { quiet } from '$lib/quiet';
  import { editorOf, type Grant } from '$lib/profile/data.svelte';
  import { emoteCan, uploadTargets } from '$lib/tool/editors';
  import { session } from '$lib/tool/session.svelte';
  import {
    QUEUE_MAX,
    TAG_MAX,
    UPLOAD_ACCEPT,
    cleanTag,
    fileProblem,
    isWide,
    nameFromFile,
    nameProblem,
    sameFile,
    tagsProblem,
    uploadProblem,
  } from '$lib/rules';
  import EmoteFlags from '$lib/tool/EmoteFlags.svelte';
  import LocalPreview from '$lib/tool/LocalPreview.svelte';
  import { notify } from '$lib/tool/notices.svelte';

  type Props = {
    /** In the dialog: the form scrolls inside it and Cancel closes it. */
    inDialog?: boolean;
    /** Sending right now (the dialog won't close while it is). */
    busy?: boolean;
    onclose?: () => void;
  };
  let { inDialog = false, busy = $bindable(false), onclose }: Props = $props();

  type Status = 'ready' | 'uploading' | 'done' | 'failed';
  type Item = {
    key: number;
    file: File;
    url: string;
    width: number;
    height: number;
    sizeError: string | null;
    name: string;
    prefilled: string;
    tags: string[];
    zeroWidth: boolean;
    visibility: EmoteVisibility;
    nsfw: boolean;
    status: Status;
    progress: number;
    error: string | null;
    made: Uploaded | null;
    open: boolean;
  };

  let items = $state<Item[]>([]);
  let over = $state(false);
  let tried = $state(false);
  let accepted = $state(false);
  let skipped = $state<string | null>(null);
  let input: HTMLInputElement;
  let dropButton: HTMLButtonElement;
  let acceptBox: HTMLInputElement;
  let nextKey = 0;
  /** The running upload of each file, so its Cancel can stop it. */
  const running = new Map<number, AbortController>();

  let grants = $state<readonly Grant[] | null>(null);
  let grantsFailed = $state(false);
  function loadGrants() {
    grantsFailed = false;
    void quiet(editorOf().then((g) => (grants = g)), 'upload-as list', () => (grantsFailed = true));
  }
  $effect(() => {
    if (session.me) loadGrants();
  });
  const me = $derived(session.me ? { id: session.me.id, displayName: session.me.identities[0]?.displayName ?? 'You' } : null);
  const targets = $derived(uploadTargets(me, grants));
  let picked = $state<string | null>(null);
  // Your own account unless you picked another, or the page asked for one you may upload for.
  const asked = $derived(page.url?.searchParams.get('for') ?? null);
  const owner = $derived(targets.find((t) => t.id === (picked ?? asked)) ?? targets[0] ?? null);
  const forOther = $derived(!!owner && !owner.self);
  /** Where an upload opens: its edit page when you may edit it, else the account it went to. */
  const openHref = (id: string) =>
    !forOther || emoteCan(grants, owner?.id, session.me?.id, 'EMOTE_MANAGE') ? `/me/emotes/${id}` : `/users/${encodeURIComponent(owner!.id)}`;

  const waiting = $derived(items.filter((i) => i.status === 'ready' || i.status === 'failed'));
  const doneCount = $derived(items.filter((i) => i.status === 'done').length);

  const fileError = (i: Item) => fileProblem(i.file) ?? i.sizeError;
  const nameError = (i: Item) => (tried || i.name !== i.prefilled ? nameProblem(i.name) : null);
  const tagError = (i: Item) => tagsProblem(i.tags);
  const problem = (i: Item) => fileError(i) ?? nameProblem(i.name) ?? tagError(i);
  const twin = (i: Item) => !!i.name.trim() && items.some((o) => o !== i && o.status !== 'done' && o.name.trim() === i.name.trim());

  onDestroy(() => items.forEach((i) => URL.revokeObjectURL(i.url)));

  /** Reads the picture's size, so a too-big or unreadable file is named before the upload. */
  async function measure(item: Item) {
    if (fileProblem(item.file)) return;
    try {
      const bmp = await createImageBitmap(item.file);
      const it = items.find((x) => x.key === item.key);
      if (it) {
        it.width = bmp.width;
        it.height = bmp.height;
        if (bmp.width > 1000 || bmp.height > 1000) it.sizeError = `It’s ${bmp.width} × ${bmp.height} px; at most 1000 × 1000.`;
      }
      bmp.close();
    } catch {
      const it = items.find((x) => x.key === item.key);
      if (it) it.sizeError = 'That file doesn’t open as an image.';
    }
  }

  export function add(list: FileList | File[] | null | undefined) {
    if (!list?.length) return;
    const notes: string[] = [];
    let twice = 0;
    let tooMany = 0;
    const firstNew = items.length;
    for (const file of list) {
      if (items.some((i) => i.status !== 'done' && sameFile(i.file, file))) {
        twice++;
        continue;
      }
      if (items.filter((i) => i.status !== 'done').length >= QUEUE_MAX) {
        tooMany++;
        continue;
      }
      const name = nameFromFile(file.name);
      const item: Item = {
        key: nextKey++, file, url: URL.createObjectURL(file), width: 0, height: 0, sizeError: null,
        name, prefilled: name, tags: [], zeroWidth: false, visibility: 'PUBLIC', nsfw: false,
        status: 'ready', progress: 0, error: null, made: null, open: false,
      };
      items.push(item);
      measure(item);
    }
    // The first new file opens, so its fields are right there.
    if (items.length > firstNew) items.forEach((i, n) => (i.open = n === firstNew));
    if (twice) notes.push(`${twice === 1 ? 'One file was' : `${count(twice)} files were`} already in the list.`);
    if (tooMany) notes.push(`${count(tooMany)} left out: ${QUEUE_MAX} at a time, as many as the hourly limit allows.`);
    skipped = notes.join(' ') || null;
  }

  function remove(item: Item) {
    URL.revokeObjectURL(item.url);
    items = items.filter((i) => i.key !== item.key);
    if (!items.length) queueMicrotask(() => dropButton?.focus());
  }

  /** The finished uploads leave the list (closing the dialog after a batch starts it fresh). */
  export function clearDone() {
    for (const i of items) if (i.status === 'done') URL.revokeObjectURL(i.url);
    items = items.filter((i) => i.status !== 'done');
    if (!items.length) tried = false;
  }

  function drop(e: DragEvent) {
    e.preventDefault();
    over = false;
    add(e.dataTransfer?.files);
  }

  /** The first file's tags and flags, on every other file still waiting. */
  function copyToAll() {
    const [first, ...rest] = waiting;
    for (const i of rest) Object.assign(i, { tags: [...first.tags], zeroWidth: first.zeroWidth, visibility: first.visibility, nsfw: first.nsfw });
    notify(`Tags and flags copied to ${count(rest.length)} ${rest.length === 1 ? 'file' : 'files'}.`);
  }

  /** Stops the file that is being sent; it goes back to waiting, and the rest of the batch waits too. */
  function cancel(item: Item) {
    running.get(item.key)?.abort();
  }

  async function send(item: Item) {
    item.status = 'uploading';
    item.progress = 0;
    item.error = null;
    const ctl = new AbortController();
    running.set(item.key, ctl);
    try {
      item.made = await uploadEmote(
        item.file,
        item.name.trim(),
        item.tags,
        { zeroWidth: item.zeroWidth, visibility: item.visibility, nsfw: item.nsfw },
        (p) => (item.progress = p),
        forOther ? owner!.id : null,
        { signal: ctl.signal },
      );
      item.status = 'done';
      item.open = false;
      return true;
    } catch (err) {
      if (err instanceof ApiError && err.code === 'aborted') {
        item.status = 'ready';
        item.progress = 0;
        return 'stop';
      }
      item.status = 'failed';
      item.error = err instanceof ApiError ? uploadProblem(err.code, err.message) : 'The upload didn’t go through.';
      item.open = true;
      return err instanceof ApiError && err.code === 'rate_limited' ? 'stop' : false;
    } finally {
      running.delete(item.key);
    }
  }

  /** Sends one failed file again (the row's Retry). */
  function retry(item: Item) {
    if (busy || !accepted) return;
    void run([item]);
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (busy) return;
    tried = true;
    if (!waiting.length) {
      dropButton.focus();
      return;
    }
    const bad = waiting.find(problem);
    if (bad) {
      items.forEach((i) => (i.open = i === bad));
      queueMicrotask(() => document.getElementById(`item-${bad.key}`)?.querySelector<HTMLElement>('[aria-invalid="true"], .drop-again, input')?.focus());
      return;
    }
    if (!accepted) {
      acceptBox.focus();
      return;
    }
    await run([...waiting]);
  }

  async function run(batch: Item[]) {
    busy = true;
    let ok = 0;
    for (const item of batch) {
      const r = await send(item);
      if (r === true) ok++;
      if (r === 'stop') break; // the hourly limit: the rest would be refused the same way
    }
    busy = false;
    const failed = batch.filter((i) => i.status === 'failed').length;
    if (batch.length === 1 && ok === 1 && items.length === 1) {
      const made = batch[0].made!;
      notify(`${iso(made.name)} uploaded${forOther ? ` for ${iso(clean(owner!.displayName))}` : ''}. It’s pending until a moderator looks at it.`);
      const href = openHref(made.id);
      clearDone();
      accepted = false;
      tried = false;
      onclose?.();
      await goto(href);
      return;
    }
    if (ok) notify(`${count(ok)} ${ok === 1 ? 'emote' : 'emotes'} uploaded${failed ? `, ${count(failed)} didn’t go through` : ''}. New emotes are pending until a moderator looks at them.`);
    const first = batch.find((i) => i.status === 'failed');
    if (first) queueMicrotask(() => document.getElementById(`item-${first.key}`)?.querySelector<HTMLElement>('.err')?.focus());
  }
</script>

<form class="ui-form up-form" class:in-dialog={inDialog} onsubmit={submit} novalidate>
  {#if grantsFailed}
    <RetryNote onretry={loadGrants}>Couldn’t load the accounts you can upload for. You can still upload as yourself.</RetryNote>
  {/if}
  {#if targets.length > 1}
    <label class="ui-field as">
      <span class="ui-label">Upload as</span>
      <select class="ui-input" value={owner?.id} onchange={(e) => (picked = e.currentTarget.value)} disabled={busy} aria-describedby="as-hint">
        {#each targets as t (t.id)}<option value={t.id}>{t.self ? `${clean(t.displayName)} (you)` : clean(t.displayName)}</option>{/each}
      </select>
      <span class="ui-hint" id="as-hint">{forOther ? `${clean(owner!.displayName)} owns what you upload here; it counts toward both your hourly limits.` : 'You own what you upload. You can also upload for accounts you edit.'}</span>
    </label>
  {/if}
  <button
    type="button"
    class="drop"
    class:over
    class:slim={items.length > 0}
    class:invalid={tried && !items.length}
    bind:this={dropButton}
    aria-describedby="drop-hint"
    onclick={() => input.click()}
    ondragover={(e) => { e.preventDefault(); over = true; }}
    ondragleave={() => (over = false)}
    ondrop={drop}
  >
    <Icon name="upload" size={items.length ? 18 : 26} />
    <span>{items.length ? 'Add more images' : 'Drop images here, or pick them'}</span>
  </button>
  <input bind:this={input} class="sr-only" type="file" tabindex="-1" multiple accept={UPLOAD_ACCEPT}
    onchange={(e) => { add(e.currentTarget.files); e.currentTarget.value = ''; }} />
  <p class="ui-hint" id="drop-hint">
    {#if tried && !items.length}<span class="ui-error">Pick at least one image.</span>
    {:else if skipped}{skipped}
    {:else}Each file gets its own name, tags and flags. The name comes from the file name until you change it.{/if}
  </p>

  {#if items.length}
    <ol class="queue" aria-label="Files to upload">
      {#each items as item (item.key)}
        {@const fe = fileError(item)}
        {@const ne = nameError(item)}
        {@const te = tagError(item)}
        {@const bad = !!(fe || (tried && (nameProblem(item.name) || te)))}
        {@const wide = !!item.width && isWide(item.width, item.height)}
        <li class="item ui-card" class:bad class:done={item.status === 'done'} id="item-{item.key}" in:fly={{ y: 8, duration: dur(200) }} out:fade={{ duration: dur(120) }} animate:flip={{ duration: dur(200) }}>
          <div class="head">
            <button type="button" class="toggle" aria-expanded={item.open} aria-controls="body-{item.key}" onclick={() => (item.open = !item.open)} disabled={item.status === 'done'}>
              <span class="thumb"><img src={item.url} alt="" /></span>
              <span class="who">
                <strong><bdi>{item.status === 'done' && item.made ? iso(item.made.name) : item.name.trim() || item.file.name}</bdi></strong>
                <span class="ui-hint"><bdi class="fname">{item.file.name}</bdi> · <span class="ui-num">{bytes(item.file.size)}</span>{#if item.width} · <span class="ui-num">{item.width} × {item.height}</span>{/if}</span>
              </span>
            </button>
            <span class="state">
              {#if item.status === 'uploading'}
                <span class="bar" role="progressbar" aria-label="Uploading {item.name}" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(item.progress * 100)}><span style:transform="scaleX({Math.max(0.04, item.progress)})"></span></span>
                <button type="button" class="ui-btn ghost small" onclick={() => cancel(item)} aria-label="Cancel uploading {item.file.name}">Cancel</button>
              {:else if item.status === 'done' && item.made}
                <span class="ui-chip pending">Pending</span>
                <a class="ui-btn ghost small" href={openHref(item.made.id)} onclick={() => onclose?.()}>Open</a>
              {:else if item.status === 'failed'}
                <span class="ui-chip rejected">Didn’t go through</span>
                <button type="button" class="ui-btn secondary small" onclick={() => retry(item)} disabled={busy || !accepted} aria-label="Retry {item.file.name}">Retry</button>
              {:else if bad}
                <span class="ui-chip rejected">Needs a fix</span>
              {/if}
              {#if item.status !== 'uploading' && item.status !== 'done'}
                <IconButton icon="x" label="Remove {item.file.name}" size="s" tip="top" onclick={() => remove(item)} disabled={busy} />
              {/if}
            </span>
          </div>

          {#if item.error}<p class="ui-error err" role="alert" tabindex="-1">{item.error}</p>
          {:else if fe}<p class="ui-error err drop-again" tabindex="-1">{fe} Remove it, or fix the file and add it again.</p>{/if}

          {#if item.open && item.status !== 'done'}
            <div class="body" id="body-{item.key}">
              <div class="fields">
                <label class="ui-field">
                  <span class="ui-label">Name</span>
                  <input class="ui-input" bind:value={item.name} maxlength="100" autocomplete="off" spellcheck="false" disabled={item.status === 'uploading'}
                    aria-invalid={ne ? 'true' : undefined} aria-describedby="name-{item.key}" placeholder="catJAM" />
                  {#if ne}<span class="ui-error" id="name-{item.key}">{ne}</span>
                  {:else if twin(item)}<span class="ui-hint" id="name-{item.key}">Another file here has this name too. That works, but chat can only show one per set.</span>
                  {:else}<span class="ui-hint" id="name-{item.key}">What people type in chat. No spaces.</span>{/if}
                </label>
                <div class="ui-field">
                  <TagInput label="Tags" showLabel tags={item.tags} onchange={(t) => (item.tags = t)} max={TAG_MAX} clean={cleanTag}
                    placeholder="cat, dance" hint="Enter or a comma adds a tag. Up to {TAG_MAX}; they help people find it." disabled={item.status === 'uploading'} id="tags-{item.key}" />
                  {#if te}<span class="ui-error">{te}</span>{/if}
                </div>
                <EmoteFlags id="f{item.key}" bind:zeroWidth={item.zeroWidth} bind:visibility={item.visibility} bind:nsfw={item.nsfw} disabled={item.status === 'uploading'} />
                <div class="ui-row wide-row">
                  <div><strong>Wide</strong><span class="ui-hint">{#if !item.width}Worked out from the picture.{:else if wide}Yes: it’s at least 1,75 times as wide as tall, so it shows as a wide emote.{:else}No: under 1,75 times as wide as tall. Set from the picture, not here.{/if}</span></div>
                  {#if wide}<span class="ui-chip">Wide</span>{/if}
                </div>
              </div>
              <div class="pv">
                <span class="ui-label">Preview</span>
                {#if item.width && !fe}
                  <LocalPreview src={item.url} width={item.width} height={item.height} name={item.name.trim() || item.file.name} />
                {:else}
                  <p class="ui-hint">{fe ? 'No preview: the file can’t be uploaded as it is.' : 'Reading the image…'}</p>
                {/if}
              </div>
            </div>
          {/if}
        </li>
      {/each}
    </ol>
  {/if}

  <div class="accept">
    <label class="ui-check">
      <input type="checkbox" bind:this={acceptBox} bind:checked={accepted} disabled={busy}
        aria-invalid={tried && !accepted && waiting.length ? 'true' : undefined} aria-describedby="accept-note" />
      <span>I accept the <a href="/guidelines" target="_blank" rel="noopener">emote guidelines</a><span class="sr-only"> (opens in a new tab)</span></span>
    </label>
    <p class="ui-hint" id="accept-note">
      {#if tried && !accepted && waiting.length}<span class="ui-error">Tick the box to upload.</span>
      {:else}Only upload what you made or may use. New emotes are pending until a moderator approves them; you can use them in your own sets straight away.{/if}
    </p>
  </div>

  <div class="ui-actions foot">
    <button class="ui-btn primary" type="submit" disabled={busy || (!!items.length && !waiting.length)}>
      {#if busy}<span class="ui-spinner small" aria-hidden="true"></span> Uploading…
      {:else if waiting.length > 1}Upload {count(waiting.length)} emotes
      {:else}Upload{/if}
    </button>
    {#if waiting.length > 1}
      <button type="button" class="ui-btn secondary" onclick={copyToAll} disabled={busy}>Use the first one’s tags and flags for all</button>
    {/if}
    {#if inDialog}
      <button type="button" class="ui-btn ghost" onclick={() => onclose?.()} disabled={busy}>{doneCount && !waiting.length ? 'Done' : 'Close'}</button>
    {:else}
      <a class="ui-btn ghost" href="/me">{doneCount && !waiting.length ? 'Done' : 'Cancel'}</a>
    {/if}
  </div>
</form>

<style>
  .drop { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; min-height: 168px; padding: 20px;
    border-radius: var(--radius-card); border: 1px dashed var(--border-strong); background: var(--surface); color: var(--muted); font: inherit; cursor: pointer;
    transition: border-color var(--fast), background-color var(--fast); }
  .in-dialog .drop { min-height: 140px; }
  .drop.slim { flex-direction: row; min-height: 56px; padding: 10px 16px; }
  .drop.over { border-color: var(--accent-border); background: var(--surface-hover); }
  @media (hover: hover) and (pointer: fine) { .drop:hover { border-color: var(--accent-border); background: var(--surface-hover); } }
  .drop.invalid { border-color: var(--red); }
  #drop-hint { margin: -8px 0 0; }
  .as { max-width: 420px; }

  .queue { container-type: inline-size; list-style: none; margin: 0; padding: 0; display: grid; gap: 12px; }
  .item { padding: 12px 14px; }
  .item.bad { border-color: color-mix(in srgb, var(--red) 55%, var(--border)); }
  .head { display: flex; align-items: center; gap: 12px; min-width: 0; }
  .toggle { flex: 1; display: flex; align-items: center; gap: 12px; min-width: 0; min-height: 48px; padding: 0; border: 0; background: none;
    color: inherit; font: inherit; text-align: left; cursor: pointer; border-radius: var(--radius); }
  .toggle:disabled { cursor: default; }
  .toggle:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  .thumb { flex: none; display: grid; place-items: center; width: 48px; height: 48px; border-radius: var(--radius); background: var(--stage-dark); overflow: hidden; }
  .thumb img { max-width: 44px; max-height: 44px; object-fit: contain; }
  .who { display: grid; min-width: 0; line-height: 1.35; }
  .who strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }
  .who .ui-hint { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .fname { unicode-bidi: isolate; }
  .state { flex: none; display: flex; align-items: center; gap: 8px; }
  .bar { display: block; width: 96px; height: 4px; border-radius: 2px; background: var(--border); overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--accent); transform-origin: 0 50%; transition: transform var(--fast) linear; }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .bar span { transition: none; } }
  .small { min-height: 32px; height: 32px; padding: 0 10px; }
  .err { margin: 10px 0 0; outline: none; }

  .body { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 20px 24px; margin-top: 14px; padding-top: 14px; border-top: 1px solid var(--border); }
  .fields { display: grid; align-content: start; gap: 14px; min-width: 0; }
  .fields .ui-error { margin: 0; outline: none; }
  .wide-row { flex-wrap: nowrap; gap: 16px; }
  .wide-row .ui-hint { display: block; }
  .pv { display: grid; align-content: start; gap: 8px; min-width: 0; }

  .accept { display: grid; gap: 4px; }
  .accept .ui-check { align-items: center; gap: 10px; min-height: 40px; font-weight: 500; }
  .accept .ui-check a { color: var(--accent-text, var(--accent)); text-decoration: underline; text-underline-offset: 2px; }
  .accept .ui-hint { margin: 0; }
  .foot { flex-wrap: wrap; padding-top: 4px; }
  /* In the dialog the buttons stay at the foot of what scrolls, so a short window (a phone on its
     side, 200% zoom) never leaves Upload below the fold. The dialog's own padding is bridged by
     the negative margins, and the bar has the dialog's ground. */
  .in-dialog .foot { position: sticky; bottom: calc(-16px - env(safe-area-inset-bottom, 0px)); z-index: 2; margin: 0 -16px calc(-16px - env(safe-area-inset-bottom, 0px));
    padding: 12px 16px calc(12px + env(safe-area-inset-bottom, 0px)); background: var(--elevated); border-top: 1px solid var(--border); }
  .ui-spinner.small { width: 13px; height: 13px; border-color: color-mix(in srgb, var(--accent-ink) 35%, transparent); border-top-color: var(--accent-ink); }
  @container (max-width: 700px) { .body { grid-template-columns: minmax(0, 1fr); } }
  @media (max-width: 520px) {
    .bar { width: 56px; }
    .state .ui-chip { display: none; }
  }
</style>
