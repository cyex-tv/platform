-- Personal lists: what an account hides from its own view ("Hide for me", by emote or by
-- creator) and the emotes it marked as favourites. Additive: three new tables.
--
-- Private to the account: only its own session reads or changes them, nothing public joins them,
-- and the public reads stay the same for everyone (the website filters with the list it gets).
-- Caps (1000 hidden emotes, 200 hidden creators, 1000 favourites) are checked by the api under a
-- lock on the account row. Erasing an account removes its rows (`privacy::erase`); a hard-deleted
-- emote or account takes its rows with it.
CREATE TABLE hidden_emotes (
    account_id uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    emote_id   uuid NOT NULL REFERENCES emotes (id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, emote_id)
);

CREATE INDEX hidden_emotes_newest ON hidden_emotes (account_id, created_at DESC, emote_id);

CREATE TABLE hidden_creators (
    account_id uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    creator_id uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, creator_id),
    CHECK (account_id <> creator_id)
);

CREATE INDEX hidden_creators_newest ON hidden_creators (account_id, created_at DESC, creator_id);

CREATE TABLE favourite_emotes (
    account_id uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    emote_id   uuid NOT NULL REFERENCES emotes (id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, emote_id)
);

CREATE INDEX favourite_emotes_newest ON favourite_emotes (account_id, created_at DESC, emote_id);
