-- Public account activity feed: the account a public-facing audit row is about. Set for the
-- changes an account's owner or its account editors make to the account's sets, channels and
-- cosmetics, and for the approval of the account's emotes. Staff actions (moderation, grants)
-- never set it, so they can't reach the feed.
ALTER TABLE audit_log
    ADD COLUMN subject_account_id uuid REFERENCES accounts (id) ON DELETE SET NULL;

CREATE INDEX audit_log_subject ON audit_log (subject_account_id, id DESC)
    WHERE subject_account_id IS NOT NULL;
