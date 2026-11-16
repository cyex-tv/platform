# 0004: Search

- Status: accepted
- Date: 2026/11/16

## Context
Emote search is mostly short-name prefix and fuzzy matching, plus tags, filters and popularity. A separate engine (Meilisearch, Typesense) means another service, a sync pipeline and backups.

## Decision
- v1 uses Postgres:
  - `pg_trgm` GIN index on `lower(name)` for fuzzy and prefix matching;
  - full text on tags;
  - filters as plain columns;
  - ranking by a `popularity` column, the number of channels using the emote, refreshed on a schedule.
- Only `approved`, listed emotes are searchable.
- **Move to Typesense or Meilisearch** when one of these is true:
  - p95 search latency is over 150 ms at normal load;
  - search uses over 30 % of database CPU;
  - typo tolerance or relevance needs exceed what trigrams give.
  
  Search goes behind a trait in `crates/db`, so swapping the engine doesn't touch handlers.

## Consequences
- No extra service in v1.
- Relevance is simpler than a dedicated engine.
