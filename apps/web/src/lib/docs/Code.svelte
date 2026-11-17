<script lang="ts">
  // One sample block. With `tabs` it is a language-switched request (the choice is shared by every
  // block on the page, see behaviour.ts); without, a single labelled block such as a response.
  type Tab = { lang: string; label: string; code: string };
  let {
    tabs,
    label = '',
    meta = '',
    code = '',
  }: { tabs?: Tab[]; label?: string; meta?: string; code?: string } = $props();
</script>

<div class="code-block">
  <div class="code-head">
    {#if tabs && tabs.length}
      <div class="lang-tabs" role="tablist" aria-label="Language">
        {#each tabs as t, i}
          <button type="button" role="tab" class="lang-tab" data-lang={t.lang} aria-selected={i === 0}>{t.label}</button>
        {/each}
      </div>
    {:else}
      <div class="resp-head"><span class="resp-status">{label}</span>{#if meta}<span class="resp-type">{meta}</span>{/if}</div>
    {/if}
    <button type="button" class="code-copy" aria-label="Copy">
      <svg class="icon-copy" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><rect x="5.5" y="5.5" width="8" height="8" rx="1"/><path d="M10.5 3.5v-1h-8v8h1"/></svg>
      <svg class="icon-done" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 8.5l3.2 3L13 4.5"/></svg>
    </button>
  </div>
  {#if tabs && tabs.length}
    {#each tabs as t, i}
      <pre data-lang={t.lang} hidden={i > 0}><code>{t.code}</code></pre>
    {/each}
  {:else}
    <pre><code>{code}</code></pre>
  {/if}
</div>
