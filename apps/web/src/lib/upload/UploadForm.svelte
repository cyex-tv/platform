<script lang="ts">
  // The upload form inside the site-wide upload dialog (UploadDialog), one image at a time
  // (owner, 2026/11/22: the upload is a dialog again, /upload and /me/upload are gone).
  //   left   the form: Uploading as (a small select, only when you may upload for an account you
  //          edit, ADR 0014; `upload.for` picks one), Name (from the file name until typed), Tags
  //          (TagInput chips), the two flags a creator sets (Overlaying = zero-width, Private =
  //          only your own channel; everything else is public, unlisted is for moderators and
  //          NSFW emotes aren't allowed, ADR 0020), how many of yours wait for review, "I accept
  //          the rules and guidelines" (the link opens /guidelines in a new tab), Discard and
  //          Upload.
  //   right  a large drop zone with Browse files; once an image is in, its preview at the sizes
  //          cyex.tv makes (1x to 4x, at most 128 px tall), and our limits under it.
  // Files are checked in the browser first (type, 8 MB, 1000 × 1000 px). The upload shows its
  // progress and can be cancelled; one that sends nothing for 30 s stalls (UPLOAD_STALL_MS). A
  // refusal is said in words and the image stays to fix or retry. A file whose bytes are already a
  // public emote says "Looks like <emote>" (same-file.ts); it can still be uploaded. When it goes
  // through, the dialog closes and the new emote's page opens.
  import { goto } from '$app/navigation';
  import { ApiError, gql, uploadEmote, type EmoteVisibility } from '$lib/api';
  import { UPLOAD_QUOTA } from '$lib/queries';
  import { quotaView, type Quota } from './quota';
  import Icon from '$lib/components/Icon.svelte';
  import TagInput from '$lib/components/TagInput.svelte';
  import RetryNote from '$lib/components/RetryNote.svelte';
  import { bytes, clean, iso } from '$lib/format';
  import { quiet } from '$lib/quiet';
  import { editorOf, type Grant } from '$lib/profile/data.svelte';
  import { emoteCan, uploadTargets } from '$lib/tool/editors';
  import { session } from '$lib/tool/session.svelte';
  import { TAG_MAX, UPLOAD_ACCEPT, cleanTag, fileProblem, nameFromFile, nameProblem, tagsProblem, uploadProblem } from '$lib/rules';
  import EmoteFlags from '$lib/tool/EmoteFlags.svelte';
  import LocalPreview from '$lib/tool/LocalPreview.svelte';
  import LeaveGuard from '$lib/tool/LeaveGuard.svelte';
  import { notify } from '$lib/tool/notices.svelte';
  import { sameFileEmotes, sameFileLine, type SameFileEmote } from './same-file';
  import { DEFAULT_EMOTE_CAPS, upload, uploadLimits } from './upload.svelte';

  type Props = {
    /** Sending right now (the dialog won't close while it is). */
    busy?: boolean;
    onclose?: () => void;
  };
  let { busy = $bindable(false), onclose }: Props = $props();
  const uid = $props.id();

  type Picked = { file: File; url: string; width: number; height: number; sizeError: string | null; same: SameFileEmote[] };
  let picked = $state<Picked | null>(null);
  let name = $state('');
  let prefilled = $state('');
  let tags = $state<string[]>([]);
  let zeroWidth = $state(false);
  let visibility = $state<EmoteVisibility>('PUBLIC');
  let accepted = $state(false);
  let tried = $state(false);
  let over = $state(false);
  let note = $state<string | null>(null);
  let error = $state<string | null>(null);
  let progress = $state(0);
  let input: HTMLInputElement;
  let browse = $state<HTMLButtonElement>();
  let nameInput = $state<HTMLInputElement>();
  let acceptBox = $state<HTMLInputElement>();
  let running: AbortController | null = null;

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
  let as = $state<string | null>(null);
  // Your own account unless you picked another, or the dialog was opened for one you may upload for.
  const owner = $derived(targets.find((t) => t.id === (as ?? upload.for)) ?? targets[0] ?? null);
  const forOther = $derived(!!owner && !owner.self);
  /** Where an upload opens: its edit page when you may edit it, else the account it went to. */
  const openHref = (id: string) =>
    !forOther || emoteCan(grants, owner?.id, session.me?.id, 'EMOTE_MANAGE') ? `/me/emotes/${id}` : `/users/${encodeURIComponent(owner!.id)}`;

  // Where the account picked stands with emotes waiting for review; null until read.
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
  // The caps for this account, as the API says them (`uploadQuota`); the built-in numbers until then.
  const caps = $derived(quota ? { maxBytes: quota.maxBytes, maxSide: quota.maxSide, maxFrames: quota.maxFrames } : DEFAULT_EMOTE_CAPS);

  const fileError = $derived(picked ? (fileProblem(picked.file, caps.maxBytes) ?? picked.sizeError) : null);
  const nameError = $derived(tried || name !== prefilled ? nameProblem(name) : null);
  const tagError = $derived(tagsProblem(tags));
  const dirty = $derived(!!picked || !!name.trim() || tags.length > 0);

  $effect(() => () => {
    if (picked) URL.revokeObjectURL(picked.url);
  });

  /** Reads the picture's size, so a too-big or unreadable file is named before the upload. */
  async function measure(p: Picked) {
    if (fileProblem(p.file, caps.maxBytes)) return;
    try {
      const bmp = await createImageBitmap(p.file);
      if (picked?.url === p.url) {
        picked.width = bmp.width;
        picked.height = bmp.height;
        if (bmp.width > caps.maxSide || bmp.height > caps.maxSide) picked.sizeError = `It’s ${bmp.width} × ${bmp.height} px; at most ${caps.maxSide} × ${caps.maxSide}.`;
      }
      bmp.close();
    } catch {
      if (picked?.url === p.url) picked.sizeError = 'That file doesn’t open as an image.';
    }
  }

  /** Asks whether the file is already a public emote; quietly. */
  async function checkSame(p: Picked) {
    if (!session.me || fileProblem(p.file, caps.maxBytes)) return;
    const found = await sameFileEmotes(p.file);
    if (picked?.url === p.url && found.length) picked.same = found;
  }

  /** Takes the image to upload (the first one, when several are dropped). */
  export function add(list: FileList | File[] | null | undefined) {
    if (!list?.length || busy) return;
    const file = list[0];
    note = list.length > 1 ? `One image at a time: ${file.name} is in, the other ${list.length === 2 ? 'one was' : `${list.length - 1} were`} left out.` : null;
    if (picked) URL.revokeObjectURL(picked.url);
    const next: Picked = { file, url: URL.createObjectURL(file), width: 0, height: 0, sizeError: null, same: [] };
    picked = next;
    error = null;
    progress = 0;
    // The name follows the file until you type your own.
    if (!name.trim() || name === prefilled) {
      name = nameFromFile(file.name);
      prefilled = name;
    }
    void measure(next);
    void checkSame(next);
  }

  /** Empties the form: the image, name, tags and flags. Nothing to discard closes the dialog. */
  export function discard() {
    if (busy) return;
    if (!dirty) {
      onclose?.();
      return;
    }
    if (picked) URL.revokeObjectURL(picked.url);
    picked = null;
    name = '';
    prefilled = '';
    tags = [];
    zeroWidth = false;
    visibility = 'PUBLIC';
    tried = false;
    note = null;
    error = null;
    progress = 0;
    queueMicrotask(() => browse?.focus());
  }

  function drop(e: DragEvent) {
    e.preventDefault();
    // The dialog around the form takes drops too: one drop is one add, not two.
    e.stopPropagation();
    over = false;
    add(e.dataTransfer?.files);
  }

  function cancel() {
    running?.abort();
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (busy) return;
    tried = true;
    error = null;
    if (!picked) {
      browse?.focus();
      return;
    }
    if (fileError) return;
    if (nameProblem(name) || tagError) {
      nameInput?.focus();
      return;
    }
    if (!accepted) {
      acceptBox?.focus();
      return;
    }
    if (room?.full) return;
    busy = true;
    progress = 0;
    running = new AbortController();
    try {
      const made = await uploadEmote(picked.file, name.trim(), tags, { zeroWidth, visibility }, (p) => (progress = p), forOther ? owner!.id : null, {
        signal: running.signal,
      });
      notify(`${iso(made.name)} uploaded${forOther ? ` for ${iso(clean(owner!.displayName))}` : ''}. It’s pending until a moderator looks at it.`);
      const href = openHref(made.id);
      busy = false;
      accepted = false;
      discard();
      onclose?.();
      await goto(href);
    } catch (err) {
      if (!(err instanceof ApiError && err.code === 'aborted')) {
        error = err instanceof ApiError ? uploadProblem(err.code, err.message) : 'The upload didn’t go through.';
        if (err instanceof ApiError && err.code === 'too_many_pending') loadQuota();
      }
    } finally {
      busy = false;
      running = null;
    }
  }
</script>

<!-- Closing or reloading the tab loses the image; the dialog keeps it while you browse. -->
<LeaveGuard dirty={busy || !!picked} inApp={false} what={busy ? 'the upload in progress' : 'the image waiting to upload'} />

<form class="up" id="upload-{uid}" onsubmit={submit} novalidate>
    <div class="fields">
    {#if targets.length > 1}
      <div class="ui-field as">
        <label class="ui-label" for="upload-as-{uid}">Uploading as</label>
        <select id="upload-as-{uid}" class="ui-input" value={owner?.id} onchange={(e) => (as = e.currentTarget.value)} disabled={busy} aria-describedby="as-hint-{uid}">
          {#each targets as t (t.id)}<option value={t.id}>{t.self ? `${clean(t.displayName)} (you)` : clean(t.displayName)}</option>{/each}
        </select>
        {#if forOther}<span class="ui-hint" id="as-hint-{uid}">{clean(owner!.displayName)} owns it; it counts toward both your hourly limits.</span>
        {:else}<span class="sr-only" id="as-hint-{uid}">You own what you upload.</span>{/if}
      </div>
    {/if}
    {#if grantsFailed}
      <RetryNote onretry={loadGrants}>Couldn’t load the accounts you can upload for. You can still upload as yourself.</RetryNote>
    {/if}

    <div class="ui-field">
      <label class="ui-label" for="upload-name-{uid}">Name</label>
      <input bind:this={nameInput} id="upload-name-{uid}" class="ui-input" bind:value={name} maxlength="100" autocomplete="off" spellcheck="false" placeholder="catJAM"
        disabled={busy} aria-invalid={nameError ? 'true' : undefined} aria-describedby="name-hint-{uid}" />
      {#if nameError}<span class="ui-error" id="name-hint-{uid}">{nameError}</span>
      {:else}<span class="ui-hint" id="name-hint-{uid}">What people type in chat, no spaces.</span>{/if}
    </div>

    <div class="ui-field">
      <TagInput label="Tags" showLabel {tags} onchange={(t) => (tags = t)} max={TAG_MAX} clean={cleanTag} placeholder="cat, dance"
        hint="Enter or a comma adds a tag, up to {TAG_MAX}. They help people find it." disabled={busy} id="upload-tags-{uid}" />
      {#if tagError}<span class="ui-error">{tagError}</span>{/if}
    </div>

    <EmoteFlags id="up-{uid}" bind:zeroWidth bind:visibility disabled={busy} />

    {#if room}
      <p class="quota" class:full={room.full} role="status" id="quota-{uid}">
        {#if room.full}<span class="ui-error">{room.refusal}</span>{:else}<span class="ui-mono">{room.line}</span>{/if}
      </p>
    {/if}
    </div>

    <div class="end">
      <label class="accept">
        <input type="checkbox" bind:this={acceptBox} bind:checked={accepted} disabled={busy} data-testid="accept-rules"
          aria-invalid={tried && !accepted && picked ? 'true' : undefined} aria-describedby={tried && !accepted && picked ? `accept-note-${uid}` : undefined} />
        <span>I accept the <a href="/guidelines" target="_blank" rel="noopener">rules and guidelines</a><span class="sr-only"> (opens in a new tab)</span></span>
      </label>
      {#if tried && !accepted && picked}<p class="ui-error note" id="accept-note-{uid}">Tick the box to upload.</p>{/if}
      {#if error}<p class="ui-error note" role="alert">{error}</p>{/if}
      <div class="actions">
        {#if busy}
          <button type="button" class="ui-btn ghost" onclick={cancel}>Cancel</button>
        {:else}
          <button type="button" class="ui-btn ghost" onclick={discard}>Discard</button>
        {/if}
        <button class="ui-btn primary" type="submit" disabled={busy || !!room?.full} aria-describedby={room ? `quota-${uid}` : undefined}>
          {#if busy}<span class="ui-spinner small" aria-hidden="true"></span> Uploading…{:else}Upload{/if}
        </button>
      </div>
    </div>

  <div class="side">
    <div
      class="drop"
      class:over
      class:has={!!picked}
      class:invalid={(tried && !picked) || !!fileError}
      role="presentation"
      ondragover={(e) => { e.preventDefault(); over = true; }}
      ondragleave={() => (over = false)}
      ondrop={drop}
    >
      {#if picked}
        <div class="pv">
          {#if picked.width && !fileError}
            <LocalPreview src={picked.url} width={picked.width} height={picked.height} name={name.trim() || picked.file.name} />
          {:else if fileError}
            <p class="ui-error" role="alert">{fileError} Pick another image.</p>
          {:else}
            <p class="ui-hint">Reading the image…</p>
          {/if}
        </div>
        <p class="meta ui-mono" title={picked.file.name}><bdi>{picked.file.name}</bdi> · {bytes(picked.file.size)}{#if picked.width}{' · '}{picked.width} × {picked.height}{/if}</p>
        {#if busy}
          <span class="bar" role="progressbar" aria-label="Uploading {name}" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(progress * 100)}><span style:transform="scaleX({Math.max(0.04, progress)})"></span></span>
        {:else}
          <button type="button" class="ui-btn secondary small" bind:this={browse} onclick={() => input.click()}>Replace image</button>
        {/if}
        {#if picked.same.length}
          {@const line = sameFileLine(picked.same)}
          <p class="same" role="status">
            Looks like <a href="/emotes/{encodeURIComponent(picked.same[0].id)}" target="_blank" rel="noopener"><bdi>{clean(picked.same[0].name)}</bdi><span class="sr-only"> (opens in a new tab)</span></a>{#if line.by}{' '}<bdi>{clean(line.by)}</bdi>{/if}{line.more}, already on cyex.tv. You can add that one to your sets, or upload this anyway.
          </p>
        {/if}
      {:else}
        <Icon name="upload" size={28} />
        <p class="lead">Drop an image here</p>
        <button type="button" class="ui-btn secondary" bind:this={browse} onclick={() => input.click()} aria-describedby="limits-{uid}">Browse files</button>
        {#if tried}<p class="ui-error">Pick an image to upload.</p>{/if}
      {/if}
      {#if note}<p class="ui-hint">{note}</p>{/if}
    </div>
    <input bind:this={input} class="sr-only" type="file" tabindex="-1" accept={UPLOAD_ACCEPT}
      onchange={(e) => { add(e.currentTarget.files); e.currentTarget.value = ''; }} />
    <ul class="limits" id="limits-{uid}" aria-label="Limits">
      {#each uploadLimits(quota ? caps : null) as l (l)}<li>{l}</li>{/each}
    </ul>
  </div>
</form>

<style>
  /* Two columns: the form left, the drop zone right (its limits under it). From 760px down one column,
     the drop zone first and the actions pinned to the bottom of the sheet. */
  .up { display: grid; grid-template-columns: minmax(0, 340px) minmax(0, 1fr); height: 100%; min-height: 0; }
  /* The fields scroll; the rules box and the buttons stay at the bottom of the left column. */
  .up { grid-template-rows: minmax(0, 1fr) auto; }
  .fields { grid-column: 1; grid-row: 1; display: flex; flex-direction: column; gap: 16px; min-width: 0; min-height: 0; padding: 20px 20px 12px;
    overflow: hidden auto; overscroll-behavior: contain; border-right: 1px solid var(--border); }
  .end { grid-column: 1; grid-row: 2; border-right: 1px solid var(--border); }
  .side { grid-column: 2; grid-row: 1 / span 2; display: grid; grid-template-rows: minmax(0, 1fr) auto; gap: 12px; min-width: 0; min-height: 0; padding: 20px; overflow: hidden auto; }
  .as .ui-input { max-width: none; }
  .quota { margin: 0; font-size: var(--t-small); color: var(--muted); }
  .end { display: grid; gap: 8px; padding: 12px 20px 16px; border-top: 1px solid var(--border); background: var(--elevated); }
  .accept { display: flex; align-items: flex-start; gap: 10px; min-height: 40px; font-size: var(--t-small); font-weight: 500; color: var(--text); cursor: pointer; }
  .accept input { flex: none; width: 18px; height: 18px; margin: 1px 0 0; accent-color: var(--accent); }
  .accept a { color: var(--accent-text, var(--accent)); text-decoration: underline; text-underline-offset: 2px; }
  .note { margin: 0; }
  .actions { display: grid; grid-template-columns: auto minmax(0, 1fr); gap: 8px; }
  .actions .ui-btn { min-height: 40px; }

  /* The drop zone: a dashed hairline area, no card; it fills the column. */
  .drop { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; min-height: 320px; padding: 24px;
    border: 1px dashed var(--border-strong); border-radius: var(--radius-card); color: var(--muted); text-align: center;
    transition: border-color var(--fast), background-color var(--fast), color var(--fast); }
  .drop :global(svg) { color: var(--dim); }
  .drop.over { border-color: var(--accent-border); background: var(--surface-hover); color: var(--text); }
  .drop.invalid { border-color: var(--red); }
  .drop.has { justify-content: center; }
  .lead { margin: 0; font-size: var(--t-ui, 14px); font-weight: 600; color: var(--text); }
  .drop p { margin: 0; }
  .pv { display: grid; place-items: center; max-width: 100%; min-height: 160px; }
  .pv :global(.sizes) { justify-content: center; }
  .meta { max-width: 100%; font-size: var(--t-fine); color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .same { max-width: 52ch; font-size: var(--t-fine); line-height: 1.45; color: var(--muted); overflow-wrap: anywhere; }
  .same a { color: var(--accent-text); font-weight: 600; text-decoration: underline; text-underline-offset: 2px; }
  .bar { display: block; width: min(240px, 100%); height: 3px; border-radius: var(--radius-sm); background: var(--border); overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--accent); transform-origin: 0 50%; transition: transform var(--fast) linear; }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .bar span { transition: none; } }
  .small { min-height: 32px; height: 32px; padding: 0 12px; }
  .limits { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; justify-content: center; gap: 4px 16px;
    font-size: var(--t-fine); color: var(--muted); font-variant-numeric: tabular-nums; }
  .limits li { white-space: nowrap; }
  .ui-spinner.small { width: 13px; height: 13px; border-color: color-mix(in srgb, var(--accent-ink) 35%, transparent); border-top-color: var(--accent-ink); }

  @media (max-width: 719.98px) {
    .up { display: flex; flex-direction: column; overflow: hidden auto; overscroll-behavior: contain; }
    .side { order: 1; flex: none; grid-template-rows: auto auto; overflow: visible; padding-bottom: 8px; }
    .fields { order: 2; flex: none; overflow: visible; padding-top: 4px; border-right: 0; }
    .end { order: 3; position: sticky; bottom: 0; z-index: 2; margin-top: auto; border-right: 0; padding-bottom: calc(12px + env(safe-area-inset-bottom, 0px)); }
    .drop { min-height: 200px; }
  }
  /* Phones: 16px fields, 44px targets. */
  @media (pointer: coarse), (max-width: 639.98px) {
    .small, .actions .ui-btn { min-height: 44px; height: 44px; }
    .accept { min-height: 44px; }
  }
  @media (hover: hover) and (pointer: fine) { .drop:not(.has):hover { border-color: var(--accent-border); } }
</style>
