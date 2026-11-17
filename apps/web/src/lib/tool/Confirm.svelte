<script lang="ts">
  // A destructive button that arms first: the first click turns it red with "Click again to …",
  // the second within three seconds does it (cyex-app's menu rule, as a button).
  let {
    label,
    confirm,
    busy = false,
    small = false,
    onconfirm,
  }: { label: string; confirm: string; busy?: boolean; small?: boolean; onconfirm: () => void } = $props();
  let armed = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  function click() {
    if (armed) {
      clearTimeout(timer);
      armed = false;
      onconfirm();
      return;
    }
    armed = true;
    timer = setTimeout(() => (armed = false), 3000);
  }
</script>

<button class="ui-btn danger" class:armed class:small type="button" disabled={busy} onclick={click} onblur={() => (armed = false)}>
  {armed ? confirm : label}
</button>
<span class="sr" role="status" aria-live="polite">{armed ? confirm : ''}</span>

<style>
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
</style>
