<script lang="ts">
  // One GraphQL query, mutation or subscription.
  import Code from './Code.svelte';
  import Prose from './Prose.svelte';
  import TypeRef from './TypeRef.svelte';
  import { gqlChipClass, gqlChipLong, type GqlOpPayload } from './lazy';
  let { op, known }: { op: GqlOpPayload; known: string[] } = $props();
  const accessLabel = { 'signed-in': 'Signed in', restricted: 'Restricted' } as const;
  const accessHint = { 'signed-in': 'Needs a session', restricted: 'Needs a session and a permission' } as const;
  const args = $derived(op.args.length ? `(${op.args.map((a) => `${a.name}: ${a.type}`).join(', ')})` : '');
</script>

<section class="op" id={op.id}>
  <div class="op-main">
    <div class="op-head">
      <span class="method {gqlChipClass[op.kind]}">{gqlChipLong[op.kind]}</span>
      <h3 class="op-title">{op.name}</h3>
      {#if op.access !== 'public'}
        <span class="auth-chip" class:neutral={op.access === 'signed-in'} title={accessHint[op.access]}>{accessLabel[op.access]}</span>
      {/if}
      {#if op.deprecated}<span class="auth-chip neutral">Deprecated</span>{/if}
    </div>
    <div class="op-path"><span class="op-host">{`${op.kind} `}</span>{op.name}{args}<span class="op-host">: </span>{op.returns}</div>
    {#if op.description.length}<div class="op-desc"><Prose paras={op.description} /></div>{/if}
    {#if op.deprecated}<p class="op-desc">{op.deprecated}</p>{/if}
    {#if op.args.length}
      <div class="param-group">
        <div class="param-group-title">Arguments</div>
        {#each op.args as a}
          <div class="param">
            <div class="param-sig">
              <code class="param-name">{a.name}</code>
              <span class="param-type"><TypeRef type={a.type} {known} /></span>
              {#if a.required}<span class="param-req">required</span>{/if}
              {#if a.default}<span class="param-default">default <code>{a.default}</code></span>{/if}
            </div>
            {#if a.description.length}<div class="param-desc"><Prose paras={a.description} /></div>{/if}
          </div>
        {/each}
      </div>
    {/if}
    <div class="param-group">
      <div class="param-group-title">Returns</div>
      <div class="resp-row"><span class="status-pip ok"></span><code class="resp-code"><TypeRef type={op.returns} {known} /></code></div>
    </div>
  </div>
  <div class="op-aside">
    <div class="code-panel">
      {#if op.kind === 'subscription'}
        <Code label="Operation" meta="GraphQL" code={op.document} copy="Copy operation" />
        <Code label="Frames" meta="→ sent, ← received" code={op.frames ?? ''} copy="Copy frames" />
      {:else}
        <Code tabs={op.samples} copy="Copy request" />
        <Code label="200" meta="application/json" code={op.response} ok copy="Copy response" />
      {/if}
    </div>
  </div>
</section>
