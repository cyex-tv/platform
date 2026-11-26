<script lang="ts">
  // The one modal dialog (the .ui-dialog look from tool.css): a native <dialog>, so focus stays
  // inside, the page behind is inert, Escape closes it and focus goes back to what opened it.
  // `busy` locks it while something is being sent: Escape and the backdrop do nothing.
  //   <Dialog bind:this={dlg} title="New set" {busy} onsubmit={save}>
  //     …fields…
  //     {#snippet footer()}<button …>Cancel</button>…{/snippet}
  //   </Dialog>      dlg.open()   dlg.close()
  // With `onsubmit` the content sits in a <form novalidate>. Without `title` pass `label`
  // (the accessible name). `bare` leaves out the .body wrapper (content that brings its own).
  import type { Snippet } from 'svelte';

  type Props = {
    title?: string;
    /** A title with markup in it, in place of `title`; it is wrapped in the h2. */
    heading?: Snippet;
    /** The accessible name when there is no visible `title`. */
    label?: string;
    /** Id for the title (aria-labelledby); one is made when left out. */
    titleId?: string;
    describedby?: string;
    wide?: boolean;
    /** Extra class on the <dialog>, for the caller's own styles (via :global). */
    class?: string;
    busy?: boolean;
    /** A click on the dimmed page closes it (off by default: forms keep what was typed). */
    backdrop?: boolean;
    /** Esc was pressed. Call `event.preventDefault()` to keep the dialog open. */
    oncancel?: (e: Event) => void;
    onclose?: () => void;
    onsubmit?: (e: SubmitEvent) => void;
    bare?: boolean;
    children?: Snippet;
    footer?: Snippet;
  };
  let { title, heading, label, titleId, describedby, wide = false, class: className = '', busy = false, backdrop = false, oncancel, onclose, onsubmit, bare = false, children, footer }: Props = $props();

  const generated = $props.id();
  const headingId = $derived(titleId ?? `${generated}-title`);
  let dialog: HTMLDialogElement;

  let opener: HTMLElement | null = null;

  export function open() {
    opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialog.showModal();
  }
  export function close() {
    dialog.close();
  }
  /** The <dialog> itself, for queries inside it. */
  export function element() {
    return dialog;
  }
  export function isOpen() {
    return !!dialog?.open;
  }

  function closed() {
    // The browser returns focus on its own; this covers the ones that don't (and a test DOM).
    if (opener?.isConnected && document.activeElement !== opener) opener.focus();
    opener = null;
    onclose?.();
  }

  function cancel(e: Event) {
    if (busy) e.preventDefault();
    else oncancel?.(e);
  }

  function click(e: MouseEvent) {
    // A click on the dialog box itself (not its content) lands on the backdrop.
    if (backdrop && !busy && e.target === dialog) dialog.close();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog
  class={['ui-dialog', className]}
  class:wide
  bind:this={dialog}
  aria-labelledby={title || heading ? headingId : titleId}
  aria-label={title || heading ? undefined : label}
  aria-describedby={describedby}
  oncancel={cancel}
  onclose={closed}
  onclick={click}
>
  {#snippet content()}
    {#if bare}
      {@render children?.()}
    {:else}
      <div class="body">
        {#if title}<h2 id={headingId}>{title}</h2>{:else if heading}<h2 id={headingId}>{@render heading()}</h2>{/if}
        {@render children?.()}
      </div>
    {/if}
    {#if footer}<footer>{@render footer()}</footer>{/if}
  {/snippet}
  {#if onsubmit}
    <form {onsubmit} novalidate>{@render content()}</form>
  {:else}
    {@render content()}
  {/if}
</dialog>

<style>
  .wide { width: min(640px, calc(100vw - 24px)); }
</style>
