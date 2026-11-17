<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import Footer from '$lib/components/Footer.svelte';
  import Nav from '$lib/components/Nav.svelte';
  import { ME } from '$lib/queries';

  let { children } = $props();
  let signedIn = $state(false);
  // The signed-in pages (`(tool)`) bring their own header in the cyex-app style.
  const tool = $derived(page.route.id?.startsWith('/(tool)') ?? false);

  onMount(async () => {
    // Only the browser has the session cookie; signed out is the default.
    try {
      signedIn = Boolean((await gql(ME, {})).me);
    } catch {
      signedIn = false;
    }
  });
</script>

{#if tool}
  {@render children()}
{:else}
  <a class="skip" href="#main">Skip to content</a>
  <Nav {signedIn} />
  <main id="main" tabindex="-1">
    {@render children()}
  </main>
  <Footer />
{/if}
