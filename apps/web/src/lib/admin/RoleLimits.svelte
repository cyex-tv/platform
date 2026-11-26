<script lang="ts">
  // The role editor's Limits: every per-account limit a role can change, grouped (Requests,
  // Uploads, File sizes, Account actions). Each box shows the default as its placeholder; empty keeps it, a
  // number sets it for everyone who holds the role. A tooltip on hover and keyboard focus says
  // what the limit counts (read out as the box's description), as the permissions do.
  import type { RoleLimitFieldsFragment } from '$lib/gql/graphql';
  import { amount, grouped, info, numberText, per, valueProblem } from './role-limits';

  let {
    limits,
    values = $bindable(),
    disabled = false,
    tried = false,
    idPrefix = 'limit',
  }: {
    limits: readonly RoleLimitFieldsFragment[];
    /** The boxes' text by limit key; empty keeps the default. */
    values: Record<string, string>;
    disabled?: boolean;
    /** Show every problem, not only in boxes that were changed. */
    tried?: boolean;
    idPrefix?: string;
  } = $props();

  const groups = $derived(grouped(limits));
  let touched = $state<Record<string, boolean>>({});

  // Escape puts the tooltip away without closing the dialog around it (WCAG 1.4.13).
  let hushed = $state<string | null>(null);
  let shown = $state<string | null>(null);
  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && shown && hushed !== shown) {
      e.stopPropagation();
      e.preventDefault();
      hushed = shown;
    }
  }
</script>

<div class="groups" role="presentation" {onkeydown}>
  {#each groups as g (g.group)}
    <fieldset class="group">
      <legend class="sr-only">{g.label}</legend>
      <div class="head" aria-hidden="true">{g.label}</div>
      <ul>
        {#each g.items as l (l.key)}
          {@const what = info(l.key)}
          {@const problem = tried || touched[l.key] ? valueProblem(values[l.key], l) : null}
          {@const unit = what.unit ?? per(l.windowSeconds) ?? 'at once'}
          <li class="item" class:hushed={hushed === l.key} class:set={!!values[l.key]?.trim()}
            onfocusin={() => ((shown = l.key), hushed !== l.key && (hushed = null))} onfocusout={() => (shown = hushed = null)}
            onpointerenter={() => ((shown = l.key), hushed !== l.key && (hushed = null))} onpointerleave={() => (shown = hushed = null)}>
            <div class="row">
              <label class="lbl" for="{idPrefix}-{l.key}">{what.label}</label>
              <span class="box">
                <input class="ui-input ui-mono num" id="{idPrefix}-{l.key}" inputmode="numeric" autocomplete="off" maxlength="7"
                  placeholder={numberText(l.key, l.default)} bind:value={values[l.key]} {disabled}
                  oninput={() => (touched[l.key] = true)}
                  aria-invalid={problem ? 'true' : undefined}
                  aria-describedby="{idPrefix}-{l.key}-tip{problem ? ` ${idPrefix}-${l.key}-err` : ''}" />
                <span class="unit" aria-hidden="true">{unit}</span>
              </span>
            </div>
            {#if problem}<span class="ui-error" id="{idPrefix}-{l.key}-err">{problem}</span>{/if}
            <span class="tip" role="tooltip" id="{idPrefix}-{l.key}-tip">
              {what.hint} Default: {amount(l, l.default)}. Empty keeps it; {info(l.key).unit === 'MB' ? '0,01' : '1'} to {numberText(l.key, l.max)} replaces it.
            </span>
          </li>
        {/each}
      </ul>
    </fieldset>
  {/each}
</div>

<style>
  /* Flat groups: an uppercase head, then label/value rows with hairlines (the .adm-lv shape). */
  .groups { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, 320px), 1fr)); gap: 0 28px; align-items: start; }
  .group { margin: 0; padding: 0 0 8px; border: 0; background: none; min-width: 0; }
  .head { display: flex; align-items: center; min-height: 36px; padding: 0; margin-bottom: 2px;
    border-bottom: 1px solid var(--border); font-size: var(--t-label); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  ul { list-style: none; margin: 0; padding: 0; }
  .item { position: relative; display: grid; gap: 2px; padding: 2px 0; border-bottom: 1px solid var(--border); }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 40px; font-size: var(--t-small); }
  .lbl { cursor: pointer; }
  .lbl { min-width: 0; }
  .item.set .lbl { font-weight: 600; }
  .box { display: inline-flex; align-items: center; gap: 6px; flex: none; }
  .num { width: 84px; height: 30px; min-height: 30px; padding: 0 8px; text-align: right; }
  .unit { width: 64px; font-size: var(--t-fine); color: var(--dim); }
  .ui-error { font-size: 12px; text-align: right; }
  @media (pointer: coarse) { .row { min-height: 44px; } .num { height: 40px; } }

  /* The tooltip: below the row, after the usual delay on hover; at once on keyboard focus. */
  .tip { position: absolute; left: 0; right: 0; top: calc(100% + 2px); z-index: var(--z-tip); display: block;
    padding: 7px 9px; border-radius: var(--radius-btn); background: var(--elevated, var(--surface)); color: var(--text);
    border: 1px solid var(--border-strong); box-shadow: var(--shadow-pop); font-size: 12px; line-height: 1.45; font-weight: 400;
    opacity: 0; transform: translateY(-2px); pointer-events: none; transition: opacity var(--fast), transform var(--fast); }
  .item.hushed .tip { opacity: 0 !important; transition: none; }
  .item:has(input:focus-visible) .tip { opacity: 1; transform: none; }
  @media (hover: hover) and (pointer: fine) {
    .item:hover .tip { opacity: 1; transform: none; transition-delay: var(--delay-tip, .4s); }
  }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .tip { transform: none; transition: opacity var(--fast); } }
</style>
