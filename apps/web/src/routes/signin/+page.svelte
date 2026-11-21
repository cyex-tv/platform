<script lang="ts">
  import { page } from '$app/state';
  import { signInUrl } from '$lib/api';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';

  const back = $derived(page.url.searchParams.get('back') ?? '/me');
</script>

<svelte:head>
  <title>Sign in - cyex.tv</title>
  <meta name="description" content="Sign in to cyex.tv with Twitch or Kick." />
</svelte:head>

<ColumnLayout top>
  <div class="center">
    <section class="card" aria-labelledby="signin-h">
      <h1 class="page-title" id="signin-h">Sign in</h1>
      <p class="lead">With the account you stream or chat with. You can link the other one afterwards.</p>
      <div class="choices">
        <a class="btn btn-s choice" href={signInUrl('twitch', back)}>
          <span class="mark twitch" aria-hidden="true"
            ><svg viewBox="0 0 24 24" width="16" height="16"
              ><path d="M4.3 3 3 6.4v13.8h4.7V23h2.6l2.7-2.8h3.9L22 15V3H4.3Zm15.9 11.2-3 3h-4.7l-2.6 2.6v-2.6H5.9V4.8h14.3v9.4Zm-3-6.2v5.2h-1.8V8h1.8Zm-4.8 0v5.2h-1.8V8h1.8Z" /></svg
            ></span
          >
          Continue with Twitch
        </a>
        <a class="btn btn-s choice" href={signInUrl('kick', back)}>
          <span class="mark kick" aria-hidden="true"
            ><svg viewBox="0 0 24 24" width="16" height="16"><path d="M3 3h6v4h2V5h2V3h8v6h-2v2h-2v2h2v2h2v6h-8v-2h-2v-2H9v4H3V3Z" /></svg></span
          >
          Continue with Kick
        </a>
      </div>
      <h2 class="label">What cyex.tv keeps</h2>
      <dl class="kv">
        <dt>From the platform</dt><dd>User ID, login, display name, picture</dd>
        <dt>Never gets</dt><dd>Your password, email or access to your account</dd>
      </dl>
    </section>
  </div>
</ColumnLayout>

<style>
  /* One centred panel at the form measure; the platform colours only on the small marks. */
  .center { display: grid; justify-items: center; padding-block: clamp(24px, 6vw, 72px); }
  .card { width: 100%; max-width: var(--measure-form); padding: clamp(20px, 4vw, 32px); border: 1px solid var(--border);
    border-radius: var(--radius-card); background: var(--surface); }
  .lead { margin: 8px 0 0; font-size: var(--t-ui); line-height: 1.55; color: var(--muted); max-width: none; }
  .choices { display: grid; gap: 10px; margin-top: 24px; }
  .choice { width: 100%; height: 48px; justify-content: center; font-size: var(--t-ui); background: var(--bg); }
  .mark { display: inline-grid; place-items: center; width: 24px; height: 24px; border-radius: var(--radius); }
  .mark svg { display: block; }
  .twitch { color: var(--twitch); }
  .kick { color: var(--kick); background: #0b0e0f; }
  .mark path { fill: currentColor; }
  .label { display: flex; align-items: center; min-height: 36px; margin: 20px 0 0; font-size: var(--t-label, 11px); font-weight: 600; letter-spacing: .06em;
    text-transform: uppercase; color: var(--dim); }
  .kv { display: grid; grid-template-columns: auto minmax(0, 1fr); margin: 0; border-top: 1px solid var(--border); }
  .kv dt, .kv dd { margin: 0; padding: 9px 0; border-bottom: 1px solid var(--border); font-size: var(--t-small); }
  .kv dt { color: var(--muted); padding-right: 16px; white-space: nowrap; }
  .kv dd { text-align: right; font-weight: 600; }
  .kv dt:last-of-type, .kv dd:last-of-type { border-bottom: 0; }
  @media (max-width: 420px) {
    .kv { grid-template-columns: minmax(0, 1fr); }
    .kv dt { padding-bottom: 0; border-bottom: 0; }
    .kv dd { text-align: left; padding-top: 2px; }
  }
  @media (forced-colors: active) { .mark { forced-color-adjust: auto; } }
</style>
