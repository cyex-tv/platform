-- Merging emotes (ADR 0040). Additive: three columns on `emotes`, an index, a permission.
--
-- A merged emote is retired: `deleted_at` is set (so every read, search and set write already
-- leaves it out) and `merged_into` names the emote that took its place, always the final one
-- (merging into an emote that was merged follows it, and emotes merged into the source are
-- pointed at the new target in the same transaction, so a chain is never longer than one step).
-- Reads of a merged ID answer the target: REST with a 301, GraphQL with `mergedInto`.
ALTER TABLE emotes
    ADD COLUMN merged_into uuid REFERENCES emotes (id),
    ADD COLUMN merged_at   timestamptz,
    ADD COLUMN merged_by   uuid REFERENCES accounts (id) ON DELETE SET NULL,
    ADD CONSTRAINT emotes_merged_not_self CHECK (merged_into IS DISTINCT FROM id),
    ADD CONSTRAINT emotes_merged_is_deleted CHECK (merged_into IS NULL OR deleted_at IS NOT NULL);

CREATE INDEX emotes_merged_into ON emotes (merged_into) WHERE merged_into IS NOT NULL;

-- emotes.merge: the built-in Admin role.
INSERT INTO role_permissions (role_id, permission)
SELECT r.id, 'emotes.merge' FROM roles r WHERE r.system = 'admin'
ON CONFLICT DO NOTHING;
