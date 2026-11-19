<script lang="ts">
  // The permissions dialog for an account editor (ADR 0014): grouped permissions, "Everything",
  // and which sets they may edit (all, new ones included, or a few). Adding someone and changing an
  // editor open the same dialog; Save applies at once (no invitation step).
  //   <EditorDialog bind:this={d} held={held} mine={scope} sets={sets} onsave={...} />
  //   d.open(account, { permissions, scope }, 'add' | 'edit')
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { clean } from '$lib/format';
  import type { EditorPermission } from '$lib/gql/graphql';
  import {
    ALL_SETS,
    GROUPS,
    allOn,
    covers,
    grantable,
    locked,
    scopeLabel,
    setAll,
    toggle,
    toggleSet,
    type Scope,
  } from './editors';

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
    /** The account's sets, for the "Sets they can edit" list. */
    sets: readonly SetOption[];
    onsave: (who: Who, permissions: EditorPermission[], scope: Scope) => Promise<void>;
    /** A failed save as a sentence shown in the dialog. */
    describe: (e: unknown) => string;
  } = $props();

  let dialog: HTMLDialogElement;
  let setsButton = $state<HTMLButtonElement>();
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
  // A manager limited to some sets only offers those, and never "All sets".
  const offeredSets = $derived(mine.all ? sets : sets.filter((s) => covers(mine, s.id)));

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

  // Esc closes the sets list first, then the dialog. Tab wraps around inside the dialog.
  function keydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && setsOpen) {
      e.preventDefault();
      e.stopPropagation();
      setsOpen = false;
      setsButton?.focus();
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

  function cancel(e: Event) {
    if (setsOpen) {
      e.preventDefault();
      setsOpen = false;
      setsButton?.focus();
    }
  }
</script>

<dialog class="ui-dialog perms-dialog" bind:this={dialog} aria-labelledby="ed-title" oncancel={cancel} onkeydown={keydown} onclose={() => { who = null; setsOpen = false; }}>
  <form onsubmit={submit} novalidate>
    <div class="body">
      <div class="title">
        {#if who}<Avatar src={who.avatar} name={who.displayName} size={32} tone="tool" />{/if}
        <h2 id="ed-title">{#if who}What <bdi>{clean(who.displayName)}</bdi> can do{/if}</h2>
        <button class="ui-icon close" type="button" aria-label="Close" onclick={() => dialog.close()}><Icon name="x" size={18} /></button>
      </div>

      <div class="ui-field sets">
        <span class="ui-label" id="ed-sets-label">Sets they can edit</span>
        <button class="ui-input pick" type="button" bind:this={setsButton} aria-expanded={setsOpen} aria-controls="ed-sets"
          aria-labelledby="ed-sets-label ed-sets-value" onclick={() => (setsOpen = !setsOpen)}>
          <span id="ed-sets-value">{scope.all ? 'All sets' : scope.ids.length ? scopeLabel(scope, sets) : 'Pick sets'}</span>
          <Icon name="chevron-down" size={16} />
        </button>
        {#if setsOpen}
          <div class="list" id="ed-sets" role="group" aria-labelledby="ed-sets-label">
            {#if mine.all}
              <label class="ui-check row">
                <input type="checkbox" checked={scope.all} onchange={(e) => (scope = e.currentTarget.checked ? ALL_SETS : { all: false, ids: [] })} />
                <span class="text"><span class="name">All sets</span><span class="ui-hint">New sets included.</span></span>
              </label>
            {/if}
            {#each offeredSets as s (s.id)}
              <label class="ui-check row">
                <input type="checkbox" checked={!scope.all && scope.ids.includes(s.id)}
                  onchange={(e) => {
                    const next = toggleSet(scope, s.id, e.currentTarget.checked);
                    scope = !mine.all && next.all ? { all: false, ids: [] } : next;
                  }} />
                <span class="text"><bdi class="name">{clean(s.name)}</bdi>{#if s.kind === 'PERSONAL'}<span class="ui-hint">Personal set</span>{/if}</span>
              </label>
            {:else}
              <p class="ui-hint empty">No sets yet.</p>
            {/each}
          </div>
        {/if}
        <span class="ui-hint">{scope.all ? 'Every set, and any set made later.' : 'Only these sets. Editors limited to some sets can’t make new ones.'}</span>
      </div>


      <label class="ui-check all">
        <input type="checkbox" checked={everything} indeterminate={some} onchange={(e) => (picked = setAll(e.currentTarget.checked, offer))} />
        <strong>Everything</strong>
        <span class="ui-hint">{held.size < GROUPS.flatMap((g) => g.items).length ? 'that you can grant' : 'on this account'}</span>
      </label>

      {#each GROUPS as g (g.label)}
        <fieldset class="group">
          <legend class="ui-label">{g.label}</legend>
          <div class="items">
            {#each g.items as p (p.key)}
              {@const can = held.has(p.key)}
              {@const lock = locked(picked, p.key)}
              <label class="perm" class:off={!can}>
                <input type="checkbox" checked={picked.includes(p.key)} disabled={!can || lock}
                  onchange={(e) => (picked = toggle(picked, p.key, e.currentTarget.checked))} aria-describedby="ed-{p.key}" />
                <span class="text">
                  <span class="name">{p.label}</span>
                  <span class="ui-hint" id="ed-{p.key}">{!can ? 'You don’t hold this one.' : lock ? 'Comes with managing sets.' : p.line}</span>
                </span>
              </label>
            {/each}
          </div>
          {#if g.note}<p class="ui-hint note">{g.note}</p>{/if}
        </fieldset>
      {/each}

      {#if !picked.length}<p class="ui-hint">Pick at least one{mode === 'edit' ? ', or remove them instead' : ''}.</p>{/if}
      {#if !scopeOk}<p class="ui-hint">Pick at least one set.</p>{/if}
      {#if problem}<p class="ui-error" role="alert">{problem}</p>{/if}
    </div>
    <footer>
      <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
      <button class="ui-btn primary" type="submit" disabled={busy || !picked.length || !scopeOk}>
        {busy ? 'Saving…' : mode === 'add' ? 'Add editor' : 'Save'}
      </button>
    </footer>
  </form>
</dialog>

<style>
  .perms-dialog { width: min(640px, calc(100vw - 24px)); }
  .perms-dialog form { max-height: calc(100dvh - 48px); grid-template-rows: minmax(0, 1fr) auto; }
  .perms-dialog .body { overflow-y: auto; overscroll-behavior: contain; gap: 16px; }
  .title { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .title h2 { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .close { flex: none; margin: -8px -8px -8px 0; }
  .all { gap: 10px; padding: 4px 0 12px; border-bottom: 1px solid var(--border); }
  .sets { padding-bottom: 14px; border-bottom: 1px solid var(--border); }
  .all strong { font-weight: 600; }
  .group { margin: 0; padding: 0; border: 0; min-width: 0; }
  .group legend { padding: 0; margin-bottom: 4px; }
  .items { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0 16px; }
  .perm { display: flex; align-items: flex-start; gap: 10px; min-height: 40px; padding: 5px 0; cursor: pointer; }
  .perm.off { cursor: default; }
  .perm.off .name { color: var(--dim); }
  .perm input, .row input { flex: none; width: 16px; height: 16px; margin: 3px 0 0; accent-color: var(--accent); }
  .text { display: grid; gap: 1px; min-width: 0; }
  .name { font-weight: 600; overflow-wrap: anywhere; }
  .note { margin-top: 2px; }
  .pick { display: flex; align-items: center; justify-content: space-between; gap: 8px; text-align: left; cursor: pointer; }
  .pick span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .list { display: grid; max-height: 220px; overflow-y: auto; overscroll-behavior: contain; padding: 4px 12px;
    border: 1px solid var(--border); border-radius: var(--radius-control, 3px); background: var(--surface); }
  .row { align-items: flex-start; gap: 10px; min-height: 40px; padding: 8px 0; border-bottom: 1px solid var(--border); }
  .row:last-child { border-bottom: 0; }
  .empty { padding: 10px 0; }
  @media (max-width: 560px) {
    .items { grid-template-columns: minmax(0, 1fr); }
  }
</style>
