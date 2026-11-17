<script lang="ts">
  import { goto } from '$app/navigation';
  import { ApiError, uploadEmote } from '$lib/api';
  import { bytes } from '$lib/format';
  import { fileProblem, nameProblem, parseTags, tagsProblem } from '$lib/rules';
  import { notify } from '$lib/tool/notices.svelte';

  let file = $state<File | null>(null);
  let preview = $state<string | null>(null);
  let name = $state('');
  let tagText = $state('');
  let busy = $state(false);
  let over = $state(false);
  let tried = $state(false);
  let serverError = $state<string | null>(null);
  let input: HTMLInputElement;

  const fileError = $derived(file ? fileProblem(file) : tried ? 'Pick an image.' : null);
  const nameError = $derived(tried || name ? nameProblem(name) : null);
  const tags = $derived(parseTags(tagText));
  const tagError = $derived(tagsProblem(tags));

  function pick(f: File | undefined | null) {
    if (!f) return;
    if (preview) URL.revokeObjectURL(preview);
    file = f;
    preview = URL.createObjectURL(f);
    serverError = null;
    if (!name) name = f.name.replace(/\.[^.]+$/, '').replace(/\s+/g, '').slice(0, 100);
  }

  function drop(e: DragEvent) {
    e.preventDefault();
    over = false;
    pick(e.dataTransfer?.files[0]);
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    tried = true;
    if (!file || fileError || nameError || tagError) return;
    busy = true;
    serverError = null;
    try {
      const made = await uploadEmote(file, name.trim(), tags);
      notify(`${made.name} uploaded. It’s pending until a moderator looks at it.`);
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
  <div class="ui-head">
    <div>
      <h1>Upload an emote</h1>
      <p>PNG, APNG, GIF or WebP, at most 5 MB and 1000×1000 px. Square or wide works best.</p>
    </div>
  </div>

  <form class="ui-form" onsubmit={submit} novalidate>
    <div class="ui-field">
      <span class="ui-label" id="file-label">Image</span>
      <button
        type="button"
        class="drop"
        class:over
        class:has={file}
        class:invalid={!!fileError}
        aria-labelledby="file-label"
        aria-describedby="file-hint"
        onclick={() => input.click()}
        ondragover={(e) => { e.preventDefault(); over = true; }}
        ondragleave={() => (over = false)}
        ondrop={drop}
      >
        {#if preview && file}
          <span class="sizes">
            {#each [28, 56, 112] as h}<img src={preview} alt="" style:max-height="{h}px" />{/each}
          </span>
          <span class="ui-hint"><span class="fname">{file.name}</span> · <span class="ui-mono">{bytes(file.size)}</span> · Pick another</span>
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
      <input class="ui-input" bind:value={name} maxlength="100" autocomplete="off" spellcheck="false"
        aria-invalid={nameError ? 'true' : undefined} placeholder="catJAM" />
      {#if nameError}<span class="ui-error">{nameError}</span>{:else}<span class="ui-hint">What people type in chat. No spaces.</span>{/if}
    </label>

    <label class="ui-field">
      <span class="ui-label">Tags</span>
      <input class="ui-input" bind:value={tagText} autocomplete="off" aria-invalid={tagError ? 'true' : undefined} placeholder="cat, dance" />
      {#if tagError}<span class="ui-error">{tagError}</span>{:else}<span class="ui-hint">Up to 10, separated by commas. They help people find it.</span>{/if}
    </label>

    {#if serverError}<p class="ui-error" role="alert">{serverError}</p>{/if}

    <div class="ui-actions">
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
  .drop { display: grid; justify-items: center; align-content: center; gap: 10px; min-height: 168px; padding: 20px;
    border: 1px dashed var(--border-strong); background: var(--surface); color: var(--muted); font: inherit; cursor: pointer;
    transition: border-color var(--fast), background-color var(--fast); }
  .drop:hover, .drop.over { border-color: var(--accent-border); background: var(--surface-hover); }
  .drop.invalid { border-color: var(--red); }
  .drop svg { fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
  .sizes { display: flex; align-items: flex-end; gap: 20px; }
  .sizes img { max-width: 220px; height: auto; }
  .fname { display: inline-block; max-width: 28ch; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; vertical-align: bottom; }
  .small { width: 13px; height: 13px; border-color: color-mix(in srgb, var(--accent-ink) 35%, transparent); border-top-color: var(--accent-ink); }
</style>
