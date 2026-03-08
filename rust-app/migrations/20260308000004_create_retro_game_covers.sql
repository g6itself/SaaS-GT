-- ─── Jaquettes par région ─────────────────────────────────────────────────────
-- Plusieurs jaquettes peuvent exister pour un même jeu (USA, Europe, Japan…)
CREATE TABLE IF NOT EXISTS retro_game_covers (
    id         SERIAL PRIMARY KEY,
    game_id    INTEGER     NOT NULL REFERENCES retro_games(id) ON DELETE CASCADE,
    region     TEXT        NOT NULL, -- ex: 'USA', 'Europe', 'Japan', 'Japan / USA', 'World'
    url        TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_retro_game_covers_game_region
    ON retro_game_covers(game_id, region);

CREATE INDEX IF NOT EXISTS idx_retro_game_covers_game_id
    ON retro_game_covers(game_id);