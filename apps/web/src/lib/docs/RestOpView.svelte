<script lang="ts">
  // One REST endpoint: description and parameters beside the sample panel.
  import Code from './Code.svelte';
  import Fields from './Fields.svelte';
  import Inline from './Inline.svelte';
  import Prose from './Prose.svelte';
  import type { RestOp } from './rest';
  let { op, base }: { op: RestOp; base: string } = $props();
</script>

<section class="op" id={op.id}>
  <div class="op-main">
    <div class="op-head">
      <span class="method m-{op.method.toLowerCase()}">{op.method}</span>
      <h3 class="op-title">{op.title}</h3>
      {#if op.signedIn}<span class="auth-chip neutral" title="Needs a session">Signed in</span>{/if}
    </div>
    <div class="op-path"><span class="op-host">{base}</span>{op.path}</div>
    {#if op.summaryRest.length || op.description.length}
      <div class="op-desc"><Prose paras={[...op.summaryRest, ...op.description]} /></div>
    {/if}
    <Fields title="Path parameters" fields={op.pathParams} />
    <Fields title="Query parameters" fields={op.queryParams} />
    <Fields title="Headers" fields={op.headerParams} />
    {#if op.bodyFields.length}
      <Fields title={`Request body · ${op.bodyType}`} fields={op.bodyFields} />
    {/if}
    <div class="param-group">
      <div class="param-group-title">Responses</div>
      {#each op.responses as r}
        <div class="resp-row">
          <span class="status-pip" class:ok={r.ok} class:err={!r.ok}></span>
          <code class="resp-code">{r.status}</code>
          <span class="resp-desc">{#if r.description.length}<Inline paras={r.description} />{/if}</span>
        </div>
      {/each}
    </div>
  </div>
  <div class="op-aside">
    <div class="code-panel">
      <Code tabs={op.samples} copy="Copy request" />
      <Code label={op.responseStatus} meta={op.responseType} code={op.responseExample} ok copy="Copy response" />
    </div>
  </div>
</section>
