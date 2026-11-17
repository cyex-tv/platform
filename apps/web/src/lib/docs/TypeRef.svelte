<script lang="ts">
  // A GraphQL type written out, with its named type linked when the reference documents it.
  import { typeId } from './graphql';
  let { type, known }: { type: string; known: string[] } = $props();
  const m = $derived(/^([^\w]*)(\w+)([^\w]*)$/.exec(type));
</script>

{#if m && known.includes(m[2])}{m[1]}<a href="#{typeId(m[2])}">{m[2]}</a>{m[3]}{:else}{type}{/if}
