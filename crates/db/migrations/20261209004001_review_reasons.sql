-- The review queue's ready reject reasons (additive). Staff with `review.settings` change the
-- list on /admin/review/reasons; the reject dialog offers them in `position` order. Seeded with
-- the reasons the web used to carry in its code.
CREATE TABLE review_reasons (
    position   integer PRIMARY KEY CHECK (position BETWEEN 0 AND 99),
    body       text NOT NULL CHECK (char_length(body) BETWEEN 1 AND 200),
    updated_by uuid REFERENCES accounts (id) ON DELETE SET NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO review_reasons (position, body) VALUES
    (0, 'Low quality or blurry'),
    (1, 'Already uploaded (a duplicate)'),
    (2, 'Not an emote'),
    (3, 'NSFW but not marked'),
    (4, 'Hateful or harassing'),
    (5, 'The name breaks the rules');

-- `review.settings` (ADR 0010 amendment): who may change the list. The built-in Admin role, which
-- starts with every permission, gets it; no other role changes.
INSERT INTO role_permissions (role_id, permission)
SELECT r.id, 'review.settings' FROM roles r WHERE r.system = 'admin'
ON CONFLICT DO NOTHING;
