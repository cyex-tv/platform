# Shared components

Svelte 5 components for the public cyex.tv pages. They follow cyex-design and cyex-web: tokens from
`src/tokens.css` only, both themes, reduced motion, hover only behind `(hover: hover) and (pointer: fine)`.
See them all with live data at `/dev/components` (dev server only; a 404 in production builds).

## Shell (mounted by `src/routes/+layout.svelte`, don't add them to pages)

| Component | Props | Notes |
|---|---|---|
| `Nav` | `me?: { displayName, avatar?, platform?, admin? } \| null`, `onsignout?()` | Sticky, 64px plus the top safe area, **in the flow**: pages start right under it, so don't pad by `--nav-h`. Glass after 8px of scroll. Links: Emotes. Search field: HeaderSearch (no shortcut hint). Signed in: Upload and the account menu (your emotes, upload, sets, channels; account, admin for staff; sign out last). 720px and narrower: search and menu buttons (the avatar when signed in), menu in a `BottomSheet`. |
| `Footer` | none | Browse, Download, Cosmetics, Help, Developers, Source, Takedowns (Terms, Privacy and Contact once their pages exist), non-affiliation line. |
| `HeaderSearch` (`lib/shell`) | `phone?`, `onclose?()` | The header's search field is the search: typing drops a list under it with the top 5 emotes (`quickSearch`'s name matcher) and top 3 people; focusing it empty lists the channels you can manage by role. ARIA combobox (↑ ↓ Enter Escape). `/` outside a field focuses it (a page input marked `data-page-search` takes `/` instead); `focusSearch(query?)` from `lib/shell/search.svelte.ts` does it from code. No command palette. |

## Page shapes (redesign, 2026/11/21)

Every page is one of three shapes under the one header (`.claude/runs/2026-11-20-redesign/design-rules.md`):

| Component | Props | Usage |
|---|---|---|
| `InspectorLayout` (`lib/shell`) | `title` (the panel's uppercase label), `label?` (its accessible name), snippets `head?`, `tabs?`, `children`, `panel`, `panelActions?`; `narrow?: 'before' \| 'after'`, `fold?`, `headPad? = true`, `width? = 360`, `testid?` | The Inspector: head, tabs and content left; a full-height sticky panel right (facts, actions, related). Under 1100px one column, the panel a card before the content (or after it, `narrow="after"` for editors), folded behind a toggle with `fold`. Used by the set page, the set and emote editors and every /me page (through `MeFrame`). |
| `RailLayout` / `SideRail` (`lib/shell`) | as before | Rail + content: Settings and admin. |
| `ColumnLayout` (`lib/shell`) | `width?`, `top?` | One column: reading pages, not-found and loading states. |
| `MeFrame` (`lib/profile`) | `title`, `label?`, `panel`, `children`, `narrow?`, `fold?` | The /me pages' Inspector: you (ProfileHeader, flat), the profile's tabs plus the page you are on, the page's panel. The page's `PageHead` is its h1. |
| `PanelBlock` | `title`, `count?`, `more?: { href, label }`, `fold?`, `bind:open?`, `level? = 3`, children | One block of a panel: uppercase label (a disclosure with `fold`), hairline between blocks. Inside: `<dl class="kv">` for facts (label left, value right), `.pb-actions` for full-width buttons (`.pair` for two side by side), `.pb-hint` for a line of help. |
| `LinkTabs` | `label`, `items: { href, label, count?, current? }[]` | Tabs that are links (the URL holds the tab): bold current, no bar or fill, arrow keys. |
| `FactList` | `items: { label, value, href?, mono? }[]`, `label?` | A facts list (label left, value right, hairlines, tabular numbers) outside a PanelBlock: order totals, download facts. |
| `LinkRows` | `items: { href, label, hint?, icon?, external? }[]`, `label?` | Rows that lead somewhere (related pages, downloads): icon, label, hint line, chevron (arrow out for another site); the whole 44px row is the link. |
| `SelectSwitch` (`browse/`) | `on`, `onchange(on)`, `label? = 'Select'` | The grid's Select-mode switch. |

## Emotes

| Component | Props | Usage |
|---|---|---|
| `EmoteGrid` | `items: CardEmote[]`, `density?: 'comfortable' \| 'compact'`, `creator? = true`, `labels? = true`, `loading?`, `skeletons? = 12`, `empty?`, `eager? = 0`, `label?`, `href?(e)`, `ranked?` | `<EmoteGrid items={data.results.items} loading={more} eager={12} />`. One size: `compact` is only for small pickers, never a switch. `ranked` numbers the tiles (#1, #2 …) for Trending. |
| `EmotePicture` | `emote: Picture`, `sizes?`, `eager?`, `alt?`, `class?`, `use?` | Every emote picture in a list: animated ones play without hover; under reduced motion (the site setting, or the OS on System) the still frame, picked in script (no `<picture><source media>`: Chromium leaks those). Lazy and async by default. |
| `EmoteSkeleton` | `density?`, `creator?` | `<EmoteSkeleton />` (same box as a card) |

- `CardEmote` is the API's `EmoteFields` (or a set member spread with `shownName`): `host`, `animated`, `files`, `id`, `name`, `owner?`, `flags?`, `shownName?`.
- Art box 112px (comfortable) / 80px (compact), picture at 3/4 of it. Wide emotes (≥ 1,75:1) take two cells; the grid packs densely.
- Animated emotes always play (`EmotePicture`); under reduced motion they show `*_static.webp`. Labels: animated, zero-width, wide. Every tile in a grid row is the row's height.
- `eager` = cards above the fold (they load without `loading="lazy"`).

## Chat

| Component | Props | Usage |
|---|---|---|
| `ChatPreview` | `platform: 'twitch' \| 'kick'`, `lines: ChatLine[]`, `live?`, `visible? = 6`, `interval? = 2500`, `play? = true`, `label?` | `<ChatPreview platform="kick" {lines} live visible={5} />` |

`ChatLine = { user: string; parts: ({ text } \| { emote: ChatEmote })[] }`; `ChatEmote` is a `CardEmote`-like picture plus `name` and `zeroWidth?`. A zero-width emote stacks on the emote before it (a blank text part between them is dropped). Use made-up viewers only (lumen_fox, quietoak, nora_byte, pixelmoth). `live` adds a line every `interval` ms, cycling `lines`, with a fixed frame height; it stops off screen, in a hidden tab and under reduced motion.

## Primitives

| Component | Props | Usage |
|---|---|---|
| `Segmented` | `options: {value, label, icon?, disabled?}[]`, `value`, `onchange(v)` (controlled: the parent sets `value`), `label`, `size?: 's' \| 'm'`, `height?: number` | `<Segmented label="Sort" options={[{value:'top',label:'Top'},{value:'new',label:'New'}]} value={sort} onchange={(v) => (sort = v)} />` Radiogroup: one tab stop, arrows/Home/End select (skipping disabled). The checked option is bold with an accent icon, never a box, fill or bar. |
| `Popover` | `label`, `bind:open?`, `align?: 'start' \| 'end'`, `width? = '320px'`, `sheet?`, `onclose?`, snippets `trigger(props)`, `children`, `footer?` | `<Popover label="Filters" sheet>{#snippet trigger(p)}<button class="btn btn-s" {...p}>Filters</button>{/snippet}…</Popover>` Spread `props` on the trigger. Renders in the top layer (popover API) at fixed coordinates, follows the trigger on scroll and closes when it scrolls away, so no ancestor can clip or cover it. Escape, outside click or focus leaving closes. `sheet`: a `BottomSheet` at 720px and narrower. |
| `BottomSheet` | `bind:open`, `label`, `title? = true`, `onclose?`, snippets `children`, `footer?` | `<BottomSheet bind:open={filters} label="Filters">…{#snippet footer()}<button class="btn btn-primary">Show 120 emotes</button>{/snippet}</BottomSheet>` Modal dialog; drag the handle down, tap outside or Escape to close. |
| `CopyLink` | `url: () => string`, `kind?: 'site' (.btn.btn-s) \| 'tool' (.ui-btn.secondary)` | `<CopyLink url={() => location.origin + page.url.pathname} />` The one Copy link button: "Copy link", then "Copied" or "Couldn’t copy", sized to the longest so it never jumps, with a status line for screen readers. |
| `Chip` | `label`, `onremove?()`, `href?` | `<Chip label="Animated" onremove={() => drop('animated')} />` or `<Chip label="Animated" href={urlWithout('animated')} />` |
| `IconButton` | `label`, `icon: IconName`, `href?`, `size?: 's' (32) \| 'm' (40)`, `variant?: 'ghost' \| 'outline'`, `tip?: 'bottom' \| 'top' \| 'none'`, `pressed?`, button attributes | `<IconButton icon="copy" label="Copy link" onclick={copy} />` `label` is the aria-label and the tooltip. |
| `Icon` | `name: IconName`, `size? = 16`, `stroke? = 1.7` | `<Icon name="search" />` Names: search, x, menu, chevron-down, arrow-right, check, sliders, tv, file, log-out, enter, grid, grid-dense, copy, plus, more, pause, play, user, arrow-left, flag. Add Lucide paths to the map as needed. |
| `Eyebrow` | `lit?: 'top' \| 'middle' \| 'bottom'`, children | `<Eyebrow lit="top">For channels</Eyebrow>` |
| `SectionHead` | `title`, `eyebrow?`, `lit?`, `lead?`, `level?: 1 \| 2`, `size?: 'section' \| 'page'`, `center?`, `id?`, snippet `actions?` | `<SectionHead eyebrow="For channels" lit="top" title="One set per channel." lead="…" />` Page titles: `level={1} size="page"`. |
| `Mark`, `ThemeToggle` | as before | |

## Global classes (`src/app.css`)

`.page` (content column), `.wrap`, `.section`, `.btn`, `.btn-primary`, `.btn-s`, `.actions`, `.link`, `.input`, `.field`, `.mount`, `.steps`, `.band`, `.page-head`, `.muted`, `.fine`, `.sr-only`. The nav already shows one accent button (Get the extension); pages add at most one primary action per view.

## Things to know

- Tokens added: `--ease-drawer`, `--dur-pop`, `--dur-sheet`, `--delay-tip`, `--z-nav`, `--z-pop`, `--z-tip`, `--scrim`, `--shadow-pop`, `--t-label`, `--emote-art`, `--emote-art-compact`, `--emote-chat`.
- Inputs are 16px (no iOS zoom). Controls have `touch-action: manipulation` and no text selection.
- `src/lib/shell/search.ts` holds the header search's pure logic (`move`, `typing`, `announce`, `searchHref`); `src/lib/shell/nav.ts` the header links and the channel role order.
- `src/lib/emote.ts` adds `hasStill`, `isWide`, `srcsetW` (width descriptors; pair with `sizes`).

## Fields

| Component | Props | Usage |
|---|---|---|
| `TagInput` | `tags: string[]`, `onchange(tags)`, `label`, `showLabel?`, `placeholder?`, `max?`, `clean?(raw)`, `hint?`, `id?`, `disabled?` | Enter (or a comma) turns the typed text into a chip; pasting `a, b` adds two; Backspace in the empty field takes the last chip off; each chip has a remove button. Rules in `tag-input.ts` (tested). The browse filters use it; the upload form can too. |
