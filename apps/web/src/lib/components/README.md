# Shared components

Svelte 5 components for the public cyex.tv pages. They follow cyex-design and cyex-web: tokens from
`src/tokens.css` only, both themes, reduced motion, hover only behind `(hover: hover) and (pointer: fine)`.
See them all with live data at `/dev/components` (dev server only; a 404 in production builds).

## Shell (mounted by `src/routes/+layout.svelte`, don't add them to pages)

| Component | Props | Notes |
|---|---|---|
| `Nav` | `me?: { displayName, avatar? } \| null`, `onsignout?()` | Sticky, 64px plus the top safe area, **in the flow**: pages start right under it, so don't pad by `--nav-h`. Glass after 8px of scroll. Search field opens the palette. 720px and narrower: search and menu buttons, menu in a `BottomSheet`. |
| `Footer` | none | Product, Developers, Legal (`/terms`, `/privacy`, `/contact`), non-affiliation line. |
| `CommandPalette` | none | Ctrl K / Cmd K, or `/` outside a field. Open it from code with `openPalette(query?)` from `palette.svelte.ts`. |

## Emotes

| Component | Props | Usage |
|---|---|---|
| `EmoteGrid` | `items: CardEmote[]`, `density?: 'comfortable' \| 'compact'`, `creator? = true`, `labels? = true`, `loading?`, `skeletons? = 12`, `empty?`, `eager? = 0`, `label?`, `href?(e)` | `<EmoteGrid items={data.results.items} {density} loading={more} eager={12} />` |
| `EmoteCard` | `emote: CardEmote`, `density?`, `creator?`, `labels?`, `href?`, `eager?` | `<EmoteCard emote={e} />` (for strips and rows outside a grid) |
| `EmoteSkeleton` | `density?`, `creator?` | `<EmoteSkeleton />` (same box as a card) |

- `CardEmote` is the API's `EmoteFields` (or a set member spread with `shownName`): `host`, `animated`, `files`, `id`, `name`, `owner?`, `flags?`, `shownName?`.
- Art box 112px (comfortable) / 80px (compact), picture at 3/4 of it. Wide emotes (≥ 1,75:1) take two cells; the grid packs densely.
- Animated emotes show `1x_static.webp` and play on mouse hover or keyboard focus; never under reduced motion. Labels: animated, zero-width, wide.
- `eager` = cards above the fold (they load without `loading="lazy"`).

## Chat

| Component | Props | Usage |
|---|---|---|
| `ChatPreview` | `platform: 'twitch' \| 'kick'`, `lines: ChatLine[]`, `live?`, `visible? = 6`, `interval? = 2500`, `play? = true`, `label?` | `<ChatPreview platform="kick" {lines} live visible={5} />` |

`ChatLine = { user: string; parts: ({ text } \| { emote: ChatEmote })[] }`; `ChatEmote` is a `CardEmote`-like picture plus `name` and `zeroWidth?`. A zero-width emote stacks on the emote before it (a blank text part between them is dropped). Use made-up viewers only (lumen_fox, quietoak, nora_byte, pixelmoth). `live` adds a line every `interval` ms, cycling `lines`, with a fixed frame height; it stops off screen, in a hidden tab and under reduced motion.

## Primitives

| Component | Props | Usage |
|---|---|---|
| `Segmented` | `options: {value, label}[]`, `bind:value`, `label`, `size?: 's' \| 'm'`, `onchange?(v)` | `<Segmented label="Sort" options={[{value:'top',label:'Top'},{value:'new',label:'New'}]} bind:value={sort} onchange={go} />` Radiogroup: one tab stop, arrows/Home/End select. |
| `Popover` | `label`, `bind:open?`, `align?: 'start' \| 'end'`, `width? = '320px'`, `sheet?`, `onclose?`, snippets `trigger(props)`, `children`, `footer?` | `<Popover label="Filters" sheet>{#snippet trigger(p)}<button class="btn btn-s" {...p}>Filters</button>{/snippet}…</Popover>` Spread `props` on the trigger. Escape, outside click or focus leaving closes. `sheet`: a `BottomSheet` at 720px and narrower. |
| `BottomSheet` | `bind:open`, `label`, `title? = true`, `onclose?`, snippets `children`, `footer?` | `<BottomSheet bind:open={filters} label="Filters">…{#snippet footer()}<button class="btn btn-primary">Show 120 emotes</button>{/snippet}</BottomSheet>` Modal dialog; drag the handle down, tap outside or Escape to close. |
| `Chip` | `label`, `onremove?()`, `href?` | `<Chip label="Animated" onremove={() => drop('animated')} />` or `<Chip label="Animated" href={urlWithout('animated')} />` |
| `IconButton` | `label`, `icon: IconName`, `href?`, `size?: 's' (32) \| 'm' (40)`, `variant?: 'ghost' \| 'outline'`, `tip?: 'bottom' \| 'top' \| 'none'`, `pressed?`, button attributes | `<IconButton icon="copy" label="Copy link" onclick={copy} />` `label` is the aria-label and the tooltip. |
| `Icon` | `name: IconName`, `size? = 16`, `stroke? = 1.7` | `<Icon name="search" />` Names: search, x, menu, chevron-down, arrow-right, check, sliders, tv, file, log-out, enter, grid, grid-dense, copy, plus, more, pause, play, user. Add Lucide paths to the map as needed. |
| `Eyebrow` | `lit?: 'top' \| 'middle' \| 'bottom'`, children | `<Eyebrow lit="top">For channels</Eyebrow>` |
| `SectionHead` | `title`, `eyebrow?`, `lit?`, `lead?`, `level?: 1 \| 2`, `size?: 'section' \| 'page'`, `center?`, `id?`, snippet `actions?` | `<SectionHead eyebrow="For channels" lit="top" title="One set per channel." lead="…" />` Page titles: `level={1} size="page"`. |
| `Mark`, `ThemeToggle` | as before | |

## Global classes (`src/app.css`)

`.page` (content column), `.wrap`, `.section`, `.btn`, `.btn-primary`, `.btn-s`, `.actions`, `.link`, `.input`, `.field`, `.mount`, `.steps`, `.band`, `.page-head`, `.muted`, `.fine`, `.sr-only`. The nav already shows one accent button (Get the extension); pages add at most one primary action per view.

## Things to know

- Tokens added: `--ease-drawer`, `--dur-pop`, `--dur-sheet`, `--delay-tip`, `--z-nav`, `--z-pop`, `--z-tip`, `--scrim`, `--shadow-pop`, `--t-label`, `--emote-art`, `--emote-art-compact`, `--emote-chat`.
- Inputs are 16px (no iOS zoom). Controls have `touch-action: manipulation` and no text selection.
- `src/lib/palette.ts` holds the palette's pure logic (`asLogin`, `channelJumps`, `pageJumps`, `PAGES`).
- `src/lib/emote.ts` adds `hasStill`, `isWide`, `srcsetW` (width descriptors; pair with `sizes`).
