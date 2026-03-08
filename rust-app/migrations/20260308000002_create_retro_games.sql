-- ─── Table des jeux rétro ─────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS retro_games (
    id               SERIAL PRIMARY KEY,
    console_id       INTEGER      NOT NULL REFERENCES retro_consoles(id) ON DELETE CASCADE,
    name             TEXT         NOT NULL,
    description      TEXT,
    cover_image_url  TEXT,
    release_date     DATE,
    created_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_retro_games_console_name
    ON retro_games(console_id, name);

CREATE INDEX IF NOT EXISTS idx_retro_games_console_id
    ON retro_games(console_id);