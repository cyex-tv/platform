<script lang="ts">
  // The consent page of app sign-in (ADR 0021). An app such as cyex deck sent the browser here
  // through /v1/auth/app/authorize. Signed out, you sign in with Twitch or Kick and come back to
  // this same address; signed in, the card names the app, what it may do and which account it
  // gets. Allow or Deny goes back to the app on this computer (its loopback address), which then
  // finishes on its own. Nothing here is remembered: every sign-in from the app asks again.
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { ApiError, signInUrl, signOut } from '$lib/api';
  import { type AppClient, type AppRequest, answer, appClient, listener, readRequest } from '$lib/app-auth';
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Mark from '$lib/components/Mark.svelte';
  import State from '$lib/components/State.svelte';
  import { clean } from '$lib/format';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import { forgetMe, loadMe, type Me } from '$lib/tool/session.svelte';

  let status = $state<'loading' | 'ready' | 'signin' | 'invalid' | 'failed' | 'done'>('loading');
  let request = $state<AppRequest | null>(null);
  let client = $state<AppClient | null>(null);
  let me = $state<Me | null>(null);
  let busy = $state<'allow' | 'deny' | 'switch' | null>(null);
  let problem = $state('');
  let allowed = $state(false);

  const here = $derived(`${page.url.pathname}${page.url.search}`);
  const who = $derived(me?.identities.find((i) => i.platform === me?.displayPlatform) ?? me?.identities[0] ?? null);
  const name = $derived(clean(who?.displayName ?? '') || clean(who?.login ?? '') || 'Your account');

  async function load() {
    status = 'loading';
    problem = '';
    request = readRequest(page.url.searchParams);
    if (!request) {
      status = 'invalid';
      return;
    }
    try {
      const [found, account] = await Promise.all([
        appClient(request.clientId),
        loadMe({ fresh: true }).catch((e) => {
          if (e instanceof ApiError && e.code === 'signed_out') return null;
          throw e;
        }),
      ]);
      if (!found) {
        status = 'invalid';
        return;
      }
      client = found;
      me = account;
      status = account ? 'ready' : 'signin';
    } catch {
      status = 'failed';
    }
  }

  onMount(load);

  async function decide(allow: boolean) {
    if (busy || !request) return;
    busy = allow ? 'allow' : 'deny';
    problem = '';
    let location: string;
    try {
      location = await answer(request, allow);
    } catch (err) {
      busy = null;
      if (err instanceof ApiError && err.code === 'signed_out') {
        me = null;
        status = 'signin';
        return;
      }
      if (err instanceof ApiError && (err.code === 'unknown_client' || err.code === 'bad_redirect_uri')) {
        status = 'invalid';
        return;
      }
      problem = err instanceof ApiError && err.code === 'offline' ? err.message : 'That didn’t go through. Try again in a moment.';
      return;
    }
    allowed = allow;
    status = 'done';
    window.location.assign(location);
  }

  async function switchAccount() {
    if (busy) return;
    busy = 'switch';
    try {
      await signOut();
    } catch {
      busy = null;
      problem = 'Signing out didn’t work. Try again in a moment.';
      return;
    }
    forgetMe();
    me = null;
    busy = null;
    status = 'signin';
  }
</script>

<svelte:head>
  <title>{client ? `Allow ${client.name}` : 'Allow an app'} - cyex.tv</title>
  <meta name="robots" content="noindex" />
</svelte:head>

<ColumnLayout top>
  <div class="center">
    {#if status === 'loading'}
      <div class="wide"><State kind="loading" title="Loading" /></div>
    {:else if status === 'failed'}
      <div class="wide">
        <State kind="error" title="This page didn’t load" level={1}>
          Nothing was allowed. Try again in a moment.
          {#snippet actions()}<button class="ui-btn secondary" type="button" onclick={load}>Try again</button>{/snippet}
        </State>
      </div>
    {:else if status === 'invalid' || !request || !client}
      <div class="wide">
        <State kind="error" title="This sign-in link doesn’t work" level={1}>
          It’s incomplete or not from an app cyex.tv knows. Start signing in again from the app.
          {#snippet actions()}<a class="ui-btn secondary" href="/">Go to the front page</a>{/snippet}
        </State>
      </div>
    {:else}
      <section class="card" aria-labelledby="consent-h">
        <div class="app">
          <span class="tile" aria-hidden="true"><Mark size={26} /></span>
          <div class="app-text">
            <h1 class="page-title" id="consent-h">
              {status === 'signin' ? `Sign in to use ${client.name}` : `Allow ${client.name}?`}
            </h1>
            <p class="sub">On this computer, at <span class="mono">{listener(request.redirectUri)}</span></p>
          </div>
        </div>

        <p class="lead"><bdi>{client.name}</bdi> wants to:</p>
        <ul class="scopes">
          {#each client.scopes as scope (scope.id)}
            <li><span class="tick" aria-hidden="true"><Icon name="check" /></span>{scope.description}</li>
          {/each}
        </ul>

        {#if status === 'signin'}
          <p class="lead">Sign in with the account you use on cyex.tv. You come back here afterwards.</p>
          <div class="choices">
            <a class="btn btn-s big" href={signInUrl('twitch', here)}>Continue with Twitch</a>
            <a class="btn btn-s big" href={signInUrl('kick', here)}>Continue with Kick</a>
          </div>
        {:else if status === 'done'}
          <p class="lead done" role="status">
            {allowed ? `Done. Go back to ${client.name}; it finishes signing in by itself.` : `Not allowed. ${client.name} wasn’t given access.`}
          </p>
          <p class="fine note">If nothing happens, close this tab and start again from the app.</p>
        {:else if me}
          <div class="account">
            <Avatar src={who?.avatar ?? null} {name} size={32} />
            <div class="account-text">
              <span class="fine">Signed in as</span>
              <strong><bdi>{name}</bdi></strong>
            </div>
            <button class="link" type="button" onclick={switchAccount} disabled={busy !== null}>
              {busy === 'switch' ? 'Signing out…' : 'Not you?'}
            </button>
          </div>
          {#if problem}<p class="ui-error" role="alert">{problem}</p>{/if}
          <div class="choices two">
            <button class="btn btn-s big" type="button" onclick={() => decide(false)} disabled={busy !== null}>
              {busy === 'deny' ? 'Denying…' : 'Deny'}
            </button>
            <button class="btn btn-s btn-primary big" type="button" onclick={() => decide(true)} disabled={busy !== null}>
              {busy === 'allow' ? 'Allowing…' : 'Allow'}
            </button>
          </div>
          <p class="fine note">
            It can’t upload, delete or change your account. Sign it out any time with
            <a href="/me/account">Your profile → Account → Sign out everywhere</a>.
          </p>
        {/if}
      </section>
    {/if}
  </div>
</ColumnLayout>

<style>
  /* The sign-in page's measure: one centred card. */
  .center { display: grid; justify-items: center; padding-block: clamp(24px, 6vw, 72px); }
  .wide { width: 100%; max-width: var(--measure-form); }
  .card { width: 100%; max-width: var(--measure-form); padding: clamp(20px, 4vw, 32px); border: 1px solid var(--border);
    border-radius: var(--radius-card); background: var(--surface); }
  .app { display: flex; align-items: center; gap: 14px; min-width: 0; }
  .app-text { min-width: 0; }
  .app h1 { margin: 0; text-wrap: balance; }
  .tile { flex: none; display: grid; place-items: center; width: 48px; height: 48px; border: 1px solid var(--border);
    border-radius: var(--radius-card); background: var(--bg); }
  .sub { margin: 2px 0 0; color: var(--muted); font-size: var(--t-small); }
  .mono { font-family: var(--font-mono); font-variant-numeric: tabular-nums; }
  .lead { margin: 20px 0 0; font-size: var(--t-ui); line-height: 1.55; color: var(--muted); max-width: none; text-wrap: pretty; }
  .lead bdi { color: var(--text); font-weight: 600; }
  .done { color: var(--text); }
  .scopes { list-style: none; margin: 10px 0 0; padding: 0; display: grid; gap: 8px; }
  .scopes li { display: flex; align-items: flex-start; gap: 10px; font-size: var(--t-ui); line-height: 1.45; color: var(--text); }
  .tick { flex: none; display: inline-grid; place-items: center; width: 20px; height: 20px; color: var(--accent-text); }
  .tick :global(svg) { width: 16px; height: 16px; }
  .account { display: flex; align-items: center; gap: 12px; margin-top: 24px; padding: 12px; border: 1px solid var(--border);
    border-radius: var(--radius-card); background: var(--bg); min-width: 0; }
  .account-text { display: grid; min-width: 0; flex: 1; }
  .account-text strong { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .account-text .fine { margin: 0; }
  .link { flex: none; border: 0; background: none; padding: 4px 2px; color: var(--accent-text); font: inherit; font-size: var(--t-ui); cursor: pointer;
    border-radius: var(--radius); }
  .link:hover { text-decoration: underline; text-underline-offset: 3px; }
  .link:disabled { color: var(--muted); cursor: default; text-decoration: none; }
  .ui-error { margin: 12px 0 0; }
  .choices { display: grid; gap: 10px; margin-top: 20px; }
  .choices.two { grid-template-columns: 1fr 1fr; }
  .big { width: 100%; height: 44px; justify-content: center; font-size: var(--t-ui); }
  .btn.big:not(.btn-primary) { background: var(--bg); }
  .note { margin: 16px 0 0; }
  .note a { color: var(--text-sub, var(--muted)); text-decoration: underline; text-underline-offset: 3px; }
  @media (max-width: 380px) { .choices.two { grid-template-columns: 1fr; } .choices.two .btn-primary { order: -1; } }
</style>
