-- The "Subscriber" role (ADR 0038): holders get 10 personal set slots. It is public, holds no
-- permission beyond what everyone has, and sits just above the User role (position 0). No colour:
-- the colour is the owner's choice in the role editor. A role already called "Subscriber" is left
-- alone, so running this twice, or after someone made it by hand, changes nothing.
INSERT INTO roles (id, name, description, position, public, limits)
SELECT gen_random_uuid(), 'Subscriber', 'Supporters of cyex.tv. Ten personal set slots.', 1, true,
       '{"personal_set_slots": 10}'::jsonb
 WHERE NOT EXISTS (SELECT 1 FROM roles WHERE lower(name) = 'subscriber');
