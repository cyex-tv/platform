<script lang="ts">
  // The permissions dialog for an account editor (ADR 0014): which sets they may edit (a dropdown:
  // all, new ones included, or a few), "Everything", and the permissions in small groups, two
  // columns of one-line rows; what each one allows is a tooltip (hover, or keyboard focus) read out
  // as the box's description. It fits the window without scrolling; on phones it is a full-height
  // sheet whose body scrolls and whose footer stays put. Adding someone and changing an editor open
  // the same dialog; Save applies at once (no invitation step).
  //   <EditorDialog bind:this={d} held={held} mine={scope} sets={sets} onsave={...} />
  //   d.open(account, { permissions, scope }, 'add' | 'edit')
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { clean } from '$lib/format';
  import type { EditorPermission } from '$lib/gql/graphql';
  import SetScopePicker from './SetScopePicker.svelte';
  import { ALL_SETS, GROUPS, allOn, grantable, setAll, toggle, type Scope } from './editors';

  type Who = { id: string; displayName: string; avatar?: string | null };
  type SetOption = { id: string; name: string; kind?: string };

  let {
    held,
    mine = ALL_SETS,
    sets,
    onsave,
    describe,
  }: {
    /** The permissions the caller may grant on this account. */
    held: ReadonlySet<EditorPermission>;
    /** The sets the caller may grant; a limited manager can't hand out "All sets". */
    mine?: Scope;
    /** The account's sets, for the "Sets they can edit" dropdown. */
    sets: readonly SetOption[];
    onsave: (who: Who, permissions: EditorPermission[], scope: Scope) => Promise<void>;
    /** A failed save as a sentence shown in the dialog. */
    describe: (e: unknown) => string;
  } = $props();

  let dialog: HTMLDialogElement;
  let who = $state<Who | null>(null);
  let mode = $state<'add' | 'edit'>('add');
  let picked = $state<EditorPermission[]>([]);
  let scope = $state<Scope>(ALL_SETS);
  let setsOpen = $state(false);
  let busy = $state(false);
  let problem = $state('');

  const offer = $derived(grantable(held));
  const everything = $derived(allOn(picked, offer));
  const some = $derived(picked.length > 0 && !everything);

  export function open(account: Who, start: { permissions: readonly EditorPermission[]; scope: Scope }, as: 'add' | 'edit') {
    who = account;
    mode = as;
    picked = [...start.permissions];
    scope = mine.all ? start.scope : start.scope.all ? { all: false, ids: [] } : start.scope;
    setsOpen = false;
    problem = '';
    dialog.showModal();
  }

  const scopeOk = $derived(scope.all || scope.ids.length > 0);

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!who || busy || !picked.length || !scopeOk) return;
    busy = true;
    problem = '';
    try {
      await onsave(who, picked, scope);
      dialog.close();
    } catch (err) {
      problem = describe(err);
    } finally {
      busy = false;
    }
  }

  // Tab wraps around inside the dialog (the sets dropdown keeps Esc to itself while it is open).
  function keydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && shown && hushed !== shown && !setsOpen) {
      // Esc puts a tooltip away first (WCAG 1.4.13); the next Esc closes the dialog.
      e.preventDefault();
      e.stopPropagation();
      hushed = shown;
      return;
    }
    if (e.key !== 'Tab') return;
    const items = [...dialog.querySelectorAll<HTMLElement>('button:not([disabled]), input:not([disabled]), a[href]')].filter((el) => el.offsetParent !== null);
    if (!items.length) return;
    const first = items[0];
    const last = items[items.length - 1];
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  }

  // The row whose tooltip is up (hover or focus), and the one Esc put away.
  let shown = $state<string | null>(null);
  let hushed = $state<string | null>(null);
  function tipOn(key: string) {
    shown = key;
    if (hushed !== key) hushed = null;
  }
  function tipOff() {
    shown = hushed = null;
  }

  function cancel(e: Event) {
    // A save is on its way: closing now would hide whether it worked. Wait for the answer.
    if (busy) {
      e.preventDefault();
      return;
    }
    // Esc while the sets dropdown is open closes only the dropdown.
    if (setsOpen) e.preventDefault();
  }
</script>

<dialog class="ui-dialog perms-dialog" bind:this={dialog} aria-labelledby="ed-title" oncancel={cancel} onkeydown={keydown} onclose={() => { who = null; setsOpen = false; }}>
  <form onsubmit={submit} novalidate>
    <div class="head">
      {#if who}<Avatar src={who.avatar} name={who.displayName} size={28} tone="tool" />{/if}
      <h2 id="ed-title">{#if who}What <bdi>{clean(who.displayName)}</bdi> can do{/if}</h2>
      <button class="ui-icon close" type="button" aria-label="Close" disabled={busy} onclick={() => dialog.close()}><Icon name="x" size={18} /></button>
    </div>
    <div class="body">
      <div class="sets">
        <span class="ui-label" id="ed-sets-label">Sets they can edit</span>
        <SetScopePicker bind:scope bind:open={setsOpen} {mine} {sets} labelledby="ed-sets-label" />
        <span class="ui-hint">{scope.all ? 'Every set, and any set made later.' : 'Only these sets; they can’t make new ones.'}</span>
      </div>

      <label class="ui-check all" class:on={everything}>
        <input type="checkbox" checked={everything} indeterminate={some} onchange={(e) => (picked = setAll(e.currentTarget.checked, offer))} />
        <span class="name">Everything</span>
        <span class="ui-hint">{held.size < GROUPS.flatMap((g) => g.items).length ? 'that you can grant' : 'on this account'}</span>
      </label>

      <div class="groups">
        {#each GROUPS as g (g.label)}
          <fieldset class="group">
            <legend class="ui-label">{g.label}</legend>
            {#each g.items as p (p.key)}
              {@const can = held.has(p.key)}
              {@const on = picked.includes(p.key)}
              <div class="perm" class:off={!can} class:on class:hushed={hushed === p.key} role="presentation"
                onpointerenter={() => tipOn(p.key)} onpointerleave={tipOff} onfocusin={() => tipOn(p.key)} onfocusout={tipOff}>
                <label>
                  <input type="checkbox" checked={on} disabled={!can}
                    onchange={(e) => (picked = toggle(picked, p.key, e.currentTarget.checked))} aria-describedby="ed-{p.key}" />
                  <span class="name">{p.label}</span>
                </label>
                <span class="tip" role="tooltip" id="ed-{p.key}">{can ? p.line : 'You don’t hold this one.'}</span>
              </div>
            {/each}
            {#if g.note}<p class="ui-hint note">{g.note}</p>{/if}
            {#if g.needsAllSets && !scope.all && g.items.some((p) => picked.includes(p.key))}
              <p class="ui-hint note warn">Work only with All sets.</p>
            {/if}
          </fieldset>
        {/each}
      </div>

      {#if !picked.length || !scopeOk || problem}
        <div class="status">
          {#if !picked.length}<p class="ui-hint">Pick at least one{mode === 'edit' ? ', or remove them instead' : ''}.</p>{/if}
          {#if !scopeOk}<p class="ui-hint">Pick at least one set.</p>{/if}
          {#if problem}<p class="ui-error" role="alert">{problem}</p>{/if}
        </div>
      {/if}
    </div>
    <footer>
      <button class="ui-btn secondary" type="button" disabled={busy} onclick={() => dialog.close()}>Cancel</button>
      <button class="ui-btn primary" type="submit" disabled={busy || !picked.length || !scopeOk}>
        {busy ? 'Saving…' : mode === 'add' ? 'Add editor' : 'Save'}
      </button>
    </footer>
  </form>
</dialog>

<style>
  .perms-dialog { width: min(600px, calc(100vw - 24px)); }
  .perms-dialog form { max-height: calc(100dvh - 48px); grid-template-rows: auto minmax(0, 1fr) auto; }
  .head { display: flex; align-items: center; gap: 10px; min-width: 0; padding: 12px 12px 0 16px; }
  .head h2 { flex: 1; min-width: 0; margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--t-h3); line-height: 1.3; }
  .close { flex: none; }
  .perms-dialog .body { gap: 12px; padding: 12px 16px 14px; overflow-y: auto; overscroll-behavior: contain; }
  p { margin: 0; }

  .sets { display: grid; gap: 6px; padding-bottom: 12px; border-bottom: 1px solid var(--border); }
  .all { gap: 8px; min-height: 32px; padding-bottom: 10px; border-bottom: 1px solid var(--border); font-size: var(--t-small); }
  .all.on .name { font-weight: 600; }

  /* Two columns of groups; each group a label over one-line rows. */
  .groups { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px 24px; }
  .group { margin: 0; padding: 0; border: 0; min-width: 0; }
  .group legend { padding: 0; margin-bottom: 2px; }
  .perm { position: relative; font-size: var(--t-small); }
  .perm label { display: flex; align-items: center; gap: 8px; min-height: 32px; cursor: pointer; }
  .perm input { flex: none; width: 16px; height: 16px; margin: 0; accent-color: var(--accent); }
  .perm input:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  .perm .name { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .perm.on .name { font-weight: 600; }
  .perm.off label { cursor: default; }
  .perm.off .name { color: var(--dim); }
  .note { line-height: 1.35; }
  .warn { color: var(--yellow); }
  .status { display: grid; gap: 4px; }

  /* The tooltip sits above its row (so it never makes the dialog scroll): after the usual delay
     on hover, at once on keyboard focus. */
  .tip { position: absolute; left: 0; bottom: calc(100% + 2px); z-index: var(--z-tip); max-width: min(260px, 100%); padding: 6px 9px;
    border: 1px solid var(--border-strong); border-radius: var(--radius-btn); background: var(--elevated-2); color: var(--text);
    box-shadow: var(--shadow-pop); font-size: 12px; line-height: 1.4; font-weight: 400; white-space: normal;
    opacity: 0; transform: translateY(2px); pointer-events: none; transition: opacity var(--fast), transform var(--fast); }
  .perm:has(input:focus-visible) .tip { opacity: 1; transform: none; }
  .perm.hushed .tip { opacity: 0 !important; transition: none; }
  @media (hover: hover) and (pointer: fine) {
    .perm:hover .tip { opacity: 1; transform: none; transition-delay: var(--delay-tip, .4s); }
  }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .tip { transform: none; transition: opacity var(--fast); } }
  @media (pointer: coarse) { .perm label, .all { min-height: 44px; } }

  /* Phones: a full-height sheet; the body scrolls, the footer stays at the bottom. */
  @media (max-width: 560px) {
    .perms-dialog { width: 100vw; max-width: 100vw; height: 100dvh; max-height: 100dvh; margin: 0; inset: 0; border: 0; border-radius: 0; }
    .perms-dialog form { height: 100%; max-height: 100dvh; }
    .head { padding-top: max(12px, env(safe-area-inset-top)); }
    .perms-dialog footer { padding-bottom: max(12px, env(safe-area-inset-bottom)); }
    .groups { grid-template-columns: minmax(0, 1fr); }
  }
</style>
