-- Definitions des achievements par jeu et plateforme
CREATE TABLE IF NOT EXISTS achievements (
    id                      UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    game_platform_id        UUID NOT NULL REFERENCES game_platform_ids(id) ON DELETE CASCADE,
    platform_achievement_id TEXT NOT NULL,
    name                    TEXT NOT NULL,
    description             TEXT,
    icon_url                TEXT,
    is_hidden               BOOLEAN NOT NULL DEFAULT false,
    global_unlock_pct       REAL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(game_platform_id, platform_achievement_id)
);

CREATE INDEX IF NOT EXISTS idx_achievements_game_platform ON achievements(game_platform_id);
