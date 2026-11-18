<script lang="ts">
  // A colour: the system picker (behind a swatch), the hex code and, with `alpha`, an opacity slider.
  // Reports `#RRGGBB` or `#RRGGBBAA` (upper case); a hex that isn't one shows an error and isn't sent.
  import { alphaOf, normHex, opaque, withAlpha } from '../format';

  type Props = { label: string; value: string; alpha?: boolean; error?: string | null; onchange: (v: string) => void; hideLabel?: boolean };
  let { label, value, alpha = true, error = null, onchange, hideLabel = false }: Props = $props();
  const uid = $props.id();
  let draft = $state<string | null>(null);
  const bad = $derived(draft != null && !normHex(draft));
  const a = $derived(alphaOf(value));

  function commit(t: string) {
    const h = normHex(t);
    if (!h) return;
    draft = null;
    onchange(alpha ? h : h.slice(0, 7));
  }
</script>

<div class="ps-field">
  <span class="ps-flabel" id="{uid}-l" class:sr-only={hideLabel}>{label}</span>
  <div class="ps-colour" role="group" aria-labelledby="{uid}-l">
    <span class="ps-swatch">
      <span style:background={value}></span>
      <input type="color" aria-label="{label}: pick a colour" value={opaque(value)} oninput={(e) => onchange(withAlpha(e.currentTarget.value, alpha ? a : 1))} />
    </span>
    <input
      class="ps-hex"
      type="text"
      spellcheck="false"
      autocomplete="off"
      aria-label="{label}: hex code"
      aria-invalid={bad || error ? 'true' : undefined}
      aria-describedby={bad || error ? `${uid}-e` : undefined}
      value={draft ?? value}
      oninput={(e) => {
        draft = e.currentTarget.value;
        if (normHex(draft) && /^#?([0-9a-f]{6}|[0-9a-f]{8})$/i.test(draft.trim())) commit(draft);
      }}
      onchange={(e) => commit(e.currentTarget.value)}
      onblur={() => {
        if (!bad) draft = null;
      }}
    />
    {#if alpha}
      <span class="ps-alpha">
        <input type="range" min="0" max="100" step="1" aria-label="{label}: opacity" value={Math.round(a * 100)} oninput={(e) => onchange(withAlpha(value, Number(e.currentTarget.value) / 100))} />
        <output>{Math.round(a * 100)}%</output>
      </span>
    {/if}
  </div>
  {#if bad}<p class="ps-msg" id="{uid}-e">Use a hex colour like #FF7A45 or #FF7A4580.</p>{:else if error}<p class="ps-msg" id="{uid}-e">{error}</p>{/if}
</div>
