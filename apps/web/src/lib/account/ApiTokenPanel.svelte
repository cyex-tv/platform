<script lang="ts">
  // Your API token (ADR 0028, 0029): when the live one expires, and a button that makes a new one
  // and shows it here, with Copy. The API keeps only its hash, so a token is shown once, right
  // after it is made; it lasts 3 days and rotates whenever a session signs out, one or all.
  // Nothing here stores it: it lives in this component until the page goes.
  import { ApiError, gql } from '$lib/api';
  import { useCopied } from '$lib/clipboard.svelte';
  import { serverMessage } from '$lib/components/problem';
  import Row from '$lib/components/Row.svelte';
  import Section from '$lib/components/Section.svelte';
  import { MY_CONSOLE_TOKEN } from '$lib/queries';
  import { Gate } from '$lib/tool/gate.svelte';
  import { expiryText, makeToken } from './console-token';

  // A short rest after each: the API makes five an hour, a double click mustn't burn two.
  const gate = new Gate(1500);
  const copied = useCopied();
  let expiresAt = $state<string | null>(null);
  let token = $state('');
  let problem = $state('');

  $effect(() => {
    let live = true;
    gql(MY_CONSOLE_TOKEN, {})
      .then((d) => {
        if (live && !token) expiresAt = d?.myConsoleToken?.expiresAt ?? null;
      })
      .catch(() => {});
    return () => (live = false);
  });

  const make = () =>
    gate.run(async () => {
      problem = '';
      try {
        const made = await makeToken();
        token = made.token;
        expiresAt = made.expiresAt || null;
        copied.reset();
      } catch (err) {
        problem = err instanceof ApiError ? err.message : serverMessage();
      }
    });

  const expiry = $derived(expiryText(expiresAt));
</script>

<Section title="API token" id="token-h" lead="A personal Bearer token that acts as you in the API. It lasts 3 days and is replaced when you sign out of a session or everywhere.">
  <Row label="Current token" hint={expiry ? `Expires ${expiry}` : 'None'}>
    <button class="ui-btn secondary small" type="button" disabled={gate.busy} onclick={make}>{expiresAt ? 'New token' : 'Make token'}</button>
  </Row>
  {#if token}
    <div class="tok">
      <input class="ui-input ui-mono" type="text" readonly aria-label="API token" value={token} onfocus={(e) => e.currentTarget.select()} />
      <button class="ui-btn secondary small" type="button" onclick={() => copied.copy(token)}>{copied.copied ? 'Copied' : copied.failed ? 'Copy failed' : 'Copy'}</button>
    </div>
  {/if}
  {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
</Section>

<style>
  .tok { display: flex; gap: 8px; align-items: center; padding: 10px 0; border-bottom: 1px solid var(--border); min-width: 0; }
  .tok input { flex: 1 1 auto; min-width: 0; }
  .problem { margin: 8px 0 0; font-size: var(--t-small); color: var(--muted); }
</style>
