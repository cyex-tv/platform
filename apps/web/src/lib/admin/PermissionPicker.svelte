<script lang="ts">
  // The role editor's permissions: grouped, each with a plain name and a tooltip that says what
  // it lets someone do (shown on hover and on keyboard focus, and read out as the checkbox's
  // description). A group's own checkbox picks or clears the whole group, and is indeterminate
  // while only part of it is on. What you can't grant (you don't hold it) can't be turned on.
  import { groupsFor, groupState, toggleGroup, type PermissionGroup } from './permissions';

  let {
    available,
    picked = $bindable(),
    canChange,
    disabled = false,
    idPrefix = 'perm',
  }: {
    /** The API's permission names. */
    available: string[];
    picked: string[];
    /** Whether this permission may be turned on or off by you. */
    canChange: (key: string) => boolean;
    disabled?: boolean;
    idPrefix?: string;
  } = $props();

  const groups = $derived(groupsFor(available));
  const set = $derived(new Set(picked));

  function toggle(key: string, on: boolean) {
    picked = on ? [...picked, key] : picked.filter((k) => k !== key);
  }

  // Svelte can't bind `indeterminate`: set it on the element.
  function partial(node: HTMLInputElement, on: boolean) {
    node.indeterminate = on;
    return { update: (v: boolean) => (node.indeterminate = v) };
  }

  const count = (g: PermissionGroup) => g.items.filter((i) => set.has(i.key)).length;
  const locked = (g: PermissionGroup) => disabled || g.items.every((i) => !canChange(i.key));
</script>

<div class="groups">
  {#each groups as g (g.id)}
    {@const state = groupState(g, picked)}
    <fieldset class="group">
      <legend class="sr-only">{g.label}</legend>
      <label class="head">
        <input type="checkbox" checked={state === 'all'} use:partial={state === 'some'} disabled={locked(g)}
          aria-label="{g.label}: all {g.items.length}" onchange={() => (picked = toggleGroup(g, picked, canChange))} />
        <span class="name">{g.label}</span>
        <span class="n ui-num" aria-hidden="true">{count(g)}/{g.items.length}</span>
      </label>
      <ul>
        {#each g.items as i (i.key)}
          {@const on = set.has(i.key)}
          {@const can = !disabled && canChange(i.key)}
          <li class="item" class:off={!can}>
            <label>
              <input type="checkbox" checked={on} disabled={!can} aria-describedby="{idPrefix}-{i.key}"
                onchange={(e) => toggle(i.key, e.currentTarget.checked)} />
              <span class="lbl">{i.label}</span>
            </label>
            <span class="tip" role="tooltip" id="{idPrefix}-{i.key}">
              {i.hint}{#if !can && !disabled}<span class="why"> You can only grant what you hold.</span>{/if}
              <span class="key ui-mono">{i.key}</span>
            </span>
          </li>
        {/each}
      </ul>
    </fieldset>
  {/each}
</div>

<style>
  .groups { display: grid; grid-template-columns: repeat(auto-fill, minmax(230px, 1fr)); gap: 8px; }
  .group { margin: 0; padding: 4px 0 6px; border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--bg); min-width: 0; }
  .head { display: flex; align-items: center; gap: 8px; min-height: 36px; padding: 0 10px; cursor: pointer;
    border-bottom: 1px solid var(--border); margin-bottom: 4px; }
  .head .name { flex: 1; font-size: var(--text-sub); font-weight: 600; }
  .n { font-size: 12px; color: var(--dim); }
  input { width: 16px; height: 16px; margin: 0; flex: none; accent-color: var(--accent); cursor: pointer; }
  input:disabled { cursor: not-allowed; }
  ul { list-style: none; margin: 0; padding: 0; }
  .item { position: relative; }
  .item label { display: flex; align-items: center; gap: 8px; min-height: 32px; padding: 0 10px; cursor: pointer; border-radius: var(--radius);
    font-size: var(--text-sub); }
  .item.off label { color: var(--dim); cursor: not-allowed; }
  .lbl { min-width: 0; }
  @media (pointer: coarse) { .item label, .head { min-height: 44px; } }

  /* The tooltip: below the row, after the usual delay on hover; at once on keyboard focus. */
  .tip { position: absolute; left: 10px; right: 10px; top: calc(100% + 2px); z-index: var(--z-tip); display: grid; gap: 4px;
    padding: 7px 9px; border-radius: var(--radius-btn); background: var(--elevated, var(--surface)); color: var(--text);
    border: 1px solid var(--border-strong); box-shadow: var(--shadow-pop); font-size: 12px; line-height: 1.45; font-weight: 400;
    opacity: 0; transform: translateY(-2px); pointer-events: none; transition: opacity var(--fast), transform var(--fast); }
  .why { color: var(--muted); }
  .key { color: var(--dim); font-size: 11px; }
  .item:has(input:focus-visible) .tip { opacity: 1; transform: none; }
  @media (hover: hover) and (pointer: fine) {
    .item label:hover { background: var(--row-hover); }
    .item:hover .tip { opacity: 1; transform: none; transition-delay: var(--delay-tip, .4s); }
  }
  @media (prefers-reduced-motion: reduce) { .tip { transform: none; transition: opacity var(--fast); } }
</style>
