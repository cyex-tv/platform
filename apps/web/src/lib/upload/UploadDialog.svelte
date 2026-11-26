<script lang="ts">
  // The site-wide upload dialog (upload.svelte.ts), the only way to upload (there is no upload
  // page): a modal over whatever page you're on, in two modes (owner, 2026/11/26):
  //   Quick upload  the form (UploadForm): wide screens a 960px dialog, the form on the left, a
  //                 large drop zone with our limits on the right.
  //   Editor        the emote editor (lib/editor, loaded the first time you switch to it): the
  //                 dialog grows to fit a stage, tools, frames and a panel with the upload rules.
  //                 Its result goes back to Quick upload, which uploads it as any picked file.
  // Switching keeps both: the form's file and fields, the editor's work. Phones: a full-height
  // sheet, its title bar and buttons pinned. It can't close while the image is being sent (Esc and
  // the backdrop do nothing then); otherwise what you picked and edited stays for next time.
  // Dropping a file anywhere on the open dialog adds it to the mode on show.
  import { tick, type Component } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import UploadForm from './UploadForm.svelte';
  import { useToolButtons } from '$lib/components/btn';
  import { closeUpload, upload } from './upload.svelte';
  import type { EmoteCaps } from './upload-links';

  // The dialog is a tool surface wherever it opens: tool buttons and the tools' field labels.
  useToolButtons();
  let dialog: HTMLDialogElement;
  let form = $state<UploadForm>();
  let busy = $state(false);
  let opener: Element | null = null;
  let caps = $state<EmoteCaps | null>(null);

  type Mode = 'quick' | 'editor';
  let mode = $state<Mode>('quick');
  type EditorApi = { open(file: File): void; dirty(): boolean };
  let Editor = $state.raw<Component<{ caps?: EmoteCaps | null; oncontinue: (f: File) => void; active?: boolean }, EditorApi> | null>(null);
  let editor = $state<EditorApi>();
  let loading = $state<Promise<unknown> | null>(null);
  let loadFailed = $state(false);

  /** Loads the editor's code (once). */
  function loadEditor() {
    loadFailed = false;
    loading ??= import('$lib/editor/Editor.svelte')
      .then((m) => (Editor = m.default as unknown as typeof Editor))
      .catch(() => {
        loadFailed = true;
        loading = null;
      });
    return loading;
  }

  async function setMode(m: Mode) {
    mode = m;
    if (m === 'editor') await loadEditor();
  }

  /** Opens `file` in the editor (the form's "Open in editor"). */
  async function edit(file: File) {
    await setMode('editor');
    await tick();
    editor?.open(file);
  }

  /** The editor's finished file goes to the form, as if picked. */
  function edited(file: File) {
    mode = 'quick';
    form?.add([file]);
  }

  $effect(() => {
    if (upload.open && !dialog.open) {
      opener = document.activeElement;
      dialog.showModal();
      tick().then(() => dialog.querySelector<HTMLElement>(mode === 'quick' ? '.drop .ui-btn, .form input' : '.modes [aria-checked="true"]')?.focus());
    } else if (!upload.open && dialog.open) {
      dialog.close();
    }
  });

  function close() {
    if (busy) return;
    closeUpload();
  }

  function closed() {
    upload.open = false;
    if (opener instanceof HTMLElement && opener.isConnected) opener.focus();
    opener = null;
  }

  function cancel(e: Event) {
    e.preventDefault();
    close();
  }

  // A click on the backdrop (outside the panel) closes it, as Esc does.
  function backdrop(e: MouseEvent) {
    if (e.target === dialog) close();
  }

  function drop(e: DragEvent) {
    e.preventDefault();
    const files = e.dataTransfer?.files;
    if (mode === 'editor' && files?.[0]) editor?.open(files[0]);
    else form?.add(files);
  }

  const modes = [
    { value: 'quick' as const, label: 'Quick upload' },
    { value: 'editor' as const, label: 'Editor' },
  ];
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog class="ui-dialog upload-dialog" class:wide={mode === 'editor'} bind:this={dialog} aria-labelledby="upload-title" aria-describedby="upload-line"
  oncancel={cancel} onclose={closed} onclick={backdrop}
  ondragover={(e) => e.preventDefault()} ondrop={drop}>
  <div class="panel">
    <header class="top">
      <div class="text">
        <h2 id="upload-title">Upload an emote</h2>
        <p id="upload-line">{mode === 'editor' ? 'Crop, resize, cut and draw in your browser; nothing is sent until you upload.' : 'Public once a moderator approves it. You can use it in your own sets straight away.'}</p>
      </div>
      <div class="modes"><Segmented label="Mode" size="s" height={32} options={modes} value={mode} onchange={(m) => void setMode(m)} /></div>
      <button class="ui-icon close" type="button" aria-label="Close" onclick={close} disabled={busy}><Icon name="x" size={18} /></button>
    </header>
    <div class="scroll">
      <div class="pane" hidden={mode !== 'quick'}>
        <UploadForm bind:this={form} bind:busy bind:caps onclose={close} onedit={edit} />
      </div>
      {#if mode === 'editor' || Editor}
        <div class="pane" hidden={mode !== 'editor'}>
          {#if Editor}
            <Editor bind:this={editor} {caps} oncontinue={edited} active={mode === 'editor' && upload.open} />
          {:else if loadFailed}
            <div class="wait">
              <p class="ui-error" role="alert">Couldn’t load the editor.</p>
              <div class="ui-actions">
                <button type="button" class="ui-btn secondary" onclick={() => void loadEditor()}>Try again</button>
                <button type="button" class="ui-btn ghost" onclick={() => (mode = 'quick')}>Back to quick upload</button>
              </div>
            </div>
          {:else}
            <div class="wait" role="status"><span class="ui-spinner" aria-hidden="true"></span><span class="ui-hint">Loading the editor…</span></div>
          {/if}
        </div>
      {/if}
    </div>
  </div>
</dialog>

<style>
  /* clip, not hidden: focus and scrollIntoView can't shift the panel inside the dialog. */
  .upload-dialog { width: min(960px, calc(100vw - 32px)); height: min(680px, calc(100dvh - 48px)); max-height: calc(100dvh - 48px); overflow: clip; padding: 0;
    transition-property: opacity, scale, overlay, display, width, height; }
  /* The editor: as large as the window allows, up to 1600 × 1000. */
  .upload-dialog.wide { width: min(1600px, calc(100vw - 32px)); height: min(1000px, calc(100dvh - 32px)); max-height: calc(100dvh - 32px); }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .upload-dialog { transition-property: opacity, overlay, display; } }
  .panel { display: grid; grid-template-rows: auto minmax(0, 1fr); height: 100%; }
  .top { display: flex; align-items: center; gap: 12px; padding: 12px 16px 10px; border-bottom: 1px solid var(--border); }
  .text { flex: 1; min-width: 0; }
  h2 { margin: 0; font-size: 18px; font-weight: 600; letter-spacing: -.01em; line-height: 1.35; }
  .top p { margin: 2px 0 0; font-size: var(--t-small); color: var(--muted); }
  .modes { flex: none; }
  .close { flex: none; margin: -4px -6px 0 0; }
  /* The form and the editor scroll their own columns. */
  /* clip: focus or scrollIntoView inside can't scroll the whole mode out of place. */
  .scroll { min-height: 0; overflow: clip; }
  .pane { height: 100%; min-height: 0; overflow: clip; }
  .pane[hidden] { display: none; }
  .wait { display: grid; place-content: center; justify-items: center; gap: 12px; height: 100%; }
  /* Narrower: the line under the title goes, the mode switch stays beside it. */
  @media (max-width: 719.98px) {
    .top p { display: none; }
    .top { gap: 8px; }
  }
  /* Phones: a full-height sheet; the title bar stays put while the form scrolls. */
  @media (max-width: 639.98px) {
    .upload-dialog, .upload-dialog.wide { width: 100vw; max-width: 100vw; height: 100dvh; max-height: 100dvh; margin: 0; border: 0; border-radius: 0;
      /* Full screen: it fades in place, with no scale (it would show the page edge behind it). */
      transition-property: opacity, overlay, display; scale: 1; }
    .top { padding-top: calc(12px + env(safe-area-inset-top, 0px)); }
    h2 { font-size: 16px; }
  }
  @media (max-width: 479.98px) { h2 { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); } }
</style>
