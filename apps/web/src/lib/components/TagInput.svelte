<script lang="ts">
  // A field of tags: type a tag and press Enter (or a comma) and it becomes a chip in the field;
  // pasting "a, b, c" adds three. Each chip has a remove button; Backspace in the empty field
  // takes the last one off. The field and its chips are one box, like a text field. Used by the
  // browse filters and (later) the upload form. `onchange` gets the whole new list.
  import Icon from './Icon.svelte';
  import { btnFamily } from './btn';
  import { addTags, defaultClean, keyAction, removeAt } from './tag-input';

  type Props = {
    tags: string[];
    onchange: (tags: string[]) => void;
    /** The field's visible or screen-reader label. */
    label: string;
    /** Show the label above the field (otherwise it is for screen readers only). */
    showLabel?: boolean;
    placeholder?: string;
    max?: number;
    clean?: (raw: string) => string;
    /** Characters a tag may have, as a hint under the field (e.g. "Letters, numbers and _"). */
    hint?: string;
    id?: string;
    disabled?: boolean;
  };
  // Same on the server and in the browser, so hydration keeps the label tied to the field.
  const uid = $props.id();
  let {
    tags,
    onchange,
    label,
    showLabel = false,
    placeholder = 'Add a tag',
    max = Infinity,
    clean = defaultClean,
    hint,
    id = `tags-${uid}`,
    disabled = false,
  }: Props = $props();

  // On the tool pages a visible label is the tools' uppercase field label, like the fields around it.
  const tool = btnFamily() === 'tool';
  let text = $state('');
  let input = $state<HTMLInputElement>();
  /** What the last change did, for screen readers (and the note under the field). */
  let said = $state('');
  let note = $state('');
  // Full: typing adds nothing, but Backspace and the remove buttons still work.
  const full = $derived(tags.length >= max);
  const shownNote = $derived(note || (full ? `Up to ${max} tags.` : (hint ?? '')));

  function add(raw: string) {
    const r = addTags(tags, raw, { clean, max });
    note = r.over.length ? `Up to ${max} tags.` : r.repeated.length ? `#${r.repeated[0]} is already there.` : '';
    const added = r.tags.slice(tags.length);
    if (added.length) {
      said = `Added ${added.map((t) => `#${t}`).join(', ')}.`;
      onchange(r.tags);
    }
    text = '';
  }
  function remove(i: number, refocus = true) {
    const gone = tags[i];
    said = `Removed #${gone}.`;
    note = '';
    onchange(removeAt(tags, i));
    if (refocus) input?.focus();
  }
  function key(e: KeyboardEvent) {
    const a = keyAction(e.key, text, tags.length);
    if (a === 'add') {
      e.preventDefault();
      add(text);
    } else if (a === 'pop') {
      e.preventDefault();
      remove(tags.length - 1);
    }
  }
  function typed() {
    // A comma typed or pasted mid-text splits there.
    if (/[,\n]/.test(text)) add(text);
    else if (note) note = '';
  }
  function paste(e: ClipboardEvent) {
    const t = e.clipboardData?.getData('text') ?? '';
    if (!/[,\n]/.test(t)) return;
    e.preventDefault();
    add(text + t);
  }
</script>

<div class="taginput" class:disabled>
  <label class:sr-only={!showLabel} class:upper={tool} class="lbl" for={id}>{label}</label>
  <!-- A click on the box's empty space goes to the text field, as in a plain input. -->
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="box" onclick={(e) => e.target === e.currentTarget && input?.focus()}>
    {#if tags.length}
      <ul class="chips" aria-label="{label}: {tags.length}">
        {#each tags as t, i (t)}
          <li class="chip">
            <bdi>#{t}</bdi>
            <button type="button" class="x" aria-label="Remove #{t}" {disabled} onclick={() => remove(i)}><Icon name="x" size={14} /></button>
          </li>
        {/each}
      </ul>
    {/if}
    <input
      {id}
      bind:this={input}
      bind:value={text}
      type="text"
      enterkeyhint="done"
      autocomplete="off"
      autocapitalize="off"
      spellcheck="false"
      placeholder={full ? '' : tags.length ? 'Add another' : placeholder}
      aria-describedby={shownNote ? `${id}-note` : undefined}
      {disabled}
      onkeydown={key}
      oninput={typed}
      onpaste={paste}
      onblur={() => text.trim() && add(text)}
    />
  </div>
  {#if shownNote}<p class="note" id="{id}-note">{shownNote}</p>{/if}
  <p class="sr-only" aria-live="polite">{said}</p>
</div>

<style>
  .taginput { display: grid; gap: 6px; min-width: 0; }
  .lbl { font-size: var(--t-small); font-weight: 500; color: var(--text); }
  .lbl.upper { font-size: var(--text-label); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  /* One box like a text field: the chips wrap, the input takes the rest of the last line. */
  .box { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; min-height: 40px; padding: 4px 6px; min-width: 0;
    border: 1px solid var(--field-border); border-radius: var(--radius-btn); background: var(--bg); cursor: text;
    transition: border-color var(--fast); }
  .box:has(input:focus-visible) { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  .chips { display: contents; list-style: none; margin: 0; padding: 0; }
  .chip { display: inline-flex; align-items: center; gap: 2px; min-width: 0; max-width: 100%; height: 28px; padding: 0 2px 0 8px;
    border: 1px solid var(--border-strong); border-radius: var(--radius-btn); background: var(--surface);
    color: var(--text); font-size: 13px; font-weight: 500; white-space: nowrap; }
  .chip bdi { min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .x { position: relative; flex: none; display: inline-grid; place-items: center; width: 24px; height: 24px; padding: 0; border: 0;
    border-radius: var(--radius-btn); background: none; color: var(--muted); cursor: pointer;
    transition: color var(--fast), background-color var(--fast); }
  .x::after { content: ''; position: absolute; inset: -9px -4px; } /* 40px to hit */
  .x:active { opacity: .6; transition: none; }
  .x:focus-visible { outline-offset: 0; }
  input { flex: 1 1 96px; min-width: 96px; height: 30px; padding: 0 4px; border: 0; background: none; color: var(--text);
    font: inherit; font-size: 16px; outline: none; }
  input::placeholder { color: var(--dim); }
  input:disabled { cursor: default; }
  .note { margin: 0; font-size: var(--t-fine); color: var(--dim); }
  .disabled .box { opacity: .6; cursor: default; }
  @media (pointer: coarse) { .chip { height: 32px; } .x { width: 28px; height: 28px; } .x::after { inset: -6px -2px; } }
  @media (hover: hover) and (pointer: fine) {
    .box:hover { border-color: var(--muted); }
    .x:hover { color: var(--text); background: var(--surface-2); }
  }
  @media (forced-colors: active) { .chip { border-color: CanvasText; } }
</style>
