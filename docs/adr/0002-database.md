# 0002: Database

- Status: accepted
- Date: 2026/11/16

## Context
The data is relational: accounts, identities, sets, set members, roles and audit. It needs transactions (moving emotes between sets, approval plus event). Search (0004) and the job queue (0005, Phase 3) can live in the same database at v1 scale.

## Decision
- **Postgres 17** in its own container `tv-db`, with its own volume, credentials and Docker network. It is not `deck-db`, even when it runs on the same host.
- **sqlx** with checked queries. Migrations live in `crates/db/migrations`, run by the services on start, and are forward-only. A destructive change takes two steps: add and migrate, then remove in a later release.
- Times are `timestamptz`. IDs are ULIDs stored as `uuid`, so they sort by time and don't reveal counts.
- Each service has its own DB role with only the grants it needs:
  - `worker` gets the job table and `files`;
  - `events` gets read on the outbox;
  - `api` gets the rest.
- Pool size and `statement_timeout` are set per service from config.
- Backups: a nightly `pg_dump` in custom format, 14 days kept on the host, copied off the host, and a restore tested before launch (Phase 9).

## Consequences
- One database to run. Search and queue don't add services.
- Load from search and the queue shares the same instance, so watch it. 0004 says when search moves out.
