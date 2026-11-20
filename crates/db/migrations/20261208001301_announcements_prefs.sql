-- Site announcements, notification preferences and the public collection setting (2026/11/20).
-- Additive: two new tables and one permission for the built-in Admin role.

-- One line staff put under the header for everyone. `kind` sets its icon and tone. It shows from
-- `starts_at` (or at once) until `ends_at` (or until ended); `ended_at` is set when staff end it
-- early. The newest one that is live at a time is the one shown.
CREATE TABLE announcements (
    id uuid PRIMARY KEY,
    kind text NOT NULL CHECK (kind IN ('info', 'maintenance', 'campaign')),
    body text NOT NULL CHECK (char_length(body) BETWEEN 1 AND 200),
    link_url text CHECK (link_url IS NULL OR char_length(link_url) <= 500),
    link_label text CHECK (link_label IS NULL OR char_length(link_label) <= 40),
    emote_id uuid REFERENCES emotes (id) ON DELETE SET NULL,
    badge_id uuid REFERENCES badges (id) ON DELETE SET NULL,
    starts_at timestamptz,
    ends_at timestamptz,
    ended_at timestamptz,
    created_by uuid REFERENCES accounts (id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (starts_at IS NULL OR ends_at IS NULL OR ends_at > starts_at)
);
CREATE INDEX announcements_live ON announcements (created_at DESC) WHERE ended_at IS NULL;

-- What an account chose for itself. No row: the defaults (every notification on and popping up,
-- the collection private).
--   notify_off:      notification kinds that aren't created for this account at all.
--   toast_off:       kinds that are created but don't pop up (the website reads it).
--   show_collection: the owned badges and paints are on the public profile.
CREATE TABLE account_prefs (
    account_id uuid PRIMARY KEY REFERENCES accounts (id) ON DELETE CASCADE,
    notify_off text[] NOT NULL DEFAULT '{}',
    toast_off text[] NOT NULL DEFAULT '{}',
    show_collection boolean NOT NULL DEFAULT false,
    updated_at timestamptz NOT NULL DEFAULT now()
);

-- `site.announce`: set the site announcement. The built-in Admin role, which starts with every
-- permission, gets it; no other role changes.
INSERT INTO role_permissions (role_id, permission)
SELECT r.id, 'site.announce' FROM roles r WHERE r.system = 'admin'
ON CONFLICT DO NOTHING;
