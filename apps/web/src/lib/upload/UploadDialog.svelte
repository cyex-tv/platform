<script lang="ts">
  // The site-wide upload dialog (upload.svelte.ts): the same UploadForm as /me/upload, in a modal
  // that opens over whatever page you're on. Wide screens: a 960px dialog that scrolls inside.
  // Phones: the whole screen, its title bar and buttons pinned. It can't close while files are
  // being sent (Esc and the backdrop do nothing then); otherwise the queue stays for next time,
  // and only finished uploads leave it. Dropping images anywhere on the open dialog adds them.
  import { tick } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { UPLOAD_FORMATS } from '$lib/rules';
  import UploadForm from './UploadForm.svelte';
  import { closeUpload, upload } from './upload.svelte';

  let dialog: HTMLDialogElement;
  let form = $state<UploadForm>();
  let busy = $state(false);
  let opener: Element | null = null;

  $effect(() => {
    if (upload.open && !dialog.open) {
      opener = document.activeElement;
      dialog.showModal();
      tick().then(() => dialog.querySelector<HTMLElement>('.drop')?.focus());
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
    form?.clearDone();
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
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog class="ui-dialog upload-dialog" bind:this={dialog} aria-labelledby="upload-title" aria-describedby="upload-line"
  oncancel={cancel} onclose={closed} onclick={backdrop}
  ondragover={(e) => e.preventDefault()} ondrop={(e) => { e.preventDefault(); form?.add(e.dataTransfer?.files); }}>
  <div class="panel">
    <header class="top">
      <div class="text">
        <h2 id="upload-title">Upload</h2>
        <p id="upload-line">{UPLOAD_FORMATS}, at most 8 MB and 1000 × 1000 px. Up to 20 at once.</p>
      </div>
      <button class="ui-icon close" type="button" aria-label="Close" onclick={close} disabled={busy}><Icon name="x" size={18} /></button>
    </header>
    <div class="scroll">
      <UploadForm bind:this={form} bind:busy inDialog onclose={close} />
    </div>
  </div>
</dialog>

<style>
  /* clip, not hidden: focus and scrollIntoView can't shift the panel inside the dialog. */
  .upload-dialog { width: min(960px, calc(100vw - 32px)); max-height: calc(100dvh - 48px); overflow: clip; }
  .panel { display: grid; grid-template-rows: auto minmax(0, 1fr); max-height: calc(100dvh - 48px); }
  .top { display: flex; align-items: flex-start; gap: 12px; padding: 14px 16px 12px; border-bottom: 1px solid var(--border); }
  .text { flex: 1; min-width: 0; }
  h2 { margin: 0; font-size: 1.05rem; font-weight: 700; letter-spacing: 0; }
  .top p { margin: 2px 0 0; font-size: var(--t-small); color: var(--muted); }
  .close { flex: none; margin: -4px -6px 0 0; }
  /* Vertical only: a focus ring or a 2px rounding at the edge never brings up a sideways bar. */
  .scroll { overflow: hidden auto; overscroll-behavior: contain; padding: 16px; }
  /* Phones: the whole screen; the title bar stays put while the form scrolls. */
  @media (max-width: 640px) {
    .upload-dialog { width: 100vw; max-width: 100vw; height: 100dvh; max-height: 100dvh; margin: 0; border: 0; border-radius: 0; }
    .panel { height: 100dvh; max-height: 100dvh; }
    .top { padding-top: calc(12px + env(safe-area-inset-top, 0px)); }
    .scroll { padding: 16px 16px calc(16px + env(safe-area-inset-bottom, 0px)); }
  }
</style>
