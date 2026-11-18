<script lang="ts">
  // The one empty, loading and error state, on every page (public, signed in, admin, docs and the
  // error pages): a hairline panel with an icon (or spinner), a title, a sentence and actions
  // (`ui-btn`s). Errors are announced (role=alert), loading is a status. `level={1}` when the
  // state is the whole page: its title is then the page's h1, at the one page-title size.
  // `bare` drops the panel for a state inside another panel.
  import type { Snippet } from 'svelte';

  type Props = {
    kind?: 'empty' | 'loading' | 'error';
    title?: string;
    /** A headline with markup in it (a name in <bdi>); used instead of `title`. */
    heading?: Snippet;
    children?: Snippet;
    actions?: Snippet;
    bare?: boolean;
    /** 1: the title is the page's h1 (a not-found or error page). */
    level?: 1 | 2;
    /** Overrides the role the kind implies (a not-found page is a status, not an alert). */
    role?: 'status' | 'alert' | null;
  };
  let { kind = 'empty', title, heading, children, actions, bare = false, level = 2, role: own }: Props = $props();
  const role = $derived(own !== undefined ? (own ?? undefined) : kind === 'error' ? 'alert' : kind === 'loading' ? 'status' : undefined);
  const tag = $derived(kind === 'loading' ? 'p' : level === 1 ? 'h1' : 'h2');
</script>

<div class="state" class:bare class:page={level === 1} {role} aria-busy={kind === 'loading' ? 'true' : undefined}>
  {#if kind === 'loading'}
    <span class="ui-spinner" aria-hidden="true"></span>
  {:else if kind === 'error'}
    <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9" /><path d="M12 8v5M12 16.5v.01" /></svg>
  {:else}
    <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="3" width="18" height="18" rx="3" /><path d="M8 14s1.5 2 4 2 4-2 4-2M9 9h.01M15 9h.01" /></svg>
  {/if}
  {#if heading}<svelte:element this={tag} class="t">{@render heading()}</svelte:element>{:else if title}<svelte:element this={tag} class="t">{title}</svelte:element>{/if}
  {#if children}<p>{@render children()}</p>{/if}
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</div>

<style>
  .state { display: grid; justify-items: center; gap: 10px; min-height: 180px; align-content: center; padding: 40px 20px; text-align: center;
    background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); color: var(--muted); }
  .bare { background: none; border: 0; border-radius: 0; min-height: 0; padding: 32px 16px; }
  .page { min-height: 280px; }
  svg { width: 28px; height: 28px; fill: none; stroke: var(--dim); stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
  .t { margin: 2px 0 0; font-size: var(--t-ui); font-weight: 600; line-height: 1.35; color: var(--text); overflow-wrap: anywhere; }
  .page .t { font-size: var(--t-page); line-height: 1.25; letter-spacing: -.02em; }
  p:not(.t) { margin: 0; max-width: 52ch; font-size: var(--t-small); line-height: 1.5; text-wrap: pretty; overflow-wrap: anywhere; }
  .actions { display: flex; flex-wrap: wrap; justify-content: center; gap: 8px; margin-top: 6px; }
</style>
