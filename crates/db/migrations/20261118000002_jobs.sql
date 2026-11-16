-- Phase 3: the worker's job queue (PLAN.md Phase 3 task 2). Workers claim jobs with
-- `FOR UPDATE SKIP LOCKED`, hold a lease while they run, and retry with backoff. A job whose
-- worker died is claimed again once its lease runs out. Inserting a job notifies `cyex_tv_jobs`,
-- so idle workers wake at once instead of waiting for their next poll.

CREATE TABLE jobs (
    id           bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    kind         text NOT NULL CHECK (kind ~ '^[a-z][a-z0-9_.]{0,63}$'),
    payload      jsonb NOT NULL DEFAULT '{}',
    state        text NOT NULL DEFAULT 'queued'
                 CHECK (state IN ('queued', 'running', 'done', 'failed')),
    attempts     integer NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    max_attempts integer NOT NULL DEFAULT 5 CHECK (max_attempts BETWEEN 1 AND 100),
    run_after    timestamptz NOT NULL DEFAULT now(),
    locked_until timestamptz,
    last_error   text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);

-- What a worker looks for: queued jobs that are due, and running ones whose lease ran out.
CREATE INDEX jobs_due ON jobs (run_after) WHERE state = 'queued';
CREATE INDEX jobs_leased ON jobs (locked_until) WHERE state = 'running';

CREATE FUNCTION jobs_notify() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_notify('cyex_tv_jobs', NEW.kind);
    RETURN NULL;
END;
$$;

CREATE TRIGGER jobs_notify AFTER INSERT ON jobs FOR EACH ROW EXECUTE FUNCTION jobs_notify();
