<script lang="ts">
  // A channel's webhooks (ADR 0034), under the channel on the Channels page: for its owner and
  // its account editors with the channel settings permission. Folded to one row until opened;
  // open, each webhook shows its URL, events and state with Send test, Log (the last 50
  // deliveries), New secret, Turn off/on and Delete, and a form adds one (https URL and events).
  // A secret is shown once, right after it is made or replaced.
  import { ApiError, gql } from '$lib/api';
  import { useCopied } from '$lib/clipboard.svelte';
  import { serverMessage } from '$lib/components/problem';
  import Row from '$lib/components/Row.svelte';
  import { count, dateTime } from '$lib/format';
  import type { EventType, Platform, WebhookFieldsFragment, WebhookDeliveriesQuery } from '$lib/gql/graphql';
  import { CHANNEL_WEBHOOKS, CREATE_WEBHOOK, DELETE_WEBHOOK, ROTATE_WEBHOOK_SECRET, TEST_WEBHOOK, UPDATE_WEBHOOK, WEBHOOK_DELIVERIES } from '$lib/queries';
  import Confirm from '$lib/tool/Confirm.svelte';
  import { Gate } from '$lib/tool/gate.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { deliveryLine, eventLabel, MAX_WEBHOOKS, urlProblem, WEBHOOK_EVENTS, webhookState } from './developers';

  let { platform, id, key }: { platform: Platform; id: string; key: string } = $props();

  type Hook = WebhookFieldsFragment;
  type Delivery = WebhookDeliveriesQuery['webhookDeliveries'][number];
  const gate = new Gate(800);
  const copied = useCopied();
  let open = $state(false);
  let hooks = $state<Hook[] | null>(null);
  let url = $state('');
  let picked = $state<EventType[]>(['SET_EMOTE_ADDED' as EventType, 'SET_EMOTE_REMOVED' as EventType]);
  let secret = $state<{ hook: string; value: string } | null>(null);
  let problem = $state('');
  let busy = $state<string | null>(null);
  let logs = $state<Record<string, Delivery[] | undefined>>({});

  async function load() {
    try {
      hooks = (await gql(CHANNEL_WEBHOOKS, { platform, id })).channelWebhooks;
    } catch (err) {
      hooks = [];
      fail(err);
    }
  }

  function toggle() {
    open = !open;
    if (open && hooks === null) load();
  }

  const replace = (h: Hook) => (hooks = (hooks ?? []).map((x) => (x.id === h.id ? h : x)));

  const add = (e: SubmitEvent) => {
    e.preventDefault();
    return gate.run(async () => {
      problem = urlProblem(url) ?? (picked.length ? '' : 'Pick at least one event.');
      if (problem) return;
      try {
        const r = await gql(CREATE_WEBHOOK, { platform, id, input: { url: url.trim(), events: picked } });
        hooks = [...(hooks ?? []), r.createWebhook.webhook];
        secret = { hook: r.createWebhook.webhook.id, value: r.createWebhook.secret };
        copied.reset();
        url = '';
      } catch (err) {
        problem = err instanceof ApiError ? err.message : serverMessage();
      }
    });
  };

  async function act(h: Hook, what: string, run: () => Promise<void>) {
    busy = `${h.id}:${what}`;
    try {
      await run();
    } catch (err) {
      fail(err);
    } finally {
      busy = null;
    }
  }

  const setEnabled = (h: Hook, enabled: boolean) =>
    act(h, 'enabled', async () => {
      replace((await gql(UPDATE_WEBHOOK, { id: h.id, input: { enabled } })).updateWebhook);
      notify(enabled ? 'Turned on. Events are sent again.' : 'Turned off. Nothing is sent until you turn it on.');
    });
  const test = (h: Hook) =>
    act(h, 'test', async () => {
      await gql(TEST_WEBHOOK, { id: h.id });
      notify('Test sent. It shows in the log within seconds.');
      if (logs[h.id]) logs[h.id] = (await gql(WEBHOOK_DELIVERIES, { id: h.id })).webhookDeliveries;
    });
  const rotate = (h: Hook) =>
    act(h, 'rotate', async () => {
      const r = await gql(ROTATE_WEBHOOK_SECRET, { id: h.id });
      secret = { hook: h.id, value: r.rotateWebhookSecret.secret };
      copied.reset();
    });
  const remove = (h: Hook) =>
    act(h, 'delete', async () => {
      await gql(DELETE_WEBHOOK, { id: h.id });
      hooks = (hooks ?? []).filter((x) => x.id !== h.id);
      if (secret?.hook === h.id) secret = null;
      notify('Webhook deleted.');
    });
  const showLog = (h: Hook) =>
    act(h, 'log', async () => {
      if (logs[h.id]) logs[h.id] = undefined;
      else logs[h.id] = (await gql(WEBHOOK_DELIVERIES, { id: h.id })).webhookDeliveries;
    });

  const pick = (e: EventType, on: boolean) => (picked = on ? [...picked, e] : picked.filter((x) => x !== e));
  const full = $derived((hooks?.length ?? 0) >= MAX_WEBHOOKS);
</script>

<Row label="Webhooks" hint={hooks ? `${count(hooks.length)} of ${MAX_WEBHOOKS}. Events from this channel sent to your server, signed.` : 'Events from this channel sent to your server, signed.'}>
  <button class="ui-btn secondary small" type="button" aria-expanded={open} aria-controls="wh-{key}" onclick={toggle}>{open ? 'Close' : 'Manage'}</button>
</Row>

{#if open}
  <div class="wh" id="wh-{key}">
    {#if hooks === null}
      <div class="skel" aria-hidden="true"></div>
    {:else}
      {#each hooks as h (h.id)}
        <div class="hook" class:off={!h.enabled}>
          <div class="hook-head">
            <span class="url ui-mono" title={h.url}>{h.url}</span>
            <span class="state">{webhookState(h)}</span>
          </div>
          <p class="events">{h.events.map(eventLabel).join(', ')} · made {dateTime(h.createdAt)}</p>
          <div class="acts">
            <button class="ui-btn secondary small" type="button" disabled={busy !== null} onclick={() => test(h)}>Send test</button>
            <button class="ui-btn ghost small" type="button" aria-expanded={!!logs[h.id]} disabled={busy !== null} onclick={() => showLog(h)}>{logs[h.id] ? 'Hide log' : 'Log'}</button>
            <button class="ui-btn ghost small" type="button" disabled={busy !== null} onclick={() => setEnabled(h, !h.enabled)}>{h.enabled ? 'Turn off' : 'Turn on'}</button>
            <button class="ui-btn ghost small" type="button" disabled={busy !== null} onclick={() => rotate(h)}>New secret</button>
            <Confirm small label="Delete" confirm="Click again to delete" busy={busy !== null} onconfirm={() => remove(h)} />
          </div>
          {#if secret?.hook === h.id}
            <div class="secret" role="status">
              <p>Copy the signing secret now. It isn't shown again. After New secret, the old one also signs for 24 hours.</p>
              <div class="tok">
                <input class="ui-input ui-mono" type="text" readonly aria-label="Signing secret" value={secret.value} onfocus={(e) => e.currentTarget.select()} />
                <button class="ui-btn secondary small" type="button" onclick={() => secret && copied.copy(secret.value)}>{copied.copied ? 'Copied' : copied.failed ? 'Copy failed' : 'Copy'}</button>
                <button class="ui-btn ghost small" type="button" onclick={() => (secret = null)}>Done</button>
              </div>
            </div>
          {/if}
          {#if logs[h.id]}
            {@const log = logs[h.id] ?? []}
            <ol class="log" aria-label="Latest deliveries">
              {#each log as d (d.id)}
                <li class:failed={d.state === 'failed'}>
                  <span class="ui-mono when">{dateTime(d.createdAt)}</span>
                  <span class="what">{d.eventType}</span>
                  <span class="how">{deliveryLine(d)}</span>
                </li>
              {:else}
                <li class="none">No deliveries yet.</li>
              {/each}
            </ol>
          {/if}
        </div>
      {:else}
        <p class="empty">No webhooks yet. Add your server's https URL to get this channel's events.</p>
      {/each}

      {#if !full}
        <form class="new" onsubmit={add}>
          <label class="lbl" for="wh-url-{key}">New webhook</label>
          <input id="wh-url-{key}" class="ui-input ui-mono" type="url" inputmode="url" placeholder="https://example.com/cyex-webhook" bind:value={url} />
          <fieldset class="pick">
            <legend class="sr">Events to send</legend>
            {#each WEBHOOK_EVENTS as e (e.id)}
              <label class="chk"><input type="checkbox" checked={picked.includes(e.id)} onchange={(ev) => pick(e.id, ev.currentTarget.checked)} /> {e.label}</label>
            {/each}
          </fieldset>
          <div class="acts"><button class="ui-btn secondary small" type="submit" disabled={gate.busy}>Add webhook</button></div>
        </form>
      {:else}
        <p class="empty">This channel has {MAX_WEBHOOKS} webhooks, the most it may.</p>
      {/if}
      {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
      <p class="hint"><a href="/developers#webhooks">How webhooks and their signatures work</a></p>
    {/if}
  </div>
{/if}

<style>
  .wh { padding: 4px 0 8px; border-bottom: 1px solid var(--border); min-width: 0; }
  .hook { padding: 10px 0; border-bottom: 1px solid var(--border); min-width: 0; }
  .hook.off .url { color: var(--muted); }
  .hook-head { display: flex; gap: 12px; align-items: baseline; min-width: 0; }
  .url { flex: 1 1 auto; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--t-small); }
  .state { flex: none; font-size: var(--t-fine); color: var(--muted); }
  .events { margin: 4px 0 8px; font-size: var(--t-fine); color: var(--muted); }
  .acts { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .secret { margin-top: 10px; }
  .secret p { margin: 0 0 6px; font-size: var(--t-small); }
  .tok { display: flex; gap: 8px; align-items: center; min-width: 0; }
  .tok input { flex: 1 1 auto; min-width: 0; }
  .log { list-style: none; margin: 10px 0 0; padding: 0; max-height: 320px; overflow-y: auto; border-top: 1px solid var(--border); }
  .log li { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; gap: 12px; align-items: baseline; padding: 6px 0; border-bottom: 1px solid var(--border); font-size: var(--t-fine); }
  .log li.failed .how { color: var(--danger, var(--text)); }
  .log .when { color: var(--muted); font-variant-numeric: tabular-nums; }
  .log .what { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .log .none { display: block; color: var(--muted); }
  .new { display: grid; gap: 8px; padding: 12px 0 4px; }
  .lbl { font-size: var(--t-small); font-weight: 600; }
  .pick { display: flex; flex-wrap: wrap; gap: 4px 16px; margin: 0; padding: 0; border: 0; min-width: 0; }
  .chk { display: inline-flex; align-items: center; gap: 6px; min-height: 32px; font-size: var(--t-small); }
  .empty, .hint, .problem { margin: 8px 0 0; font-size: var(--t-small); color: var(--muted); }
  .hint a { color: var(--text); text-decoration: underline; text-decoration-color: var(--border-strong); text-underline-offset: 3px; }
  .skel { height: 72px; background: var(--surface-2); border-radius: 3px; }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
  @media (max-width: 639.98px) {
    .new input, .tok input { font-size: 16px; }
    .log li { grid-template-columns: minmax(0, 1fr); gap: 2px; }
    .chk { min-height: 44px; }
  }
</style>
