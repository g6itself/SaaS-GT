-- Table des jeux (cross-platform)
CREATE TABLE IF NOT EXISTS games (
    id               UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title            TEXT NOT NULL,
    normalized_title TEXT NOT NULL,
    cover_image_url  TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Identifiants specifiques par plateforme
CREATE TABLE IF NOT EXISTS game_platform_ids (
    id                 UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    game_id            UUID NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    platform           platform_type NOT NULL,
    platform_game_id   TEXT NOT NULL,
    platform_name      TEXT,
    total_achievements INT NOT NULL DEFAULT 0,
    UNIQUE(platform, platform_game_id)
);

CREATE INDEX IF NOT EXISTS idx_game_platform_ids_game ON game_platform_ids(game_id);

-- Index trigram pour la recherche floue de jeux
CREATE INDEX IF NOT EXISTS idx_games_normalized_title ON games USING gin(normalized_title gin_trgm_ops);
