<script lang="ts">
  import type { Field } from './rest';
  import Prose from './Prose.svelte';
  let { title, fields }: { title: string; fields: Field[] } = $props();
</script>

{#if fields.length}
  <div class="param-group">
    <div class="param-group-title">{title}</div>
    {#each fields as f}
      <div class="param">
        <div class="param-sig">
          <code class="param-name">{f.name}</code>
          <span class="param-type">{f.type}</span>
          {#if f.required}<span class="param-req">required</span>{/if}
          {#if f.default}<span class="param-default">default {f.default}</span>{/if}
        </div>
        {#if f.description.length}<div class="param-desc"><Prose paras={f.description} /></div>{/if}
        {#if f.values.length}
          <div class="enum-chips">{#each f.values as v}<code>{v}</code>{/each}</div>
        {/if}
      </div>
    {/each}
  </div>
{/if}
