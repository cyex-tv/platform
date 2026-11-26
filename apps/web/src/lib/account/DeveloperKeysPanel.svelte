<script lang="ts">
  // Developers → API keys (ADR 0033): read-only keys for bots and tools. Each row shows the key's
  // name, prefix, when it was made and last used, its requests over the last 30 days as small
  // bars (one per UTC day), and Revoke. A new key is shown once, right after it is made, with
  // Copy; the API keeps only its hash. Keys are not ended by signing out.
  import { ApiError, gql } from '$lib/api';
  import { useCopied } from '$lib/clipboard.svelte';
  import { serverMessage } from '$lib/components/problem';
  import Row from '$lib/components/Row.svelte';
  import Section from '$lib/components/Section.svelte';
  import { count } from '$lib/format';
  import type { MyApiKeysQuery } from '$lib/gql/graphql';
  import { CREATE_API_KEY, MY_API_KEYS, REVOKE_API_KEY } from '$lib/queries';
  import Confirm from '$lib/tool/Confirm.svelte';
  import { Gate } from '$lib/tool/gate.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { keyHint, MAX_KEYS, usageBars, usageLine } from './developers';

  type Key = MyApiKeysQuery['myApiKeys'][number];
  const gate = new Gate(800);
  const copied = useCopied();
  let keys = $state<Key[] | null>(null);
  let name = $state('');
  let made = $state('');
  let problem = $state('');
  let revoking = $state<string | null>(null);

  $effect(() => {
    let live = true;
    gql(MY_API_KEYS, {})
      .then((d) => {
        if (live) keys = d.myApiKeys;
      })
      .catch(() => {
        if (live) keys = [];
      });
    return () => (live = false);
  });

  const full = $derived((keys?.length ?? 0) >= MAX_KEYS);

  const create = (e: SubmitEvent) => {
    e.preventDefault();
    return gate.run(async () => {
      problem = '';
      if (!name.trim()) {
        problem = 'Give the key a name, such as the bot that uses it.';
        return;
      }
      try {
        const r = await gql(CREATE_API_KEY, { name: name.trim() });
        made = r.createApiKey.key;
        keys = [...(keys ?? []), r.createApiKey.apiKey];
        name = '';
        copied.reset();
      } catch (err) {
        problem = err instanceof ApiError ? err.message : serverMessage();
      }
    });
  };

  async function revoke(k: Key) {
    revoking = k.id;
    try {
      await gql(REVOKE_API_KEY, { id: k.id });
      keys = (keys ?? []).filter((x) => x.id !== k.id);
      notify(`Revoked ${k.name}. It stops working at once.`);
    } catch (err) {
      fail(err);
    } finally {
      revoking = null;
    }
  }

  const peak = (k: Key) => Math.max(1, ...usageBars(k.usage).map((b) => b.requests));
</script>

<Section title="Developers" id="developers-h" count={keys ? `${count(keys.length)}/${MAX_KEYS}` : undefined} lead="API keys: read-only keys for bots and tools: send one as Authorization: Bearer <key>. They read public data at a higher rate than signed-out requests, never act as you, and keep working when you sign out. Revoke a key you no longer use.">
  {#if keys === null}
    <div class="skel" aria-hidden="true"></div>
  {:else}
    {#each keys as k (k.id)}
      {@const bars = usageBars(k.usage)}
      {@const top = peak(k)}
      <Row label={k.name} hint={keyHint(k)}>
        <div class="key">
          <code class="ui-mono prefix" title="The start of the key">{k.prefix}…</code>
          <span class="use" role="img" aria-label={usageLine(k.usage)} title={usageLine(k.usage)}>
            {#each bars as b (b.day)}<span class="bar" style:height="{Math.max(b.requests ? 12 : 4, Math.round((b.requests / top) * 100))}%" class:zero={!b.requests}></span>{/each}
          </span>
          <Confirm small label="Revoke" confirm="Click again to revoke" busy={revoking === k.id} onconfirm={() => revoke(k)} />
        </div>
      </Row>
      <p class="use-line ui-mono">{usageLine(k.usage)}</p>
    {:else}
      <p class="empty">No keys yet. Make one for each bot or tool, so you can revoke it alone.</p>
    {/each}
    {#if made}
      <div class="made" role="status">
        <p>Copy your new key now. It isn't shown again.</p>
        <div class="tok">
          <input class="ui-input ui-mono" type="text" readonly aria-label="New API key" value={made} onfocus={(e) => e.currentTarget.select()} />
          <button class="ui-btn secondary small" type="button" onclick={() => copied.copy(made)}>{copied.copied ? 'Copied' : copied.failed ? 'Copy failed' : 'Copy'}</button>
          <button class="ui-btn ghost small" type="button" onclick={() => (made = '')}>Done</button>
        </div>
      </div>
    {/if}
    <form class="new" onsubmit={create}>
      <label class="sr" for="key-name">Key name</label>
      <input id="key-name" class="ui-input" type="text" maxlength="64" placeholder="Key name, such as my-chat-bot" bind:value={name} disabled={full} />
      <button class="ui-btn secondary small" type="submit" disabled={gate.busy || full}>Make key</button>
    </form>
    {#if full}<p class="hint">You have {MAX_KEYS} keys, the most you may. Revoke one to make another.</p>{/if}
    {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
    <p class="hint"><a href="/developers#api-keys">How API keys work</a></p>
  {/if}
</Section>

<style>
  .key { display: flex; align-items: center; gap: 12px; min-width: 0; flex-wrap: wrap; justify-content: flex-end; }
  .prefix { font-size: var(--t-fine); color: var(--muted); white-space: nowrap; }
  .use { display: inline-flex; align-items: flex-end; gap: 1px; width: 90px; height: 24px; flex: none; }
  .bar { flex: 1 1 0; min-width: 2px; background: var(--accent); border-radius: 1px 1px 0 0; }
  .bar.zero { background: var(--border-strong); }
  .use-line { margin: 0; padding: 0 0 8px; font-size: var(--t-fine); color: var(--muted); font-variant-numeric: tabular-nums; border-bottom: 1px solid var(--border); }
  .empty { margin: 0; padding: 10px 0; font-size: var(--t-small); color: var(--muted); border-bottom: 1px solid var(--border); }
  .made { padding: 10px 0; border-bottom: 1px solid var(--border); }
  .made p { margin: 0 0 8px; font-size: var(--t-small); }
  .tok { display: flex; gap: 8px; align-items: center; min-width: 0; }
  .tok input { flex: 1 1 auto; min-width: 0; }
  .new { display: flex; gap: 8px; align-items: center; padding: 10px 0; min-width: 0; }
  .new input { flex: 1 1 auto; min-width: 0; }
  .hint, .problem { margin: 4px 0 0; font-size: var(--t-small); color: var(--muted); }
  .hint a { color: var(--text); text-decoration: underline; text-decoration-color: var(--border-strong); text-underline-offset: 3px; }
  .skel { height: 96px; background: var(--surface-2); border-radius: 3px; }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
  @media (max-width: 639.98px) { .new input, .tok input { font-size: 16px; } .key { justify-content: flex-start; } }
</style>
