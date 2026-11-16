# 0006: Frontend

- Status: accepted
- Date: 2026/11/16

## Context
cyex deck is Svelte 5. cyex has a design system in code, the cyex-skills repo:
- `cyex-design` holds the shared rules;
- `cyex-web` covers marketing pages;
- `cyex-app` covers dense tools;
- `cyex-docs` covers API docs.

The first draft of the plan chose Vue "to match 7TV", but 7TV's current site is SvelteKit.

## Decision
- **SvelteKit + TypeScript** in `apps/web`, served at `cyex.tv`.
- Styling uses the cyex token CSS and plain CSS. No Tailwind, no component library that brings its own look.
  - home page: `cyex-web`
  - search, emote pages, my sets, upload, admin: `cyex-app` at web-tool density
  - API docs: `cyex-docs`
- The API client is generated from `openapi.yaml` (0008). No hand-written fetch calls for endpoints in the spec.
- Server-side rendering for public pages (emote and channel pages are shared as links). Signed-in and admin pages can be client-rendered.

## Consequences
- The same framework as the deck, so components and patterns carry over.
- The design rules are strict (tokens only, both themes, reduced motion), and a `#uitest` pass is part of "done" for web phases.
