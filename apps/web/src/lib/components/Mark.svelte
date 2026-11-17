<script lang="ts">
  /** The cyex mark (cyex-design brand.md). `lit` picks the one bar lit, for section eyebrows. */
  let { size = 22, lit = 'all' }: { size?: number; lit?: 'all' | 'top' | 'middle' | 'bottom' } = $props();
  const fill = (bar: 'top' | 'middle' | 'bottom') => {
    if (lit === 'all') return bar === 'middle' ? 'var(--accent)' : 'var(--mark-ink)';
    if (bar !== lit) return 'var(--border-strong)';
    return bar === 'middle' ? 'var(--accent)' : 'var(--mark-ink)';
  };
</script>

<svg viewBox="3 5 25 22" width={size} height={Math.round((size * 22) / 25)} aria-hidden="true">
  <rect x="3" y="5" width="15" height="6" fill={fill('top')} />
  <rect x="10" y="13" width="18" height="6" fill={fill('middle')} />
  <rect x="3" y="21" width="15" height="6" fill={fill('bottom')} />
</svg>
