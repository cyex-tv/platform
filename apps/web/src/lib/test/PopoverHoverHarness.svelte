<script lang="ts">
  // Test harness: a hover-opening menu Popover (the account menu's shape).
  import Popover from '$lib/components/Popover.svelte';
</script>

<Popover label="Account" kind="menu" hover>
  {#snippet trigger(props)}<button type="button" {...props}>Who</button>{/snippet}
  <button type="button" role="menuitem">Profile</button>
</Popover>
