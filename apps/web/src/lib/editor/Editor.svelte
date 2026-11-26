<script lang="ts">
  // The emote editor, the upload dialog's second mode (loaded only when you switch to it). It opens
  // any picture or video in this tab, nothing is sent anywhere, and hands the finished file to the
  // quick upload form ("Continue to upload"), which uploads it as it uploads a picked file.
  //   top     the file, undo / redo, open another file, start over
  //   left    the tools (each with its key in the tooltip)
  //   middle  the stage, the frames under it
  //   right   preview, the tool's settings, turning and size, file format, the upload rules
  // Phones: one column, the tools a strip over the stage, everything else under the frames.
  // Keys (not while typing): V C P L A R O T H tools, Ctrl + Z undo, Ctrl + Shift + Z or Ctrl + Y
  // redo, K play, , and . step, [ and ] turn, + − zoom, 0 fit, 1 actual size.
  import { onMount } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import EditorIcon, { type EditorIconName } from './EditorIcon.svelte';
  import Stage from './Stage.svelte';
  import Timeline from './Timeline.svelte';
  import Panel from './Panel.svelte';
  import { EditorState, TOOLS, type Tool } from './state.svelte';
  import { AUTO_ENCODE_PIXELS } from './state.svelte';
  import { EDITOR_ACCEPT } from './sniff';
  import { rotateCcw, rotateCw } from './model';
  import { loadLimits } from '$lib/cosmetics/limits';
  import { bytes, count } from '$lib/format';
  import type { EmoteCaps } from '$lib/upload/upload-links';
  import LeaveGuard from '$lib/tool/LeaveGuard.svelte';

  type Props = {
    /** The account's own caps (uploadQuota), when read. */
    caps?: EmoteCaps | null;
    /** The finished file. */
    oncontinue: (file: File) => void;
    /** Whether the editor is the dialog's visible mode (its keys work only then). */
    active?: boolean;
  };
  let { caps = null, oncontinue, active = true }: Props = $props();
  const uid = $props.id();

  const ed = new EditorState();
  let stage = $state<Stage>();
  let input: HTMLInputElement;
  let over = $state(false);
  let finishing = $state(false);
  let emoteLimits = $state.raw<Parameters<EditorState['setLimits']>[0]>(null);

  onMount(() => {
    void loadLimits()
      .then((l) => (emoteLimits = l.emote))
      .catch(() => {});
    return () => {
      ed.pause();
      ed.cancelImport();
      ed.cancelEncode();
    };
  });
  $effect(() => ed.setLimits(emoteLimits, caps));

  /** Opens a file (from the quick upload form, a drop, or Browse). */
  export function open(file: File) {
    ed.open(file);
  }

  /** Whether there are edits that would be lost. */
  export function dirty() {
    return ed.dirty;
  }

  // The size rule needs the real file: encode once edits settle (big animations on request).
  $effect(() => {
    const d = ed.doc;
    void ed.format;
    void ed.quality;
    if (!d || ed.importing || ed.pixels > AUTO_ENCODE_PIXELS) return;
    const t = setTimeout(() => void ed.encode(), 600);
    return () => {
      clearTimeout(t);
    };
  });

  async function cont() {
    if (finishing) return;
    finishing = true;
    try {
      const file = await ed.finish();
      if (file) oncontinue(file);
    } finally {
      finishing = false;
    }
  }

  const typing = (t: EventTarget | null) => t instanceof HTMLElement && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName));

  function keydown(e: KeyboardEvent) {
    if (!active || !ed.doc) return;
    const mod = e.ctrlKey || e.metaKey;
    const k = e.key.toLowerCase();
    if (mod && (k === 'z' || k === 'y')) {
      // A text field keeps its own undo.
      if (typing(e.target)) return;
      e.preventDefault();
      if (k === 'y' || e.shiftKey) ed.redo();
      else ed.undo();
      return;
    }
    if (mod || e.altKey || typing(e.target)) return;
    const tool = TOOLS.find((t) => t.key.toLowerCase() === k);
    if (tool) {
      e.preventDefault();
      setTool(tool.id);
      return;
    }
    const d = ed.doc;
    switch (e.key) {
      case 'k':
      case 'K':
        e.preventDefault();
        return ed.toggle();
      case ',':
        e.preventDefault();
        return ed.show((ed.current - 1 + d.frames.length) % d.frames.length);
      case '.':
        e.preventDefault();
        return ed.show((ed.current + 1) % d.frames.length);
      case '[':
        e.preventDefault();
        return ed.commit(rotateCcw(d), 'Rotate left');
      case ']':
        e.preventDefault();
        return ed.commit(rotateCw(d), 'Rotate right');
      case '+':
      case '=':
        e.preventDefault();
        return stage?.zoomBy(1.25);
      case '-':
        e.preventDefault();
        return stage?.zoomBy(1 / 1.25);
      case '0':
        e.preventDefault();
        return stage?.fit();
      case '1':
        e.preventDefault();
        return stage?.actual();
    }
  }

  function setTool(t: Tool) {
    ed.tool = t;
    if (t !== 'select' && t !== 'text') ed.shape = null;
  }

  function drop(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    over = false;
    const f = e.dataTransfer?.files?.[0];
    if (f) ed.open(f);
  }

  const icon = (n: string) => n as EditorIconName;
  const progress = $derived(ed.importing);
  const pct = $derived(progress && progress.total ? Math.round((progress.done / progress.total) * 100) : 0);
  let discardArmed = $state(false);
  let disarm: ReturnType<typeof setTimeout> | undefined;
  function discard() {
    if (!ed.dirty || discardArmed) {
      clearTimeout(disarm);
      discardArmed = false;
      ed.discard();
      return;
    }
    discardArmed = true;
    disarm = setTimeout(() => (discardArmed = false), 3000);
  }
</script>

<svelte:window onkeydown={keydown} />
<!-- Closing or reloading the tab loses the edits; the dialog keeps them while you browse. -->
<LeaveGuard dirty={ed.dirty} inApp={false} what="your emote edits" />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="ed" class:empty={!ed.doc} ondragover={(e) => { e.preventDefault(); over = true; }} ondragleave={(e) => { if (e.currentTarget === e.target) over = false; }} ondrop={drop}>
  {#if ed.doc}
    <div class="top">
      <p class="file" title={ed.name}><EditorIcon name="film" size={15} /><bdi>{ed.name}</bdi></p>
      <div class="hist" role="group" aria-label="History">
        <button type="button" class="tb" disabled={!ed.canUndo} onclick={() => ed.undo()} aria-label={ed.undoLabel ? `Undo ${ed.undoLabel}` : 'Undo'} title="Undo{ed.undoLabel ? ` ${ed.undoLabel}` : ''} (Ctrl+Z)"><EditorIcon name="undo" size={16} /></button>
        <button type="button" class="tb" disabled={!ed.canRedo} onclick={() => ed.redo()} aria-label={ed.redoLabel ? `Redo ${ed.redoLabel}` : 'Redo'} title="Redo{ed.redoLabel ? ` ${ed.redoLabel}` : ''} (Ctrl+Shift+Z)"><EditorIcon name="redo" size={16} /></button>
      </div>
      <div class="files">
        <button type="button" class="ui-btn ghost small" onclick={() => input.click()}>Open another</button>
        <button type="button" class="ui-btn small" class:danger={discardArmed} class:ghost={!discardArmed} onclick={discard} onblur={() => (discardArmed = false)}>
          {discardArmed ? 'Click again to discard edits' : 'Start over'}
        </button>
      </div>
    </div>
    {#if ed.note}
      <p class="note" role="status"><span>{ed.note}</span><button type="button" class="ui-icon x" aria-label="Dismiss" onclick={() => (ed.note = null)}><Icon name="x" size={14} /></button></p>
    {/if}
    <div class="work">
      <div class="rail" role="toolbar" aria-label="Tools" aria-orientation="vertical">
        {#each TOOLS as t (t.id)}
          <button type="button" class="tool" aria-pressed={ed.tool === t.id} aria-label={t.label} aria-keyshortcuts={t.key} onclick={() => setTool(t.id)}>
            <EditorIcon name={icon(t.icon)} size={18} />
            <span class="tip" aria-hidden="true">{t.label}<kbd>{t.key}</kbd></span>
          </button>
          {#if t.id === 'crop' || t.id === 'text'}<span class="sep" aria-hidden="true"></span>{/if}
        {/each}
      </div>
      <div class="centre">
        <Stage {ed} bind:this={stage} />
        <Timeline {ed} />
      </div>
      <Panel {ed} oncontinue={cont} />
    </div>
  {:else}
    <div class="start" class:over>
      {#if progress}
        <!-- Reading a file: how far, and a way out. -->
        <div class="reading" role="status" aria-live="polite">
          <span class="ui-spinner" aria-hidden="true"></span>
          <p class="lead">{progress.total ? `Reading frames… ${count(progress.done)} of ${count(progress.total)}` : 'Opening the file…'}</p>
          <span class="bar" role="progressbar" aria-label="Reading the file" aria-valuemin="0" aria-valuemax="100" aria-valuenow={pct}><span style:transform="scaleX({Math.max(0.03, pct / 100)})"></span></span>
          <button type="button" class="ui-btn secondary" onclick={() => ed.cancelImport()}>Cancel</button>
        </div>
      {:else}
        <EditorIcon name="film" size={28} />
        <p class="lead">Drop a picture or a video here</p>
        <p class="sub">PNG, JPEG, GIF, WebP, AVIF, MP4 or WebM, any size. It stays in this browser until you upload the result.</p>
        <button type="button" class="ui-btn secondary" onclick={() => input.click()}>Browse files</button>
        {#if ed.importError}<p class="ui-error" role="alert">{ed.importError} <button type="button" class="ui-btn ghost small" onclick={() => input.click()}>Pick another file</button></p>{/if}
      {/if}
    </div>
  {/if}
  {#if ed.doc && progress}
    <div class="overlay" role="status" aria-live="polite">
      <div class="reading">
        <span class="ui-spinner" aria-hidden="true"></span>
        <p class="lead">{progress.total ? `Reading frames… ${count(progress.done)} of ${count(progress.total)}` : 'Opening the file…'}</p>
        <span class="bar" role="progressbar" aria-label="Reading the file" aria-valuemin="0" aria-valuemax="100" aria-valuenow={pct}><span style:transform="scaleX({Math.max(0.03, pct / 100)})"></span></span>
        <button type="button" class="ui-btn secondary" onclick={() => ed.cancelImport()}>Cancel</button>
      </div>
    </div>
  {/if}
  {#if ed.pending}
    <div class="overlay">
      <div class="ask" role="alertdialog" aria-labelledby="ask-{uid}" aria-describedby="ask-line-{uid}">
        <h3 id="ask-{uid}">Open <bdi>{ed.pending.name}</bdi>?</h3>
        <p id="ask-line-{uid}">Your edits to <bdi>{ed.name}</bdi> will be lost. {bytes(ed.pending.size)}.</p>
        <div class="ui-actions">
          <button type="button" class="ui-btn secondary" onclick={() => (ed.pending = null)}>Keep editing</button>
          <button type="button" class="ui-btn primary" onclick={() => ed.pending && ed.open(ed.pending, true)}>Open it</button>
        </div>
      </div>
    </div>
  {/if}
  {#if ed.doc && ed.importError}
    <p class="toast ui-error" role="alert">{ed.importError} <button type="button" class="ui-icon x" aria-label="Dismiss" onclick={() => (ed.importError = null)}><Icon name="x" size={14} /></button></p>
  {/if}
  <input bind:this={input} class="sr-only" type="file" tabindex="-1" accept={EDITOR_ACCEPT}
    onchange={(e) => { const f = e.currentTarget.files?.[0]; if (f) ed.open(f); e.currentTarget.value = ''; }} />
  {#if finishing}<span class="sr-only" role="status">Encoding the emote…</span>{/if}
</div>

<style>
  .ed { position: relative; display: grid; grid-template-rows: auto auto minmax(0, 1fr); height: 100%; min-height: 0; min-width: 0; }
  .ed.empty { grid-template-rows: minmax(0, 1fr); }
  .top { display: flex; align-items: center; gap: 8px; min-width: 0; padding: 4px 8px 4px 12px; border-bottom: 1px solid var(--border); }
  .file { display: flex; align-items: center; gap: 8px; flex: 1; min-width: 0; margin: 0; font-size: var(--t-small); font-weight: 600; color: var(--text); }
  .file :global(svg) { flex: none; color: var(--dim); }
  .file bdi { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hist, .files { display: flex; align-items: center; gap: 2px; }
  .files { gap: 4px; }
  .tb { display: inline-grid; place-items: center; width: 32px; height: 32px; padding: 0; border: 0; border-radius: var(--radius-btn); background: none; color: var(--muted); cursor: pointer;
    transition: color var(--dur-color), background-color var(--dur-color); }
  .tb:disabled { opacity: .4; cursor: not-allowed; }
  .tb:active:not(:disabled) { transform: scale(var(--press)); }
  .tb:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .note { display: flex; align-items: center; gap: 8px; margin: 0; padding: 6px 8px 6px 12px; border-bottom: 1px solid var(--border); background: var(--elevated);
    font-size: var(--t-fine); color: var(--muted); box-shadow: inset 3px 0 0 var(--accent); }
  .note span { flex: 1; min-width: 0; }
  .x { width: 28px; height: 28px; }
  .ed:not(:has(.note)) { grid-template-rows: auto minmax(0, 1fr); }
  .ed.empty { grid-template-rows: minmax(0, 1fr); }
  .work { display: grid; grid-template-columns: 52px minmax(0, 1fr) minmax(300px, 340px); min-height: 0; min-width: 0; }
  .centre { display: grid; grid-template-rows: minmax(0, 1fr) auto; min-width: 0; min-height: 0; }

  .rail { display: flex; flex-direction: column; align-items: center; gap: 2px; padding: 8px 0; border-right: 1px solid var(--border); background: var(--surface); }
  .sep { width: 24px; height: 1px; margin: 4px 0; background: var(--border); }
  .tool { position: relative; display: grid; place-items: center; width: 40px; height: 40px; padding: 0; border: 0; border-radius: var(--radius-btn); background: none; color: var(--muted); cursor: pointer;
    transition: color var(--dur-color), background-color var(--dur-color), transform var(--dur-press) ease-out; }
  /* Picked: the accent icon, no fill (the site's selected-state rule). */
  .tool[aria-pressed='true'] { color: var(--accent-text); }
  .tool:active { transform: scale(var(--press)); }
  .tool:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .tip { position: absolute; left: calc(100% + 8px); top: 50%; z-index: var(--z-tip); display: flex; align-items: center; gap: 8px; padding: 5px 8px; border-radius: var(--radius-btn);
    background: var(--text); color: var(--bg); font-size: var(--t-fine); font-weight: 500; white-space: nowrap; pointer-events: none;
    opacity: 0; transform: translateY(-50%) scale(.97); transform-origin: 0 50%; transition: opacity var(--dur-pop) ease-out, transform var(--dur-pop) ease-out; }
  kbd { padding: 0 4px; border: 1px solid color-mix(in srgb, var(--bg) 40%, transparent); border-radius: 2px; font: inherit; font-family: var(--font-mono); font-size: 11px; }
  .tool:focus-visible .tip { opacity: 1; transform: translateY(-50%); }
  @media (hover: hover) and (pointer: fine) {
    .tool:hover { color: var(--text); background: var(--surface-hover); }
    .tool[aria-pressed='true']:hover { color: var(--accent-text); }
    .tool:hover .tip { opacity: 1; transform: translateY(-50%); transition-delay: var(--delay-tip); }
    .tb:hover:not(:disabled) { color: var(--text); background: var(--surface-hover); }
  }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .tip { transition: none; } }

  .start { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; margin: 20px; padding: 24px; min-height: 0;
    border: 1px dashed var(--border-strong); border-radius: var(--radius-card); color: var(--muted); text-align: center; transition: border-color var(--fast), background-color var(--fast); }
  .start.over { border-color: var(--accent-border); background: var(--surface-hover); }
  .start :global(svg) { color: var(--dim); }
  .lead { margin: 0; font-size: var(--t-ui, 14px); font-weight: 600; color: var(--text); }
  .sub { margin: 0; max-width: 46ch; font-size: var(--t-fine); color: var(--muted); }
  .start .ui-error { margin: 0; }
  .reading { display: grid; justify-items: center; gap: 12px; min-width: min(320px, 90%); }
  .bar { display: block; width: min(280px, 100%); height: 3px; border-radius: var(--radius-sm); background: var(--border); overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--accent); transform-origin: 0 50%; transition: transform var(--fast) linear; }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .bar span { transition: none; } }
  .overlay { position: absolute; inset: 0; z-index: 5; display: grid; place-items: center; padding: 16px; background: var(--scrim); }
  .overlay .reading { padding: 20px 24px; background: var(--elevated); border: 1px solid var(--border-strong); border-radius: var(--radius-card); box-shadow: var(--shadow-dialog); color: var(--text); }
  .ask { display: grid; gap: 10px; max-width: 420px; padding: 16px; background: var(--elevated); border: 1px solid var(--border-strong); border-radius: var(--radius-card); box-shadow: var(--shadow-dialog); }
  .ask h3 { margin: 0; font-size: 16px; overflow-wrap: anywhere; }
  .ask p { margin: 0; font-size: var(--t-small); color: var(--muted); overflow-wrap: anywhere; }
  .ask .ui-actions { justify-content: flex-end; }
  .toast { position: absolute; left: 12px; bottom: 12px; z-index: 6; display: flex; align-items: center; gap: 8px; margin: 0; padding: 8px 8px 8px 12px; max-width: min(420px, calc(100% - 24px));
    background: var(--elevated); border: 1px solid var(--border-strong); border-radius: var(--radius-btn); box-shadow: var(--shadow-pop); }

  /* Narrower: the panel goes under the stage and frames; the whole editor scrolls. */
  @media (max-width: 1099.98px) {
    /* max-content rows: the frames and the panel keep their height and the editor scrolls. */
    .work { grid-template-columns: 52px minmax(0, 1fr); grid-template-rows: max-content; grid-auto-rows: max-content; overflow: hidden auto; overscroll-behavior: contain; }
    .rail { grid-row: 1; }
    .centre { grid-template-rows: min(56dvh, 520px) auto; }
    .work > :global(.panel) { grid-column: 1 / -1; }
  }
  /* Phones: the tools become a strip above the stage. */
  @media (max-width: 719.98px) {
    .work { grid-template-columns: minmax(0, 1fr); }
    .rail { flex-direction: row; overflow-x: auto; scrollbar-width: none; padding: 4px 8px; border-right: 0; border-bottom: 1px solid var(--border); }
    .sep { width: 1px; height: 24px; margin: 0 4px; flex: none; }
    .tool { flex: none; width: 44px; height: 44px; }
    .tip { display: none; }
    .centre { grid-row: 2; grid-template-rows: min(48dvh, 420px) auto; }
    .work > :global(.panel) { grid-row: 3; }
    .files .ui-btn { padding: 0 8px; }
  }
  @media (pointer: coarse) { .tb { width: 44px; height: 44px; } }
</style>
