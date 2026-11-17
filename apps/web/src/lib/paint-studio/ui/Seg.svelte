<script lang="ts" generics="T extends string">
  // A radio group drawn as tabs: the selected option is bold with an accent bar under it.
  // One tab stop; arrows, Home and End move and select.
  type Props = { label: string; options: { value: T; label: string }[]; value: T; onchange: (v: T) => void; big?: boolean; id?: string };
  let { label, options, value, onchange, big = false, id }: Props = $props();
  let root: HTMLDivElement;

  function key(e: KeyboardEvent, i: number) {
    const n = options.length;
    const to = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? (i + 1) % n : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? (i - 1 + n) % n : e.key === 'Home' ? 0 : e.key === 'End' ? n - 1 : -1;
    if (to < 0) return;
    e.preventDefault();
    onchange(options[to].value);
    root.querySelectorAll<HTMLButtonElement>('button')[to]?.focus();
  }
</script>

<div class="ps-seg" class:big role="radiogroup" aria-label={label} bind:this={root} {id}>
  {#each options as o, i (o.value)}
    <button type="button" role="radio" aria-checked={o.value === value} tabindex={o.value === value ? 0 : -1} onclick={() => onchange(o.value)} onkeydown={(e) => key(e, i)}>{o.label}</button>
  {/each}
</div>
