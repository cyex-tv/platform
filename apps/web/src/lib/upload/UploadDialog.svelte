<script lang="ts">
  // The site-wide upload dialog (upload.svelte.ts), the only way to upload (there is no upload
  // page): a modal over whatever page you're on. Wide screens: a 960px dialog, the form on the
  // left (name, tags, the two flags, the rules box, Discard and Upload), a large drop zone with
  // our limits on the right. Phones: a full-height sheet, its title bar and buttons pinned. It
  // can't close while the image is being sent (Esc and the backdrop do nothing then); otherwise
  // what you picked stays for next time. Dropping an image anywhere on the open dialog adds it.
  import { tick } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import UploadForm from './UploadForm.svelte';
  import { useToolButtons } from '$lib/components/btn';
  import { closeUpload, upload } from './upload.svelte';

  // The dialog is a tool surface wherever it opens: tool buttons and the tools' field labels.
  useToolButtons();
  let dialog: HTMLDialogElement;
  let form = $state<UploadForm>();
  let busy = $state(false);
  let opener: Element | null = null;

  $effect(() => {
    if (upload.open && !dialog.open) {
      opener = document.activeElement;
      dialog.showModal();
      tick().then(() => dialog.querySelector<HTMLElement>('.drop .ui-btn, .form input')?.focus());
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
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog class="ui-dialog upload-dialog" bind:this={dialog} aria-labelledby="upload-title" aria-describedby="upload-line"
  oncancel={cancel} onclose={closed} onclick={backdrop}
  ondragover={(e) => e.preventDefault()} ondrop={(e) => { e.preventDefault(); form?.add(e.dataTransfer?.files); }}>
  <div class="panel">
    <header class="top">
      <div class="text">
        <h2 id="upload-title">Upload an emote</h2>
        <p id="upload-line">Public once a moderator approves it. You can use it in your own sets straight away.</p>
      </div>
      <button class="ui-icon close" type="button" aria-label="Close" onclick={close} disabled={busy}><Icon name="x" size={18} /></button>
    </header>
    <div class="scroll">
      <UploadForm bind:this={form} bind:busy onclose={close} />
    </div>
  </div>
</dialog>

<style>
  /* clip, not hidden: focus and scrollIntoView can't shift the panel inside the dialog. */
  .upload-dialog { width: min(960px, calc(100vw - 32px)); height: min(680px, calc(100dvh - 48px)); max-height: calc(100dvh - 48px); overflow: clip; padding: 0; }
  .panel { display: grid; grid-template-rows: auto minmax(0, 1fr); height: 100%; }
  .top { display: flex; align-items: flex-start; gap: 12px; padding: 14px 16px 12px; border-bottom: 1px solid var(--border); }
  .text { flex: 1; min-width: 0; }
  h2 { margin: 0; font-size: 18px; font-weight: 600; letter-spacing: -.01em; line-height: 1.35; }
  .top p { margin: 2px 0 0; font-size: var(--t-small); color: var(--muted); }
  .close { flex: none; margin: -4px -6px 0 0; }
  /* The form scrolls its own columns. */
  .scroll { min-height: 0; overflow: hidden; }
  /* Phones: a full-height sheet; the title bar stays put while the form scrolls. */
  @media (max-width: 639.98px) {
    .upload-dialog { width: 100vw; max-width: 100vw; height: 100dvh; max-height: 100dvh; margin: 0; border: 0; border-radius: 0;
      /* Full screen: it fades in place, with no scale (it would show the page edge behind it). */
      transition-property: opacity, overlay, display; scale: 1; }
    .top { padding-top: calc(12px + env(safe-area-inset-top, 0px)); }
  }
</style>
