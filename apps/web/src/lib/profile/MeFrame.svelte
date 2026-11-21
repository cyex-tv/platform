<script lang="ts">
  // The frame of the /me pages (Channels, Notifications, Subscription, Upload, the set and emote
  // editors): the Workspace (redesign round 2). Your area's rail on the left (MeRail: your places
  // with counts, your sets with their meters), the page in the middle (its PageHead is the h1, a
  // Toolbar over lists), and the page's inspector on the right (facts, actions, the picked item).
  // Under 1100px the panel lies flat before the content (or after it, `narrow="after"`), folded
  // under its label with `fold` (on by default); under 900px the rail is a tab strip.
  import type { Snippet } from 'svelte';
  import WorkspaceLayout from '$lib/shell/WorkspaceLayout.svelte';
  import MeRail from './MeRail.svelte';

  type Props = {
    /** The panel's label. */
    title: string;
    label?: string;
    panel?: Snippet;
    children: Snippet;
    head?: Snippet;
    toolbar?: Snippet;
    panelActions?: Snippet;
    narrow?: 'before' | 'after';
    fold?: boolean;
    width?: number;
  };
  let { title, label, panel, children, head, toolbar, panelActions, narrow = 'before', fold = true, width }: Props = $props();
</script>

{#snippet rail()}<MeRail />{/snippet}

<WorkspaceLayout {rail} {title} {label} {panel} {head} {toolbar} {panelActions} {narrow} {fold} {width} testid="me-panel">
  {@render children()}
</WorkspaceLayout>
