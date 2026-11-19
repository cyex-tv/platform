-- Account deletion with a 30-day undo window (ADR 0019, owner's answer of 2026/11/19). Additive:
-- one nullable column and a partial index.
--
-- `deleteAccount` sets `deleted_at` (so every public read hides the account at once, as it hides
-- a deleted one) and `erase_after` (now + 30 days). Until then the account keeps its identities,
-- emotes, sets and grants, and signing in again offers to restore it, which clears both. When
-- `erase_after` has passed, the worker erases it as `deleteAccount` used to at once, and clears
-- `erase_after`. A row with `deleted_at` set and `erase_after` NULL is erased for good, as every
-- row deleted before this migration is.
ALTER TABLE accounts ADD COLUMN erase_after timestamptz;
ALTER TABLE accounts
    ADD CONSTRAINT accounts_erase_after_deleted CHECK (erase_after IS NULL OR deleted_at IS NOT NULL);

-- The worker's hourly look for accounts due to be erased.
CREATE INDEX accounts_erase_due ON accounts (erase_after) WHERE erase_after IS NOT NULL;
