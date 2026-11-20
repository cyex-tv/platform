<script lang="ts">
  // One GraphQL type, input or enum with its members.
  import Prose from './Prose.svelte';
  import TypeRef from './TypeRef.svelte';
  import { typeChip } from './lazy';
  import type { GqlType } from './graphql';
  let { t, known }: { t: GqlType; known: string[] } = $props();
</script>

<section class="op type" id={t.id}>
  <div class="op-main">
    <div class="op-head">
      <span class="method m-get">{typeChip(t)}</span>
      <h3 class="op-title">{t.name}</h3>
    </div>
    {#if t.description.length}<div class="op-desc"><Prose paras={t.description} /></div>{/if}
    <div class="param-group">
      <div class="param-group-title">{t.kind === 'enum' ? 'Values' : 'Fields'}</div>
      <div class="fields">
        {#each t.members as m}
          <div class="param">
            <div class="param-sig">
              <code class="param-name">{m.name}</code>
              {#if m.type}<span class="param-type"><TypeRef type={m.type} {known} /></span>{/if}
              {#if m.required}<span class="param-req">required</span>{/if}
            </div>
            {#if m.description.length}<div class="param-desc"><Prose paras={m.description} /></div>{/if}
          </div>
        {/each}
      </div>
    </div>
  </div>
</section>
