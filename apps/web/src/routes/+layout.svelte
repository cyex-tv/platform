<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { gql } from '$lib/api';
  import Footer from '$lib/components/Footer.svelte';
  import Nav from '$lib/components/Nav.svelte';
  import { ME } from '$lib/queries';

  let { children } = $props();
  let signedIn = $state(false);

  onMount(async () => {
    // Only the browser has the session cookie; signed out is the default.
    try {
      signedIn = Boolean((await gql(ME, {})).me);
    } catch {
      signedIn = false;
    }
  });
</script>

<a class="skip" href="#main">Skip to content</a>
<Nav {signedIn} />
<main id="main" tabindex="-1">
  {@render children()}
</main>
<Footer />
