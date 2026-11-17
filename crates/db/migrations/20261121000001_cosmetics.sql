-- Chat cosmetics (ADR 0015): badges and name paints, who owns them, and what each account shows.
-- Additive. Separate from cyex deck supporter badges (ADR 0011), which cyex.tv never stores.

-- Pictures for badges and image paints. Originals live at `originals/cosmetics/{sha256}` (never
-- public); the worker writes `cosmetics/{badges|paints}/{sha256}/{name}` and records them here.
CREATE TABLE cosmetic_images (
    id          uuid PRIMARY KEY,
    kind        text NOT NULL CHECK (kind IN ('badge', 'paint')),
    sha256      bytea NOT NULL CHECK (length(sha256) = 32),
    mime        text NOT NULL,
    width       integer NOT NULL CHECK (width > 0),
    height      integer NOT NULL CHECK (height > 0),
    frame_count integer NOT NULL CHECK (frame_count > 0),
    animated    boolean NOT NULL,
    storage_key text NOT NULL,
    -- The files the worker made, in the emote file shape. Empty until it ran.
    variants    jsonb NOT NULL DEFAULT '[]',
    created_by  uuid REFERENCES accounts (id) ON DELETE SET NULL,
    created_at  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT cosmetic_images_kind_sha256 UNIQUE (kind, sha256)
);

CREATE TABLE badges (
    id         uuid PRIMARY KEY,
    name       text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 32),
    tooltip    text NOT NULL CHECK (char_length(tooltip) BETWEEN 1 AND 64),
    image_id   uuid NOT NULL REFERENCES cosmetic_images (id),
    created_by uuid REFERENCES accounts (id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX badges_name ON badges (lower(name));

CREATE TABLE paints (
    id         uuid PRIMARY KEY,
    name       text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 32),
    kind       text NOT NULL CHECK (kind IN ('linear_gradient', 'radial_gradient', 'image')),
    angle      integer NOT NULL DEFAULT 90 CHECK (angle BETWEEN 0 AND 360),
    repeat     boolean NOT NULL DEFAULT false,
    shape      text NOT NULL DEFAULT 'circle' CHECK (shape IN ('circle', 'ellipse')),
    -- Shown where a client can't draw the paint: '#RRGGBB' or '#RRGGBBAA'.
    color      text CHECK (color ~ '^#[0-9A-F]{6}([0-9A-F]{2})?$'),
    -- [{ "at": 0..1, "color": "#RRGGBB(AA)" }], checked by the api.
    stops      jsonb NOT NULL DEFAULT '[]' CHECK (jsonb_typeof(stops) = 'array'),
    -- [{ "x": px, "y": px, "radius": px, "color": "#RRGGBB(AA)" }], at most 4.
    shadows    jsonb NOT NULL DEFAULT '[]' CHECK (jsonb_typeof(shadows) = 'array'),
    image_id   uuid REFERENCES cosmetic_images (id),
    created_by uuid REFERENCES accounts (id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK ((kind = 'image') = (image_id IS NOT NULL))
);
CREATE UNIQUE INDEX paints_name ON paints (lower(name));

-- Who owns what. `source` is how they got it; only 'grant' is written for now.
CREATE TABLE badge_entitlements (
    account_id uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    badge_id   uuid NOT NULL REFERENCES badges (id) ON DELETE CASCADE,
    source     text NOT NULL DEFAULT 'grant' CHECK (source IN ('grant', 'purchase', 'event')),
    reason     text,
    granted_by uuid REFERENCES accounts (id) ON DELETE SET NULL,
    granted_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, badge_id)
);
CREATE INDEX badge_entitlements_badge ON badge_entitlements (badge_id);

CREATE TABLE paint_entitlements (
    account_id uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    paint_id   uuid NOT NULL REFERENCES paints (id) ON DELETE CASCADE,
    source     text NOT NULL DEFAULT 'grant' CHECK (source IN ('grant', 'purchase', 'event')),
    reason     text,
    granted_by uuid REFERENCES accounts (id) ON DELETE SET NULL,
    granted_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, paint_id)
);
CREATE INDEX paint_entitlements_paint ON paint_entitlements (paint_id);

-- What an account shows. Only owned cosmetics: each column points into its entitlement table, so
-- a revoke (or deleting the cosmetic, which removes its entitlements) clears it at once.
CREATE TABLE cosmetic_selections (
    account_id uuid PRIMARY KEY REFERENCES accounts (id) ON DELETE CASCADE,
    badge_id   uuid,
    paint_id   uuid,
    updated_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (account_id, badge_id) REFERENCES badge_entitlements (account_id, badge_id)
        ON DELETE SET NULL (badge_id),
    FOREIGN KEY (account_id, paint_id) REFERENCES paint_entitlements (account_id, paint_id)
        ON DELETE SET NULL (paint_id)
);
CREATE INDEX cosmetic_selections_badge ON cosmetic_selections (badge_id) WHERE badge_id IS NOT NULL;
CREATE INDEX cosmetic_selections_paint ON cosmetic_selections (paint_id) WHERE paint_id IS NOT NULL;
