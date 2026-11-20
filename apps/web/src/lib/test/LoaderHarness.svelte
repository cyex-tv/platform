<script lang="ts">
  // Test harness: a page that loads with loader(); the state is printed so a test can read it.
  import { loader, type LoadCtx, type Loader } from '$lib/tool/loader.svelte';
  let { id, run, expose }: { id: string; run: (id: string, ctx: LoadCtx) => Promise<string>; expose?: (l: Loader<string>) => void } = $props();
  const l = loader<string>((ctx) => run(id, ctx));
  // svelte-ignore state_referenced_locally
  expose?.(l);
</script>

<p data-testid="state">{l.loading ? 'loading' : 'idle'}|{l.data ?? '-'}|{l.error ?? '-'}|{l.cause ?? '-'}</p>
