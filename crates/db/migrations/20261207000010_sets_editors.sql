-- Set-scoped account editors and new editor flags (ADR 0014, amended 2026/11/19). Additive.

-- 1. New editor flags. `channels` changes channel settings; `sets` keeps allowing that too, so
-- existing grants mean what they meant. The emote, billing and subscription flags are stored for
-- the features that will check them.
ALTER TABLE account_editors
    DROP CONSTRAINT account_editors_permissions_check,
    ADD CONSTRAINT account_editors_permissions_check CHECK (
        cardinality(permissions) BETWEEN 1 AND 10
        AND permissions <@ ARRAY[
            'emotes', 'sets', 'editors', 'profile', 'channels',
            'emote_upload', 'emote_manage', 'emote_transfer',
            'billing_payment', 'billing_subscription'
        ]::text[]
    );

-- 2. Which sets an editor may work on: every set of the owner (`all_sets`, the default, new sets
-- included), or only the ones listed in `account_editor_sets`.
ALTER TABLE account_editors ADD COLUMN all_sets boolean NOT NULL DEFAULT true;

CREATE TABLE account_editor_sets (
    account_id uuid NOT NULL,
    editor_id  uuid NOT NULL,
    set_id     uuid NOT NULL REFERENCES emote_sets (id) ON DELETE CASCADE,
    PRIMARY KEY (account_id, editor_id, set_id),
    FOREIGN KEY (account_id, editor_id)
        REFERENCES account_editors (account_id, editor_id) ON DELETE CASCADE
);

CREATE INDEX account_editor_sets_set ON account_editor_sets (set_id);
