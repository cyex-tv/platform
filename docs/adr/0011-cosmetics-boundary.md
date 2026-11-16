# 0011: Cosmetics boundary

- Status: accepted
- Date: 2026/11/16

## Context
"cyex badges" could mean two different things:
- **cyex deck supporter badges**: founder and supporter badges, plus Supporter+ custom art, bought in the deck's store and served by cyex-api at `api.cyex.app/badges/v1`. They belong to cyex.app.
- **cyex.tv chat cosmetics**: badges and name paints that cyex.tv may offer later. They belong to cyex.tv.

Mixing them would couple the two products (0001).

## Decision
- cyex.tv never stores, serves or changes cyex deck badges.
- The extension shows cyex deck badges as **their own provider**, "cyex deck", reading the public badge API as its spec says: one snapshot, `If-None-Match`, a refresh every 30 min, nothing per message. That provider has its own toggle.
- cyex.tv cosmetics, when built, get their own:
  - provider name, "cyex.tv";
  - API namespace, `/v1/cosmetics` (reserved now);
  - event types, `cosmetic.*`;
  - R2 prefix, `cosmetics/`;
  - permissions, `cosmetics.*`.
- In clients, the provider is always shown in tooltips, so a user can tell which badge came from where.
- In the deck, the cyex.tv emote provider is named `CyexTv` (label "cyex.tv"). It never touches `badges/cyex.rs`.

## Consequences
- The two can evolve on their own.
- Two "cyex" badge sources can show next to each other on one user, which is why tooltips must name the provider.
