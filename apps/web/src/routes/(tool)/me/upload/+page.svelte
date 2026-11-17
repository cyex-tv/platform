<script lang="ts">
  import { goto } from '$app/navigation';
  import { ApiError, uploadEmote } from '$lib/api';
  import { bytes, iso } from '$lib/format';
  import { fileProblem, nameFromFile, nameProblem, parseTags, tagsProblem } from '$lib/rules';
  import { onDestroy } from 'svelte';
  import { notify } from '$lib/tool/notices.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';

  let file = $state<File | null>(null);
  let preview = $state<string | null>(null);
  let name = $state('');
  let prefilled = '';
  let tagText = $state('');
  let busy = $state(false);
  let over = $state(false);
  let tried = $state(false);
  let serverError = $state<string | null>(null);
  let input: HTMLInputElement;
  let nameInput: HTMLInputElement;
  let tagInput: HTMLInputElement;
  let dropButton: HTMLButtonElement;
  let dimError = $state<string | null>(null);
  let picked = 0;

  const fileError = $derived(file ? (fileProblem(file) ?? dimError) : tried ? 'Pick an image.' : null);
  const nameError = $derived(tried || name ? nameProblem(name) : null);
  const tags = $derived(parseTags(tagText));
  const tagError = $derived(tagsProblem(tags));

  onDestroy(() => {
    if (preview) URL.revokeObjectURL(preview);
  });

  /** Reads the image's size, so a too-big or unreadable file is named before the upload. */
  async function checkSize(f: File) {
    const mine = ++picked;
    dimError = null;
    if (fileProblem(f)) return;
    try {
      const bmp = await createImageBitmap(f);
      const { width, height } = bmp;
      bmp.close();
      if (mine === picked && (width > 1000 || height > 1000)) dimError = 'At most 1000 × 1000 px.';
    } catch {
      if (mine === picked) dimError = 'That file isn’t an image.';
    }
  }

  function pick(f: File | undefined | null) {
    if (!f) return;
    if (preview) URL.revokeObjectURL(preview);
    checkSize(f);
    file = f;
    preview = URL.createObjectURL(f);
    serverError = null;
    // The name follows the file until it is typed in by hand.
    if (!name || name === prefilled) name = prefilled = nameFromFile(f.name);
  }

  function drop(e: DragEvent) {
    e.preventDefault();
    over = false;
    pick(e.dataTransfer?.files[0]);
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (busy) return;
    tried = true;
    if (!file || fileError || nameError || tagError) {
      (fileError || !file ? dropButton : nameError ? nameInput : tagInput)?.focus();
      return;
    }
    busy = true;
    serverError = null;
    try {
      const made = await uploadEmote(file, name.trim(), tags);
      notify(`${iso(made.name)} uploaded. It’s pending until a moderator looks at it.`);
      await goto(`/me/emotes/${made.id}`);
    } catch (err) {
      serverError = err instanceof ApiError ? err.message : 'The upload didn’t go through.';
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head><title>Upload - cyex.tv</title></svelte:head>

<div class="narrow">
  <PageHead title="Upload" line="PNG, APNG, GIF or WebP, at most 5 MB and 1000×1000 px. Square or wide works best." />

  <form class="ui-card ui-form" onsubmit={submit} novalidate>
    <div class="ui-field">
      <span class="ui-label" id="file-label">Image</span>
      <button
        type="button"
        class="drop"
        class:over
        class:has={file}
        class:invalid={!!fileError}
        bind:this={dropButton}
        aria-describedby={fileError ? 'file-hint' : undefined}
        onclick={() => input.click()}
        ondragover={(e) => { e.preventDefault(); over = true; }}
        ondragleave={() => (over = false)}
        ondrop={drop}
      >
        {#if preview && file}
          <span class="sizes">
            {#each [28, 56, 112] as h}<img src={preview} alt="" style:max-height="{h}px" />{/each}
          </span>
          <span class="ui-hint"><span class="fname"><bdi>{file.name}</bdi></span> · <span class="ui-num">{bytes(file.size)}</span> · Pick another</span>
        {:else}
          <svg viewBox="0 0 24 24" width="28" height="28" aria-hidden="true"><path d="M12 16V4M7 9l5-5 5 5M5 20h14" /></svg>
          <span>Drop an image here, or pick one</span>
        {/if}
      </button>
      <input bind:this={input} class="sr-only" type="file" tabindex="-1" accept="image/png,image/apng,image/gif,image/webp"
        onchange={(e) => pick(e.currentTarget.files?.[0])} />
      {#if fileError}<p class="ui-error" id="file-hint">{fileError}</p>{/if}
    </div>

    <label class="ui-field">
      <span class="ui-label">Name</span>
      <input class="ui-input" bind:this={nameInput} bind:value={name} maxlength="100" autocomplete="off" spellcheck="false"
        aria-invalid={nameError ? 'true' : undefined} aria-describedby={nameError ? 'name-msg' : 'name-hint'} placeholder="catJAM" />
      {#if nameError}<span class="ui-error" id="name-msg">{nameError}</span>{:else}<span class="ui-hint" id="name-hint">What people type in chat. No spaces.</span>{/if}
    </label>

    <label class="ui-field">
      <span class="ui-label">Tags</span>
      <input class="ui-input" bind:this={tagInput} bind:value={tagText} autocomplete="off" aria-invalid={tagError ? 'true' : undefined} aria-describedby={tagError ? 'tag-msg' : 'tag-hint'} placeholder="cat, dance" />
      {#if tagError}<span class="ui-error" id="tag-msg">{tagError}</span>{:else}<span class="ui-hint" id="tag-hint">Up to 10, separated by commas. They help people find it.</span>{/if}
    </label>

    {#if serverError}<p class="ui-error" role="alert">{serverError}</p>{/if}

    <div class="ui-actions foot">
      <button class="ui-btn primary" type="submit" disabled={busy}>
        {#if busy}<span class="ui-spinner small" aria-hidden="true"></span> Uploading…{:else}Upload{/if}
      </button>
      <a class="ui-btn ghost" href="/me">Cancel</a>
    </div>
    <p class="ui-hint">
      New emotes are pending until a moderator approves them. You can use them in your own sets straight away. Only upload
      what you made or may use.
    </p>
  </form>
</div>

<style>
  .drop { display: grid; border-radius: var(--radius-card); justify-items: center; align-content: center; gap: 10px; min-height: 168px; padding: 20px;
    border: 1px dashed var(--border-strong); background: var(--surface); color: var(--muted); font: inherit; cursor: pointer;
    transition: border-color var(--fast), background-color var(--fast); }
  .drop.over { border-color: var(--accent-border); background: var(--surface-hover); }
  @media (hover: hover) and (pointer: fine) { .drop:hover { border-color: var(--accent-border); background: var(--surface-hover); } }
  .drop.invalid { border-color: var(--red); }
  .drop svg { fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
  .sizes { display: flex; align-items: flex-end; gap: 20px; }
  .sizes img { max-width: 220px; height: auto; }
  .fname { display: inline-block; max-width: 28ch; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; vertical-align: bottom; }
  .foot { padding-top: 4px; }
  .small { width: 13px; height: 13px; border-color: color-mix(in srgb, var(--accent-ink) 35%, transparent); border-top-color: var(--accent-ink); }
</style>
