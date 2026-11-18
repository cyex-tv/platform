<script lang="ts">
  // Upload, one file or a batch (up to 20, the API's hourly limit). Each file in the queue has its
  // own name (from the file name until typed), tags, visibility and flags, and a preview at real
  // sizes on dark, light and in chat. Files are checked in the browser first (type, 5 MB,
  // 1000 × 1000 px), then sent one after another with a progress bar; a refusal is explained in
  // words and the file stays in the queue to fix or retry. One file that goes through opens its
  // page, as before; a batch stays here with a link per emote.
  import { goto } from '$app/navigation';
  import { onDestroy } from 'svelte';
  import { ApiError, uploadEmote, type Uploaded } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import { bytes, count, iso } from '$lib/format';
  import { QUEUE_MAX, fileProblem, nameFromFile, nameProblem, parseTags, sameFile, tagsProblem, uploadProblem } from '$lib/rules';
  import EmoteFlags from '$lib/tool/EmoteFlags.svelte';
  import LocalPreview from '$lib/tool/LocalPreview.svelte';
  import { notify } from '$lib/tool/notices.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';

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
    tagText: string;
    zeroWidth: boolean;
    hidden: boolean;
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
  let busy = $state(false);
  let skipped = $state<string | null>(null);
  let input: HTMLInputElement;
  let dropButton: HTMLButtonElement;
  let nextKey = 0;

  const waiting = $derived(items.filter((i) => i.status === 'ready' || i.status === 'failed'));
  const doneCount = $derived(items.filter((i) => i.status === 'done').length);

  const fileError = (i: Item) => fileProblem(i.file) ?? i.sizeError;
  const nameError = (i: Item) => (tried || i.name !== i.prefilled ? nameProblem(i.name) : null);
  const tagError = (i: Item) => tagsProblem(parseTags(i.tagText));
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

  function add(list: FileList | File[] | null | undefined) {
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
        name, prefilled: name, tagText: '', zeroWidth: false, hidden: false, nsfw: false,
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

  function drop(e: DragEvent) {
    e.preventDefault();
    over = false;
    add(e.dataTransfer?.files);
  }

  /** The first file's tags and flags, on every other file still waiting. */
  function copyToAll() {
    const [first, ...rest] = waiting;
    for (const i of rest) Object.assign(i, { tagText: first.tagText, zeroWidth: first.zeroWidth, hidden: first.hidden, nsfw: first.nsfw });
    notify(`Tags and flags copied to ${count(rest.length)} ${rest.length === 1 ? 'file' : 'files'}.`);
  }

  async function send(item: Item) {
    item.status = 'uploading';
    item.progress = 0;
    item.error = null;
    try {
      item.made = await uploadEmote(
        item.file,
        item.name.trim(),
        parseTags(item.tagText),
        { zeroWidth: item.zeroWidth, private: item.hidden, nsfw: item.nsfw },
        (p) => (item.progress = p),
      );
      item.status = 'done';
      item.open = false;
      return true;
    } catch (err) {
      item.status = 'failed';
      item.error = err instanceof ApiError ? uploadProblem(err.code, err.message) : 'The upload didn’t go through.';
      item.open = true;
      return err instanceof ApiError && err.code === 'rate_limited' ? 'stop' : false;
    }
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
    busy = true;
    const batch = [...waiting];
    let ok = 0;
    for (const item of batch) {
      const r = await send(item);
      if (r === true) ok++;
      if (r === 'stop') break; // the hourly limit: the rest would be refused the same way
    }
    busy = false;
    const failed = batch.filter((i) => i.status === 'failed').length;
    if (batch.length === 1 && ok === 1) {
      notify(`${iso(batch[0].made!.name)} uploaded. It’s pending until a moderator looks at it.`);
      await goto(`/me/emotes/${batch[0].made!.id}`);
      return;
    }
    if (ok) notify(`${count(ok)} ${ok === 1 ? 'emote' : 'emotes'} uploaded${failed ? `, ${count(failed)} didn’t go through` : ''}. New emotes are pending until a moderator looks at them.`);
    const first = batch.find((i) => i.status === 'failed');
    if (first) queueMicrotask(() => document.getElementById(`item-${first.key}`)?.querySelector<HTMLElement>('.err')?.focus());
  }
</script>

<svelte:head><title>Upload - cyex.tv</title></svelte:head>

<div class="up">
  <PageHead title="Upload" line="PNG, APNG, GIF or WebP, at most 5 MB and 1000 × 1000 px. Square or wide works best. Up to 20 at once." />

  <form class="ui-form" onsubmit={submit} novalidate>
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
    <input bind:this={input} class="sr-only" type="file" tabindex="-1" multiple accept="image/png,image/apng,image/gif,image/webp"
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
          <li class="item ui-card" class:bad class:done={item.status === 'done'} id="item-{item.key}">
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
                {:else if item.status === 'done' && item.made}
                  <span class="ui-chip pending">Pending</span>
                  <a class="ui-btn ghost small" href="/me/emotes/{item.made.id}">Open</a>
                {:else if item.status === 'failed'}
                  <span class="ui-chip rejected">Didn’t go through</span>
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
                  <label class="ui-field">
                    <span class="ui-label">Tags</span>
                    <input class="ui-input" bind:value={item.tagText} autocomplete="off" disabled={item.status === 'uploading'}
                      aria-invalid={te ? 'true' : undefined} aria-describedby="tags-{item.key}" placeholder="cat, dance" />
                    {#if te}<span class="ui-error" id="tags-{item.key}">{te}</span>{:else}<span class="ui-hint" id="tags-{item.key}">Up to 10, separated by commas. They help people find it.</span>{/if}
                  </label>
                  <EmoteFlags id="f{item.key}" bind:zeroWidth={item.zeroWidth} bind:hidden={item.hidden} bind:nsfw={item.nsfw} disabled={item.status === 'uploading'} />
                </div>
                <div class="pv">
                  <span class="ui-label">Preview</span>
                  {#if item.width && !fe}
                    <LocalPreview src={item.url} width={item.width} height={item.height} name={item.name.trim() || item.file.name} zeroWidth={item.zeroWidth} />
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

    <div class="ui-actions foot">
      <button class="ui-btn primary" type="submit" disabled={busy || (!!items.length && !waiting.length)}>
        {#if busy}<span class="ui-spinner small" aria-hidden="true"></span> Uploading…
        {:else if waiting.length > 1}Upload {count(waiting.length)} emotes
        {:else}Upload{/if}
      </button>
      {#if waiting.length > 1}
        <button type="button" class="ui-btn secondary" onclick={copyToAll} disabled={busy}>Use the first one’s tags and flags for all</button>
      {/if}
      <a class="ui-btn ghost" href="/me">{doneCount && !waiting.length ? 'Done' : 'Cancel'}</a>
    </div>
    <p class="ui-hint">
      New emotes are pending until a moderator approves them. You can use them in your own sets straight away. Only upload
      what you made or may use.
    </p>
  </form>
</div>

<style>
  .drop { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; min-height: 168px; padding: 20px;
    border-radius: var(--radius-card); border: 1px dashed var(--border-strong); background: var(--surface); color: var(--muted); font: inherit; cursor: pointer;
    transition: border-color var(--fast), background-color var(--fast); }
  .drop.slim { flex-direction: row; min-height: 56px; padding: 10px 16px; }
  .drop.over { border-color: var(--accent-border); background: var(--surface-hover); }
  @media (hover: hover) and (pointer: fine) { .drop:hover { border-color: var(--accent-border); background: var(--surface-hover); } }
  .drop.invalid { border-color: var(--red); }
  #drop-hint { margin: -8px 0 0; }

  .up { max-width: 1040px; }
  .up > :global(.head) { max-width: var(--measure-form); }
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
  @media (prefers-reduced-motion: reduce) { .bar span { transition: none; } }
  .small { min-height: 32px; height: 32px; padding: 0 10px; }
  .err { margin: 10px 0 0; outline: none; }

  .body { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 20px 24px; margin-top: 14px; padding-top: 14px; border-top: 1px solid var(--border); }
  .fields { display: grid; align-content: start; gap: 14px; min-width: 0; }
  .fields .ui-error { margin: 0; outline: none; }
  .pv { display: grid; align-content: start; gap: 8px; min-width: 0; }
  .foot { flex-wrap: wrap; padding-top: 4px; }
  .ui-spinner.small { width: 13px; height: 13px; border-color: color-mix(in srgb, var(--accent-ink) 35%, transparent); border-top-color: var(--accent-ink); }
  @container (max-width: 700px) { .body { grid-template-columns: minmax(0, 1fr); } }
  @media (max-width: 520px) {
    .bar { width: 56px; }
    .state .ui-chip { display: none; }
  }
</style>
