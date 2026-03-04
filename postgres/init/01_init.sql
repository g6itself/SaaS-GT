-- ─── Initialisation de la base de donnees ────────────────────────────────────
-- Ce script s'execute automatiquement au premier demarrage du conteneur.

-- Extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- ─── Table utilisateurs ─────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS users (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email         TEXT NOT NULL UNIQUE,
    username      TEXT NOT NULL,
    password_hash TEXT NOT NULL DEFAULT '',
    display_name  TEXT,
    avatar_url    TEXT,
    is_active     BOOLEAN NOT NULL DEFAULT true,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

-- ─── Type enum plateformes ──────────────────────────────────────────────────
CREATE TYPE platform_type AS ENUM ('steam', 'gog', 'epic');

-- ─── Connexions aux plateformes ─────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS platform_connections (
    id                UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    platform          platform_type NOT NULL,
    platform_user_id  TEXT NOT NULL,
    platform_username TEXT,
    access_token      TEXT,
    refresh_token     TEXT,
    token_expires_at  TIMESTAMPTZ,
    last_synced_at    TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, platform),
    UNIQUE(platform, platform_user_id)
);

CREATE INDEX IF NOT EXISTS idx_platform_connections_user ON platform_connections(user_id);

-- ─── Jeux ───────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS games (
    id               UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title            TEXT NOT NULL,
    normalized_title TEXT NOT NULL,
    cover_image_url  TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

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
CREATE INDEX IF NOT EXISTS idx_games_normalized_title ON games USING gin(normalized_title gin_trgm_ops);

-- ─── Achievements ───────────────────────────────────────────────────────────
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

-- ─── Achievements utilisateurs ──────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS user_achievements (
    id             UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id        UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    achievement_id UUID NOT NULL REFERENCES achievements(id) ON DELETE CASCADE,
    unlocked_at    TIMESTAMPTZ,
    is_unlocked    BOOLEAN NOT NULL DEFAULT false,
    synced_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, achievement_id)
);

CREATE INDEX IF NOT EXISTS idx_user_achievements_user ON user_achievements(user_id);
CREATE INDEX IF NOT EXISTS idx_user_achievements_unlocked ON user_achievements(user_id, is_unlocked);

-- ─── Trigger : met a jour updated_at automatiquement ────────────────────────
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE OR REPLACE TRIGGER update_platform_connections_updated_at
    BEFORE UPDATE ON platform_connections
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ─── Donnees de test ────────────────────────────────────────────────────────
INSERT INTO users (email, username) VALUES
    ('test@example.com', 'utilisateur_test')
ON CONFLICT (email) DO NOTHING;
