-- Snapshot du rang pour suivre la progression (hausse/baisse/stable)
ALTER TABLE users ADD COLUMN IF NOT EXISTS rank_snapshot      BIGINT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS rank_snapshot_at   TIMESTAMPTZ;
