<script lang="ts">
  // One label/value row of a flat form or list (redesign round 2): the label (and a hint under
  // it) on the left, the value or control on the right, a hairline under the row. Rows stack in a
  // Section. Under 560px the control drops under the label (unless `inline`). `icon` or `media`
  // leads the label (a platform icon, an avatar).
  //   <Row label="Twitch" hint="lumen_fox" icon="tv"><button class="ui-btn secondary small">Disconnect</button></Row>
  import type { Snippet } from 'svelte';
  import Icon, { type IconName } from './Icon.svelte';

  type Props = {
    label: string;
    /** A line under the label. */
    hint?: string | Snippet;
    /** The id of the control the label names (renders a <label for>). */
    for?: string;
    icon?: IconName;
    /** Something before the label instead of an icon (an avatar, a picture). */
    media?: Snippet;
    /** Keep the control beside the label on phones too (a switch, a short value). */
    inline?: boolean;
    /** A muted label (a row that is off or not there yet). */
    dim?: boolean;
    /** The value or control at the right. */
    children?: Snippet;
  };
  let { label, hint, for: forId, icon, media, inline = false, dim = false, children }: Props = $props();
</script>

<div class="row" class:inline class:has-lead={!!(icon || media)}>
  {#if media}<span class="lead">{@render media()}</span>{:else if icon}<span class="lead ico"><Icon name={icon} size={16} /></span>{/if}
  <div class="txt">
    {#if forId}<label class="lbl" class:dim for={forId}>{label}</label>{:else}<span class="lbl" class:dim>{label}</span>{/if}
    {#if hint}<span class="hint">{#if typeof hint === 'string'}{hint}{:else}{@render hint()}{/if}</span>{/if}
  </div>
  {#if children}<div class="val">{@render children()}</div>{/if}
</div>

<style>
  .row { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: center; column-gap: 16px; row-gap: 8px; min-height: 52px;
    padding: 10px 0; border-bottom: 1px solid var(--border); }
  .row.has-lead { grid-template-columns: 24px minmax(0, 1fr) auto; }
  .lead { display: inline-flex; align-items: center; justify-content: center; }
  .ico { color: var(--dim); }
  .txt { display: grid; gap: 2px; min-width: 0; }
  .lbl { font-size: var(--t-ui); font-weight: 500; color: var(--text); overflow-wrap: anywhere; }
  .lbl.dim { color: var(--muted); }
  /* The row spans its column (the site is full width); the hint keeps a reading measure. */
  .hint { max-width: 72ch; font-size: var(--t-small); line-height: 1.45; color: var(--muted); text-wrap: pretty; overflow-wrap: anywhere; }
  .val { display: flex; flex-wrap: wrap; align-items: center; justify-content: flex-end; gap: 8px; min-width: 0; text-align: right; }
  @media (max-width: 559.98px) {
    .row:not(.inline) { grid-template-columns: minmax(0, 1fr); }
    .row.has-lead:not(.inline) { grid-template-columns: 24px minmax(0, 1fr); }
    .row:not(.inline) .val { grid-column: 1 / -1; justify-content: flex-start; text-align: left; }
  }
</style>
