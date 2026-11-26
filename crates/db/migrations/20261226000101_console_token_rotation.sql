-- Console tokens rotate (ADR 0029): one lasts 3 days from when it was made, and signing any
-- session out (one or all) ends it. Additive: existing tokens are cut to 3 days from when they
-- were made (one older than that stops working now), and a CHECK keeps every console session
-- within 3 days, so a longer one can't be written by mistake.
UPDATE sessions
SET expires_at = least(expires_at, created_at + interval '3 days')
WHERE kind = 'console';

ALTER TABLE sessions ADD CONSTRAINT sessions_console_three_days
    CHECK (kind <> 'console' OR expires_at <= created_at + interval '3 days');
