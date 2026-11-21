<script lang="ts">
  // The upload form, one file or a batch (up to 20, the API's hourly limit): in the site-wide
  // upload dialog (UploadDialog) and on /me/upload as a page. Each file in the queue has its own
  // name (from the file name until typed), tags (TagInput: Enter or a comma adds one), visibility
  // and flags, and a preview at real sizes (1x to 4x). Wide is worked out from the
  // picture, as the API does. Files are checked in the browser first (type, 8 MB, 1000 × 1000 px),
  // then sent one after another with a progress bar; a refusal is explained in words and the file
  // stays in the queue to fix or retry. A file that sends nothing for 30 s stalls (and can be retried), and so
  // does one the API hasn't answered 30 s after its last byte (UPLOAD_STALL_MS, UPLOAD_ANSWER_MS);
  // Cancel on the sending row stops it. Uploading needs the guidelines box ticked once per batch.
  // One file that goes through opens its page; a batch stays with a link per emote.
  // "Upload as" (ADR 0014) lists you and the accounts where you hold Upload on every set; you by
  // default, or the account in ?for= when you may upload for it. The account owns what you upload.
  // A file that is already a public emote (the same bytes) says "Looks like <emote> by <owner>"
  // with a link (same-file.ts); it can still be uploaded.
  // An account may have at most pendingMax emotes waiting for review (20 unless its roles say
  // otherwise): the form says how many wait ("3 of 20 waiting for review", uploadQuota, for the
  // account picked in Upload as) and, at the cap, says why Upload is off.
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { onDestroy, type Snippet } from 'svelte';
  import { flip } from 'svelte/animate';
  import { fade, fly } from 'svelte/transition';
  import { ApiError, gql, uploadEmote, type EmoteVisibility, type Uploaded } from '$lib/api';
  import { UPLOAD_QUOTA } from '$lib/queries';
  import { quotaView, type Quota } from './quota';
  import Icon from '$lib/components/Icon.svelte';
  import Avatar from '$lib/components/Avatar.svelte';
  import FactRows from '$lib/components/FactRows.svelte';
  import Meter from '$lib/components/Meter.svelte';
  import PanelBlock from '$lib/components/PanelBlock.svelte';
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
    UPLOAD_FORMATS,
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
  import LeaveGuard from '$lib/tool/LeaveGuard.svelte';
  import { notify } from '$lib/tool/notices.svelte';
  import { sameFileEmotes, sameFileLine, type SameFileEmote } from './same-file';

  type Props = {
    /** In the dialog: the form scrolls inside it and Cancel closes it. */
    inDialog?: boolean;
    /** Sending right now (the dialog won't close while it is). */
    busy?: boolean;
    onclose?: () => void;
    /** The page: the side column (who, rules, guidelines, submit) is not drawn here; it comes out through `side`, for the page's panel. */
    aside?: boolean;
    side?: Snippet | null;
  };
  let { inDialog = false, busy = $bindable(false), onclose, aside = false, side = $bindable(null) }: Props = $props();
  const uid = $props.id();

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
    /** Public emotes made from these very bytes; empty when none (or not checked). */
    same: SameFileEmote[];
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
  // On the page the side column goes to the page's panel.
  $effect(() => {
    if (aside) side = sideAll;
    return () => (side = null);
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

  // Where the account picked stands with emotes waiting for review; null until read (or when it
  // can't be: the API still refuses at the cap, and the refusal says so).
  let quota = $state<Quota | null>(null);
  let quotaFor = 0;
  function loadQuota() {
    const id = owner?.id;
    if (!session.me || !id) return;
    const mine = ++quotaFor;
    void quiet(
      gql(UPLOAD_QUOTA, { ownerId: forOther ? id : null }).then((r) => {
        if (mine === quotaFor) quota = r.uploadQuota;
      }),
      'upload quota',
    );
  }
  $effect(() => {
    void owner?.id;
    quota = null;
    loadQuota();
  });
  const room = $derived(quotaView(quota, forOther));

  const who = $derived(session.me?.identities[0]);
  const waiting = $derived(items.filter((i) => i.status === 'ready' || i.status === 'failed'));
  const total = $derived(waiting.reduce((n, i) => n + i.file.size, 0));
  const doneCount = $derived(items.filter((i) => i.status === 'done').length);

  /** The file whose tags and flags are open under the sheet (one at a time). */
  const current = $derived(items.find((i) => i.open && i.status !== 'done') ?? null);
  function select(item: Item, on = !item.open) {
    for (const i of items) i.open = i === item && on;
  }

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

  /** Asks whether the file is already a public emote; signed in only, quietly. */
  async function checkSame(item: Item) {
    if (!session.me || fileProblem(item.file)) return;
    const found = await sameFileEmotes(item.file);
    const it = items.find((x) => x.key === item.key);
    if (it && found.length) it.same = found;
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
        status: 'ready', progress: 0, error: null, made: null, open: false, same: [],
      };
      items.push(item);
      measure(item);
      checkSame(item);
    }
    // The first new file opens, so its fields are right there.
    if (items.length > firstNew) select(items[firstNew], true);
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
    // The dialog around the form takes drops too: one drop is one add, not two.
    e.stopPropagation();
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
      if (quota) quota.pending++;
      return true;
    } catch (err) {
      if (err instanceof ApiError && err.code === 'aborted') {
        item.status = 'ready';
        item.progress = 0;
        return 'stop';
      }
      item.status = 'failed';
      item.error = err instanceof ApiError ? uploadProblem(err.code, err.message) : 'The upload didn’t go through.';
      select(item, true);
      if (err instanceof ApiError && err.code === 'too_many_pending') {
        loadQuota();
        return 'stop';
      }
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
      select(bad, true);
      queueMicrotask(() => (document.querySelector<HTMLElement>('#detail [aria-invalid="true"]') ?? document.getElementById(`item-${bad.key}`)?.querySelector<HTMLElement>('[aria-invalid="true"], .drop-again, input'))?.focus());
      return;
    }
    if (!accepted) {
      acceptBox.focus();
      return;
    }
    if (room?.full) return;
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
    loadQuota();
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

<!-- Closing or reloading the tab loses a file that is being sent and the ones waiting; on the page (not in the
     site-wide dialog, whose queue survives navigation) leaving for another page asks too. -->
<LeaveGuard dirty={busy || waiting.length > 0} inApp={!inDialog} what={busy ? 'the upload in progress and the files waiting' : 'the files waiting to upload'} />

<!-- Who you upload as, the review queue, the rules and the guidelines. -->
{#snippet sideTop()}
  <PanelBlock title="Uploading as">
    {#if grantsFailed}
      <RetryNote onretry={loadGrants}>Couldn’t load the accounts you can upload for. You can still upload as yourself.</RetryNote>
    {/if}
    {#if targets.length > 1}
      <label class="ui-field as">
        <span class="sr-only">Upload as</span>
        <select class="ui-input" value={owner?.id} onchange={(e) => (picked = e.currentTarget.value)} disabled={busy} aria-describedby="as-hint-{uid}">
          {#each targets as t (t.id)}<option value={t.id}>{t.self ? `${clean(t.displayName)} (you)` : clean(t.displayName)}</option>{/each}
        </select>
        <span class="ui-hint" id="as-hint-{uid}">{forOther ? `${clean(owner!.displayName)} owns what you upload here; it counts toward both your hourly limits.` : 'You own what you upload. You can also upload for accounts you edit.'}</span>
      </label>
    {:else if owner}
      <p class="who"><Avatar src={forOther ? null : who?.avatar} name={owner.displayName} size={24} tone="tool" /><span><bdi>{clean(owner.displayName)}</bdi></span></p>
    {/if}
  </PanelBlock>
  {#if room}
    <PanelBlock title="Review queue">
      <p class="quota" class:full={room.full} role="status" id="quota-{uid}">
        {#if room.full}<span class="ui-error">{room.refusal}</span>
        {:else}<span class="ui-mono">{room.line}</span>{#if waiting.length > room.room} <span class="ui-hint">Only {count(room.room)} more {room.room === 1 ? 'fits' : 'fit'} until some are reviewed.</span>{/if}{/if}
      </p>
      {#if quota && quota.pendingMax != null}<Meter used={quota.pending} capacity={quota.pendingMax} />{/if}
    </PanelBlock>
  {/if}
  <PanelBlock title="Rules">
    <FactRows label="Upload rules" items={[
      { label: 'Formats', value: UPLOAD_FORMATS.replace(' or ', ', '), mono: true },
      { label: 'File size', value: '8 MB at most', mono: true },
      { label: 'Picture', value: '1000 × 1000 px at most', mono: true },
      { label: 'At once', value: `Up to ${QUEUE_MAX}`, mono: true },
    ]} />
  </PanelBlock>
  <PanelBlock title="Guidelines" more={{ href: '/guidelines', label: 'Read' }}>
    <p class="pb-hint">Only upload what you made or may use. New emotes wait for a moderator; you can use them in your own sets straight away.</p>
  </PanelBlock>
{/snippet}

<!-- The submit area. -->
{#snippet sideBottom()}
  <div class="submit">
    <label class="ui-check accept">
      <input type="checkbox" bind:this={acceptBox} bind:checked={accepted} disabled={busy}
        aria-invalid={tried && !accepted && waiting.length ? 'true' : undefined} aria-describedby={tried && !accepted && waiting.length ? `accept-note-${uid}` : undefined} />
      <span>I accept the <a href="/guidelines" target="_blank" rel="noopener">emote guidelines</a><span class="sr-only"> (opens in a new tab)</span></span>
    </label>
    {#if tried && !accepted && waiting.length}<p class="pb-hint" id="accept-note-{uid}"><span class="ui-error">Tick the box to upload.</span></p>{/if}
    <p class="sum ui-mono" aria-live="polite">{#if waiting.length}{count(waiting.length)} {waiting.length === 1 ? 'file' : 'files'} · {bytes(total)}{:else}No files yet{/if}</p>
    <div class="foot">
      <button class="ui-btn primary" type="submit" form="upload-{uid}" disabled={busy || (!!items.length && !waiting.length) || !!room?.full}
        aria-describedby={room ? `quota-${uid}` : undefined}>
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
  </div>
{/snippet}

{#snippet sideAll()}{@render sideTop()}{@render sideBottom()}{/snippet}

<form class="up-form" class:in-dialog={inDialog} id="upload-{uid}" onsubmit={submit} novalidate>
<div class="cols" class:aside>
<div class="main">
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
    <Icon name="upload" size={items.length ? 18 : 24} />
    <span>{items.length ? 'Add more images' : 'Drop images here, or pick them'}</span>
  </button>
  <input bind:this={input} class="sr-only" type="file" tabindex="-1" multiple accept={UPLOAD_ACCEPT}
    onchange={(e) => { add(e.currentTarget.files); e.currentTarget.value = ''; }} />
  <p class="ui-hint" id="drop-hint">
    {#if tried && !items.length}<span class="ui-error">Pick at least one image.</span>
    {:else if skipped}{skipped}
    {:else}Each file gets its own name, tags and flags. The name is what people type in chat, no spaces; it comes from the file name until you change it.{/if}
  </p>

  {#if items.length}
    <ol class="queue contact-sheet" aria-label="Files to upload">
      {#each items as item (item.key)}
        {@const fe = fileError(item)}
        {@const ne = nameError(item)}
        {@const te = tagError(item)}
        {@const bad = !!(fe || (tried && (nameProblem(item.name) || te)))}
        <li class="item" class:bad class:done={item.status === 'done'} class:on={item.open && item.status !== 'done'} id="item-{item.key}" in:fly={{ y: 8, duration: dur(200) }} out:fade={{ duration: dur(120) }} animate:flip={{ duration: dur(200) }}>
          <button type="button" class="pick" aria-expanded={item.open && item.status !== 'done'} aria-controls={item.open && item.status !== 'done' ? 'detail' : undefined}
            aria-label="Tags and flags of {item.name.trim() || item.file.name}" onclick={() => select(item)} disabled={item.status === 'done'}>
            <img src={item.url} alt="" />
          </button>
          {#if item.status !== 'uploading' && item.status !== 'done'}
            <span class="rm"><IconButton icon="x" label="Remove {item.file.name}" size="s" tip="top" onclick={() => remove(item)} disabled={busy} /></span>
          {/if}
          <div class="info">
            {#if item.status === 'done' && item.made}
              <strong class="nm done-nm"><bdi>{iso(item.made.name)}</bdi></strong>
            {:else}
              <input class="nm" bind:value={item.name} maxlength="100" autocomplete="off" spellcheck="false" disabled={item.status === 'uploading'}
                aria-label="Name of {item.file.name}" aria-invalid={ne ? 'true' : undefined} aria-describedby={ne ? `name-${item.key}` : undefined} placeholder="catJAM" />
            {/if}
            {#if ne}<span class="ui-error small-note" id="name-{item.key}">{ne}</span>
            {:else if twin(item)}<span class="ui-hint small-note">Another file here has this name too. That works, but chat can only show one per set.</span>{/if}
            <p class="meta ui-mono" title={item.file.name}>{bytes(item.file.size)}{#if item.width}{' · '}{item.width} × {item.height}{/if}</p>

            {#if item.error}<p class="ui-error err" role="alert" tabindex="-1">{item.error}</p>
            {:else if fe}<p class="ui-error err drop-again" tabindex="-1">{fe} Remove it, or fix the file and add it again.</p>{/if}

            <div class="line">
              {#if item.status === 'uploading'}
                <span class="st ui-mono">Uploading {Math.round(item.progress * 100)} %</span>
                <span class="bar" role="progressbar" aria-label="Uploading {item.name}" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(item.progress * 100)}><span style:transform="scaleX({Math.max(0.04, item.progress)})"></span></span>
                <button type="button" class="ui-btn ghost small" onclick={() => cancel(item)} aria-label="Cancel uploading {item.file.name}">Cancel</button>
              {:else if item.status === 'done' && item.made}
                <span class="st">Pending review</span>
                <a class="ui-btn ghost small" href={openHref(item.made.id)} onclick={() => onclose?.()}>Open</a>
              {:else if item.status === 'failed'}
                <span class="st bad">Didn’t go through</span>
                <button type="button" class="ui-btn secondary small" onclick={() => retry(item)} disabled={busy || !accepted} aria-label="Retry {item.file.name}">Retry</button>
              {:else if bad}
                <span class="st bad">Needs a fix</span>
              {:else}
                <span class="st ok">Ready</span>
              {/if}
            </div>

            {#if item.same.length && item.status !== 'done'}
              {@const line = sameFileLine(item.same)}
              <p class="same" role="status">
                <span>Looks like <a href="/emotes/{encodeURIComponent(item.same[0].id)}" target="_blank" rel="noopener"><bdi>{clean(item.same[0].name)}</bdi><span class="sr-only"> (opens in a new tab)</span></a>{#if line.by}{' '}<bdi>{clean(line.by)}</bdi>{/if}{line.more}, already on cyex.tv. You can add that one to your sets, or upload this anyway.</span>
              </p>
            {/if}
          </div>
        </li>
      {/each}
    </ol>

    {#if current}
      {@const item = current}
      {@const fe = fileError(item)}
      {@const te = tagError(item)}
      {@const wide = !!item.width && isWide(item.width, item.height)}
      <section class="detail" id="detail" aria-label="Tags and flags of {item.name.trim() || item.file.name}">
        <h3 class="dh">Details <bdi class="dn">{item.name.trim() || item.file.name}</bdi></h3>
        <div class="body">
          <div class="fields">
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
      </section>
    {/if}
  {/if}
</div>
{#if !aside}
  <div class="sidetop">{@render sideTop()}</div>
  <div class="sidebot">{@render sideBottom()}</div>
{/if}
</div>
</form>

<style>
  /* One flat sheet of files, then the picked file's details, then (in the dialog and on a page too
     narrow for the panel) who you upload as, the rules, the guidelines and the submit area. Wide
     dialog: the files scroll on the left, the right column keeps Upload in view. Phones: one column. */
  .up-form { container-type: inline-size; }
  .in-dialog.up-form { height: 100%; }
  .cols { display: grid; gap: 0; }
  .cols.aside { display: block; }
  .main { display: grid; align-content: start; gap: 12px; min-width: 0; }
  .sidetop, .sidebot { min-width: 0; }
  .sidetop :global(.pb), .sidebot { padding-inline: 0; }
  .in-dialog .cols { height: 100%; overflow: hidden auto; overscroll-behavior: contain; align-content: start; }
  .in-dialog .main { padding: 16px; }
  .in-dialog .sidetop { padding: 4px 16px 8px; border-top: 1px solid var(--border); }
  .in-dialog .sidebot { position: sticky; bottom: 0; z-index: 2; padding: 12px 16px calc(12px + env(safe-area-inset-bottom, 0px)); background: var(--elevated); border-top: 1px solid var(--border); }
  @container (min-width: 760px) {
    .in-dialog .cols { grid-template-columns: minmax(0, 1fr) 300px; grid-template-rows: minmax(0, 1fr) auto; overflow: hidden; }
    .in-dialog .main { grid-column: 1; grid-row: 1 / span 2; overflow: hidden auto; overscroll-behavior: contain; min-height: 0; }
    .in-dialog .sidetop { grid-column: 2; grid-row: 1; overflow: hidden auto; min-height: 0; border-top: 0; border-left: 1px solid var(--border); }
    .in-dialog .sidebot { grid-column: 2; grid-row: 2; position: static; border-left: 1px solid var(--border); }
  }
  @container (min-width: 760px) {
    :not(.in-dialog) > .cols:not(.aside) { grid-template-columns: minmax(0, 1fr) 300px; grid-template-areas: 'main top' 'main bottom'; grid-template-rows: auto 1fr; column-gap: 24px; align-items: start; }
    :not(.in-dialog) > .cols:not(.aside) > .main { grid-area: main; }
    :not(.in-dialog) > .cols:not(.aside) > .sidetop { grid-area: top; }
    :not(.in-dialog) > .cols:not(.aside) > .sidebot { grid-area: bottom; position: sticky; top: 0; }
  }

  /* The drop zone: a dashed hairline area, no card. */
  .drop { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px; min-height: 144px; padding: 20px;
    border-radius: 0; border: 1px dashed var(--border-strong); background: none; color: var(--muted); font: inherit; font-size: var(--t-small); cursor: pointer;
    transition: border-color var(--fast), background-color var(--fast), color var(--fast); }
  .drop.slim { flex-direction: row; min-height: 48px; padding: 8px 16px; }
  .drop.over { border-color: var(--accent-border); background: var(--surface-hover); color: var(--text); }
  @media (hover: hover) and (pointer: fine) { .drop:hover { border-color: var(--accent-border); color: var(--text); } }
  .drop.invalid { border-color: var(--red); }
  .drop :global(svg) { color: var(--dim); }
  #drop-hint { margin: 0; max-width: 72ch; }

  /* The sheet: cells share hairlines (app.css .contact-sheet). */
  .queue { list-style: none; margin: 0; padding: 0; grid-template-columns: repeat(auto-fill, minmax(min(220px, 100%), 1fr)); }
  .item { position: relative; display: grid; grid-template-rows: auto 1fr; align-content: start; background: none; }
  .item.on { box-shadow: inset 0 0 0 1px var(--accent-border); }
  .item.bad:not(.on) { box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--red) 55%, var(--border)); }
  .item.done { background: color-mix(in srgb, var(--surface) 60%, transparent); }
  .pick { display: grid; place-items: center; width: 100%; height: 112px; padding: 0; border: 0; border-bottom: 1px solid var(--border); border-radius: 0; color: inherit; cursor: pointer;
    background-color: var(--stage-dark);
    background-image: repeating-conic-gradient(color-mix(in srgb, var(--text) 7%, transparent) 0% 25%, transparent 0% 50%); background-size: 16px 16px; }
  .pick:disabled { cursor: default; }
  .pick:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .pick img { max-width: 96px; max-height: 96px; object-fit: contain; }
  .rm { position: absolute; top: 4px; right: 4px; border-radius: var(--radius); background: color-mix(in srgb, var(--bg) 80%, transparent); }
  .info { display: grid; align-content: start; gap: 4px; min-width: 0; padding: 8px 10px 10px; }
  .nm { width: 100%; min-width: 0; min-height: 32px; margin: 0; padding: 0 0 0 0; border: 0; border-bottom: 1px solid var(--border); border-radius: 0; background: none;
    color: var(--text); font: inherit; font-weight: 600; overflow: hidden; text-overflow: ellipsis; }
  .nm:focus-visible { outline: 0; border-bottom-color: var(--accent); }
  .nm[aria-invalid='true'] { border-bottom-color: var(--red); }
  .done-nm { display: block; line-height: 32px; border-bottom-color: transparent; }
  .small-note { font-size: var(--t-small); margin: 0; line-height: 1.4; }
  .meta { margin: 0; font-size: var(--t-fine); line-height: 1.5; color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .line { display: flex; flex-wrap: wrap; align-items: center; gap: 4px 8px; min-height: 32px; }
  .st { font-size: var(--t-small); color: var(--muted); }
  .st.ok { color: var(--dim); }
  .st.bad { color: var(--red); }
  .line .small { margin-left: auto; }
  .bar { display: block; flex: 1 1 40px; min-width: 40px; height: 3px; border-radius: 2px; background: var(--border); overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--accent); transform-origin: 0 50%; transition: transform var(--fast) linear; }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .bar span { transition: none; } }
  .small { min-height: 32px; height: 32px; padding: 0 10px; }
  .err { margin: 0; font-size: var(--t-small); line-height: 1.4; outline: none; }
  .same { margin: 0; font-size: var(--t-fine); line-height: 1.45; color: var(--muted); overflow-wrap: anywhere; }
  .same a { color: var(--accent-text); font-weight: 600; text-decoration: underline; text-underline-offset: 2px; }

  /* The picked file's details: flat, under the sheet. */
  .detail { min-width: 0; padding-top: 4px; }
  .dh { display: flex; align-items: baseline; gap: 10px; margin: 0 0 12px; font-size: var(--t-label, 12px); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .dn { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: var(--font-mono); letter-spacing: 0; text-transform: none; font-weight: 500; color: var(--muted); }
  .body { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 20px 24px; }
  .fields { display: grid; align-content: start; gap: 14px; min-width: 0; }
  .fields .ui-error { margin: 0; outline: none; }
  .wide-row { flex-wrap: nowrap; gap: 16px; }
  .wide-row .ui-hint { display: block; }
  .pv { display: grid; align-content: start; gap: 8px; min-width: 0; }
  @container (max-width: 700px) { .body { grid-template-columns: minmax(0, 1fr); } }

  /* The side blocks. */
  .who { display: flex; align-items: center; gap: 10px; margin: 0; min-height: 32px; font-weight: 600; min-width: 0; }
  .who span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .as { max-width: none; }
  .quota { margin: 0; font-size: var(--t-small); color: var(--muted); }
  .quota .ui-hint { display: inline; }
  .submit .accept { align-items: flex-start; gap: 10px; min-height: 40px; font-weight: 500; font-size: var(--t-small); }
  .accept input { margin-top: 2px; }
  .accept a { color: var(--accent-text, var(--accent)); text-decoration: underline; text-underline-offset: 2px; }
  .submit { display: grid; gap: 10px; }
  .sum { margin: 4px 0 0; font-size: var(--t-small); color: var(--muted); }
  .foot { display: grid; grid-template-columns: minmax(0, 1fr); gap: 8px; }
  .foot :global(.ui-btn) { white-space: normal; height: auto; min-height: 40px; padding-block: 8px; }
  .foot > :global(*) { width: 100%; }
  /* The dialog's pinned bar stays short on phones: Upload and Close side by side, the copy button on its own row. */
  .in-dialog .foot { grid-template-columns: minmax(0, 1fr) auto; }
  .in-dialog .foot > :global(.ui-btn.secondary) { grid-column: 1 / -1; order: 3; }
  .in-dialog .foot > :global(.ui-btn.ghost) { width: auto; }
  .in-dialog .submit { gap: 6px; }
  .ui-spinner.small { width: 13px; height: 13px; border-color: color-mix(in srgb, var(--accent-ink) 35%, transparent); border-top-color: var(--accent-ink); }
  /* Phones: 16px fields, 44px targets. */
  @media (pointer: coarse), (max-width: 640px) {
    .nm { min-height: 44px; font-size: 16px; }
    .done-nm { line-height: 44px; }
    .line { min-height: 44px; }
    .small { min-height: 44px; height: 44px; }
    .pick { height: 96px; }
  }
</style>
