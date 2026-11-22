-- Managing sets no longer brings channel settings with it (ADR 0014, amendment 2026/11/22).
-- Until now a grant of `sets` also counted as `channels`. From now on the two are separate, so
-- every grant that held `sets` without `channels` gets `channels` written down: nobody loses what
-- they could do. Grants of `channels` alone keep picking the active set (the API still allows it).
-- Additive, data only. Running it twice changes nothing.
UPDATE account_editors
SET permissions = array_append(permissions, 'channels')
WHERE 'sets' = ANY (permissions)
  AND NOT ('channels' = ANY (permissions));