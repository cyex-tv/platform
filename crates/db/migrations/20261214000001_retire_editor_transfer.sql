-- Editors can't ask for a transfer any more (ADR 0014, amendment 2026/11/22): only an emote's
-- owner gives it away. Additive: no table, column or constraint changes.
--
-- - The retired `emote_transfer` flag comes off every grant that holds another flag too. A grant
--   holding only it keeps its row (removing an editor is the owner's call) and grants nothing:
--   the API ignores the name.
-- - Open transfer requests are withdrawn by the API at start-up (`transfers::withdraw_open`), not
--   here, so each one is audited and both sides are notified with the same IDs and payload the
--   API writes everywhere else.
-- Running it twice changes nothing.
UPDATE account_editors
SET permissions = array_remove(permissions, 'emote_transfer')
WHERE 'emote_transfer' = ANY (permissions)
  AND cardinality(permissions) > 1;
