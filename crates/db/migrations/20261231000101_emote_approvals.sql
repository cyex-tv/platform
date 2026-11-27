-- Two approvals per emote: public listing and personal use (ADR 0038). Additive: new columns with
-- defaults, data filled in from what each emote was, two permissions, and a trigger that keeps
-- `unlisted` following the listing approval.

ALTER TABLE emotes
    ADD COLUMN listing_review text NOT NULL DEFAULT 'pending'
        CONSTRAINT emotes_listing_review CHECK (listing_review IN ('pending', 'approved', 'denied')),
    ADD COLUMN personal_review text NOT NULL DEFAULT 'pending'
        CONSTRAINT emotes_personal_review CHECK (personal_review IN ('pending', 'approved', 'denied')),
    ADD COLUMN personal_reviewed_by uuid REFERENCES accounts (id) ON DELETE SET NULL,
    ADD COLUMN personal_reviewed_at timestamptz,
    ADD COLUMN personal_review_reason text;

-- Existing emotes:
-- - pending: both pending (the defaults);
-- - rejected or taken down: both denied;
-- - approved and listed (public) or private: listing approved, and personal use approved (it was
--   publicly approved; the owner's default);
-- - approved and unlisted (staff took it off the listings, or approved it "for personal use
--   only"): listing denied, personal use pending, so staff look at it again in the personal queue.
UPDATE emotes SET listing_review = 'denied', personal_review = 'denied'
WHERE state IN ('rejected', 'taken_down');
UPDATE emotes SET listing_review = 'approved', personal_review = 'approved',
                  personal_reviewed_by = reviewed_by, personal_reviewed_at = reviewed_at
WHERE state = 'approved' AND NOT unlisted;
UPDATE emotes SET listing_review = 'denied'
WHERE state = 'approved' AND unlisted;

-- `unlisted` follows the listing approval: an emote is unlisted unless it is private or its
-- listing was approved. The one place this rule lives, so no write can list an emote that
-- wasn't approved for listing. Never both private and unlisted (emotes_one_visibility).
CREATE FUNCTION emotes_listing_follows_review() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    NEW.unlisted := NOT NEW.private AND NEW.listing_review <> 'approved';
    RETURN NEW;
END
$$;

CREATE TRIGGER emotes_listing_follows_review
    BEFORE INSERT OR UPDATE ON emotes
    FOR EACH ROW EXECUTE FUNCTION emotes_listing_follows_review();

-- Pending public uploads are unlisted from now on, like new ones (they weren't listed anyway:
-- listings take approved emotes only).
UPDATE emotes SET unlisted = (NOT private AND listing_review <> 'approved')
WHERE unlisted <> (NOT private AND listing_review <> 'approved');

-- The personal queue: approved emotes whose personal use is pending.
CREATE INDEX emotes_personal_queue ON emotes (created_at, id)
    WHERE personal_review = 'pending' AND state = 'approved' AND deleted_at IS NULL;

-- Permissions (ADR 0038):
-- - emotes.review_personal: every role that allows or denies emotes.review says the same for it,
--   so no reviewer loses anything and no denied one gains anything;
-- - emotes.transfer: the built-in Admin role.
INSERT INTO role_permissions (role_id, permission, effect)
SELECT role_id, 'emotes.review_personal', effect FROM role_permissions
WHERE permission = 'emotes.review'
ON CONFLICT DO NOTHING;

INSERT INTO role_permissions (role_id, permission)
SELECT r.id, 'emotes.transfer' FROM roles r WHERE r.system = 'admin'
ON CONFLICT DO NOTHING;
