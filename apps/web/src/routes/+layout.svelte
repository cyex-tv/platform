<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import CommandPalette from '$lib/components/CommandPalette.svelte';
  import Footer from '$lib/components/Footer.svelte';
  import Nav, { type NavUser } from '$lib/components/Nav.svelte';
  import { ME } from '$lib/queries';
  import { ADMIN_PERMISSIONS } from '$lib/tool/session.svelte';

  let { children } = $props();
  let me = $state<NavUser | null>(null);
  // The signed-in pages (`(tool)`) bring their own header in the cyex-app style, and a page whose
  // load returns `chrome: false` (the docs at /developers) brings its own header and footer.
  const tool = $derived(page.route.id?.startsWith('/(tool)') || page.data.chrome === false);

  onMount(async () => {
    // Only the browser has the session cookie; signed out is the default.
    try {
      const m = (await gql(ME, {})).me;
      const who = m?.identities[0];
      me = who
        ? { displayName: who.displayName, avatar: who.avatar, platform: who.platform, admin: m.permissions.some((p) => ADMIN_PERMISSIONS.includes(p)) }
        : null;
    } catch {
      me = null;
    }
  });
</script>

{#if tool}
  {@render children()}
{:else}
  <a class="skip" href="#main">Skip to content</a>
  <Nav {me} onsignout={() => (me = null)} />
  <main id="main" tabindex="-1">
    {@render children()}
  </main>
  <Footer />
{/if}
<CommandPalette />
