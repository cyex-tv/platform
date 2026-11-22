-- Console tokens (ADR 0028): a personal Bearer token a signed-in person mints on the website
-- (`POST /v1/auth/token/export`, `createConsoleToken`) to use the API as themselves, for example
-- from a GraphQL playground or a script. Additive: a third session kind and one index.
--
-- - A `console` session is a row in `sessions` like the others, so it shows in `mySessions`, can
--   be signed out there, and ends with "sign out everywhere", account deletion and blocking.
-- - It names no app client (the existing CHECK keeps `app_client_id` for `app` sessions) and never
--   slides its expiry: it ends 90 days after it was made.
-- - One per account: minting a new one deletes the old one in the same transaction, and the unique
--   index makes a second one impossible even under a race.
ALTER TABLE sessions DROP CONSTRAINT sessions_kind_check;
ALTER TABLE sessions ADD CONSTRAINT sessions_kind_check
    CHECK (kind IN ('site', 'app', 'console'));

CREATE UNIQUE INDEX sessions_one_console ON sessions (account_id) WHERE kind = 'console';
