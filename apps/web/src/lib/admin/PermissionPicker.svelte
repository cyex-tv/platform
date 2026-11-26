<script lang="ts">
  // The role editor's permissions: grouped, and each one is Allow, Inherit or Deny for this role.
  // Inherit (the default) leaves it to the roles below; Allow and Deny decide it for everyone who
  // holds the role, and a Deny beats an Allow from any role under it. Each row is a radio group
  // (arrows move the choice); a group's own buttons set the whole group. A tooltip says what the
  // permission lets someone do (hover, and keyboard focus), read out as the row's description.
  // What you can't grant (you don't hold it) can't be changed; the User role can't deny
  // (it ranks last, so a deny there would never win).
  import Icon from '$lib/components/Icon.svelte';
  import { groupsFor, type PermissionGroup } from './permissions';
  import { EFFECTS, effectOf, groupCounts, setEffect, setGroup, type Effect } from './role-effects';

  let {
    available,
    allows = $bindable(),
    denies = $bindable(),
    canChange,
    noDeny = false,
    disabled = false,
    idPrefix = 'perm',
  }: {
    /** The API's permission names. */
    available: string[];
    allows: string[];
    denies: string[];
    /** Whether this permission may be changed by you. */
    canChange: (key: string) => boolean;
    /** Deny isn't offered (the User role). */
    noDeny?: boolean;
    disabled?: boolean;
    idPrefix?: string;
  } = $props();

  const groups = $derived(groupsFor(available));
  const sets = $derived({ allows, denies });
  const ICON = { allow: 'check', inherit: 'minus', deny: 'ban' } as const;

  const may = (key: string) => !disabled && canChange(key);
  const offered = (e: Effect) => !(noDeny && e === 'deny');
  const choices = $derived(EFFECTS.filter((x) => offered(x.id)));

  function choose(key: string, effect: Effect) {
    if (!may(key) || !offered(effect)) return;
    const next = setEffect(sets, key, effect);
    allows = next.allows;
    denies = next.denies;
  }
  function all(g: PermissionGroup, effect: Effect) {
    const next = setGroup(sets, g, effect, may);
    allows = next.allows;
    denies = next.denies;
  }

  // Roving radios: the chosen one is the tab stop; arrows choose and move focus.
  function onradiokey(e: KeyboardEvent, key: string) {
    const step = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 0;
    if (!step) return;
    e.preventDefault();
    const list = choices.map((x) => x.id);
    const at = list.indexOf(effectOf(sets, key));
    const next = list[(at + step + list.length) % list.length];
    choose(key, next);
    const row = (e.currentTarget as HTMLElement).closest('[role="radiogroup"]');
    row?.querySelector<HTMLElement>(`[data-effect="${next}"]`)?.focus();
  }

  // Escape puts the tooltip away (WCAG 1.4.13) without closing the dialog around it: the first
  // Escape hides it, the tooltip comes back for the next row you reach, a second Escape closes the dialog.
  let hushed = $state<string | null>(null);
  let shown = $state<string | null>(null); // the row whose tooltip is up (hover or focus)
  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && shown && hushed !== shown) {
      e.stopPropagation();
      e.preventDefault();
      hushed = shown;
    }
  }
</script>

<div class="groups" role="presentation" {onkeydown}>
  {#each groups as g (g.id)}
    {@const n = groupCounts(sets, g)}
    {@const locked = g.items.every((i) => !may(i.key))}
    <section class="group" aria-label={g.label}>
      <div class="head">
        <span class="name">{g.label}</span>
        <span class="n ui-num" aria-hidden="true">{n.allow + n.deny}/{n.of}</span>
        <span class="all" role="group" aria-label="Set all {g.label} permissions">
          {#each choices as x (x.id)}
            <button type="button" class="allbtn {x.id}" disabled={locked} aria-label="{x.label} all in {g.label}" onclick={() => all(g, x.id)}>{x.label} all</button>
          {/each}
        </span>
      </div>
      <ul>
        {#each g.items as i (i.key)}
          {@const cur = effectOf(sets, i.key)}
          {@const can = may(i.key)}
          <li class="item" class:off={!can} class:hushed={hushed === i.key}
            onfocusin={() => ((shown = i.key), hushed !== i.key && (hushed = null))} onfocusout={() => (shown = hushed = null)}
            onpointerenter={() => ((shown = i.key), hushed !== i.key && (hushed = null))} onpointerleave={() => (shown = hushed = null)}>
            <span class="lbl" id="{idPrefix}-{i.key}-l">{i.label}</span>
            <span class="seg" role="radiogroup" aria-labelledby="{idPrefix}-{i.key}-l" aria-describedby="{idPrefix}-{i.key}">
              {#each choices as x (x.id)}
                <button type="button" role="radio" class="opt {x.id}" data-effect={x.id} aria-checked={cur === x.id}
                  tabindex={cur === x.id ? 0 : -1} disabled={!can} onclick={() => choose(i.key, x.id)} onkeydown={(e) => onradiokey(e, i.key)}>
                  <Icon name={ICON[x.id]} size={13} />{x.label}
                </button>
              {/each}
            </span>
            <span class="tip" role="tooltip" id="{idPrefix}-{i.key}">
              {i.hint}{#if !can && !disabled}<span class="why"> You can only change what you hold.</span>{/if}
              <span class="key ui-mono">{i.key}</span>
            </span>
          </li>
        {/each}
      </ul>
    </section>
  {/each}
</div>

<style>
  /* Flat groups: an uppercase head over rows, hairlines between, no boxes. */
  .groups { display: grid; grid-template-columns: repeat(auto-fill, minmax(380px, 1fr)); gap: 0 28px; }
  .group { margin: 0; padding: 0 0 8px; min-width: 0; }
  .head { display: flex; flex-wrap: wrap; align-items: center; gap: 2px 8px; min-height: 36px; border-bottom: 1px solid var(--border); margin-bottom: 2px; }
  .head .name { font-size: var(--t-label); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .n { font-size: var(--t-fine); color: var(--dim); font-family: var(--font-mono); }
  .all { margin-left: auto; display: inline-flex; gap: 2px; }
  .allbtn { height: 28px; padding: 0 6px; border: 0; border-radius: var(--radius-btn); background: none; color: var(--muted); font: inherit; font-size: var(--t-fine); font-weight: 500; cursor: pointer; }
  .allbtn.deny { color: var(--red); }
  .allbtn:disabled { color: var(--dim); cursor: not-allowed; }
  ul { list-style: none; margin: 0; padding: 0; }
  .item { position: relative; display: flex; align-items: center; gap: 8px; min-height: 36px; border-bottom: 1px solid var(--border); font-size: var(--t-small); }
  .lbl { flex: 1; min-width: 0; }
  .item.off .lbl { color: var(--dim); }

  /* Three radios, no box: the chosen one is bold with a coloured icon; Deny uses the danger tone. */
  .seg { display: inline-flex; flex: none; }
  .opt { display: inline-flex; align-items: center; justify-content: center; gap: 5px; min-width: 68px; height: 32px; padding: 0 6px;
    border: 0; border-radius: var(--radius-btn); background: none; color: var(--muted); font: inherit; font-size: var(--t-fine); font-weight: 500; cursor: pointer; }
  .opt :global(svg) { color: var(--dim); }
  .opt[aria-checked='true'] { color: var(--text); font-weight: 700; }
  .opt[aria-checked='true'] :global(svg) { color: var(--accent-text); }
  .opt.deny[aria-checked='true'], .opt.deny[aria-checked='true'] :global(svg) { color: var(--red); }
  .opt:focus-visible, .allbtn:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .opt:disabled, .allbtn:disabled { cursor: not-allowed; }
  .item.off .opt[aria-checked='true'] { color: var(--muted); }
  @media (pointer: coarse) { .item { min-height: 44px; } .opt { height: 44px; } .allbtn { height: 44px; } }
  @media (max-width: 639.98px) {
    .groups { grid-template-columns: minmax(0, 1fr); }
    .item { flex-wrap: wrap; row-gap: 0; padding-top: 6px; }
    .lbl { flex-basis: 100%; }
    .seg { width: 100%; }
    .opt { flex: 1; }
  }

  /* The tooltip: below the row, after the usual delay on hover; at once on keyboard focus. */
  .tip { position: absolute; left: 0; right: 0; top: calc(100% + 2px); z-index: var(--z-tip); display: grid; gap: 4px;
    padding: 7px 9px; border-radius: var(--radius-btn); background: var(--elevated, var(--surface)); color: var(--text);
    border: 1px solid var(--border-strong); box-shadow: var(--shadow-pop); font-size: 12px; line-height: 1.45; font-weight: 400;
    opacity: 0; transform: translateY(-2px); pointer-events: none; transition: opacity var(--fast), transform var(--fast); }
  .item.hushed .tip { opacity: 0 !important; transition: none; }
  .why { color: var(--muted); }
  .key { color: var(--dim); font-size: 11px; }
  .item:has(:focus-visible) .tip { opacity: 1; transform: none; }
  @media (hover: hover) and (pointer: fine) {
    .opt:hover:not(:disabled), .allbtn:hover:not(:disabled) { color: var(--text); }
    .opt.deny:hover:not(:disabled), .allbtn.deny:hover:not(:disabled) { color: var(--red); }
    .item:hover .tip { opacity: 1; transform: none; transition-delay: var(--delay-tip, .4s); }
  }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .tip { transform: none; transition: opacity var(--fast); } }
</style>
