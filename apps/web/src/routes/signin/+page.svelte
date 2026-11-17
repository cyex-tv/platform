<script lang="ts">
  import { page } from '$app/state';
  import { signInUrl } from '$lib/api';

  const back = $derived(page.url.searchParams.get('back') ?? '/me');
</script>

<svelte:head>
  <title>Sign in - cyex.tv</title>
  <meta name="description" content="Sign in to cyex.tv with Twitch or Kick." />
</svelte:head>

<div class="page">
  <div class="form">
    <header class="page-head">
      <h1>Sign in</h1>
      <p class="lead">With the account you stream or chat with. You can link the other one afterwards.</p>
    </header>
    <div class="choices">
      <a class="btn choice" href={signInUrl('twitch', back)}>
        <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"
          ><path fill="#9146ff" d="M4.3 3 3 6.4v13.8h4.7V23h2.6l2.7-2.8h3.9L22 15V3H4.3Zm15.9 11.2-3 3h-4.7l-2.6 2.6v-2.6H5.9V4.8h14.3v9.4Zm-3-6.2v5.2h-1.8V8h1.8Zm-4.8 0v5.2h-1.8V8h1.8Z"
          /></svg
        >
        Continue with Twitch
      </a>
      <a class="btn choice" href={signInUrl('kick', back)}>
        <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"
          ><path fill="#53fc18" d="M3 3h6v4h2V5h2V3h8v6h-2v2h-2v2h2v2h2v6h-8v-2h-2v-2H9v4H3V3Z" /></svg
        >
        Continue with Kick
      </a>
    </div>
    <p class="fine note">
      cyex.tv keeps your user ID, login, display name and picture from the platform, nothing else. No email, no
      access to your account.
    </p>
  </div>
</div>

<style>
  .form { max-width: var(--measure-form); }
  .choices { display: grid; gap: 12px; max-width: 360px; }
  .choice { justify-content: flex-start; }
  .note { margin-top: 24px; max-width: 52ch; }
</style>
