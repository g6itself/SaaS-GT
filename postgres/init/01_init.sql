-- ─── Initialisation de la base de données — Achievement Tracker ──────────────
-- Schéma complet : utilisateurs, ligues, succès, plateformes, classement.
-- Ce script s'exécute automatiquement au premier démarrage du conteneur.
-- ─────────────────────────────────────────────────────────────────────────────

-- ── Extensions ───────────────────────────────────────────────────────────────

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ── Enums ────────────────────────────────────────────────────────────────────

-- Les 10 ligues dans l'ordre croissant
CREATE TYPE league_tier AS ENUM (
    'Orbit',
    'Horizon',
    'Nebula',
    'Pulsar',
    'Zenith',
    'Eclipse',
    'Supernova',
    'Apex',
    'Quasar',
    'Singularity'
);

-- Rareté des succès (détermine la valeur en points)
CREATE TYPE achievement_rarity AS ENUM (
    'common',       --    5 pts  | > 50% des joueurs
    'rare',         --   25 pts  | 10–50%
    'epic',         --  100 pts  | 1–10%
    'legendary',    --  500 pts  | 0.1–1%
    'mythic'        -- 2000 pts  | < 0.1%
);

-- Plateformes supportées
CREATE TYPE platform_type AS ENUM ('steam', 'gog', 'epic');

-- ── Fonctions utilitaires ─────────────────────────────────────────────────────

-- Mise à jour automatique de updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Calcul de la ligue depuis les points
-- Seuils calibrés pour qu'Orbit soit la majorité et Singularity < 0.01%.
CREATE OR REPLACE FUNCTION get_league(p BIGINT)
RETURNS league_tier AS $$
BEGIN
    RETURN CASE
        WHEN p <        1000 THEN 'Orbit'::league_tier
        WHEN p <        5000 THEN 'Horizon'::league_tier
        WHEN p <       15000 THEN 'Nebula'::league_tier
        WHEN p <       35000 THEN 'Pulsar'::league_tier
        WHEN p <       75000 THEN 'Zenith'::league_tier
        WHEN p <      150000 THEN 'Eclipse'::league_tier
        WHEN p <      300000 THEN 'Supernova'::league_tier
        WHEN p <      600000 THEN 'Apex'::league_tier
        WHEN p <     1000000 THEN 'Quasar'::league_tier
        ELSE                      'Singularity'::league_tier
    END;
END;
$$ LANGUAGE plpgsql IMMUTABLE STRICT;

-- Valeur en points d'une rareté
CREATE OR REPLACE FUNCTION rarity_points(r achievement_rarity)
RETURNS INTEGER AS $$
BEGIN
    RETURN CASE r
        WHEN 'common'    THEN    5
        WHEN 'rare'      THEN   25
        WHEN 'epic'      THEN  100
        WHEN 'legendary' THEN  500
        WHEN 'mythic'    THEN 2000
        ELSE 5
    END;
END;
$$ LANGUAGE plpgsql IMMUTABLE STRICT;

-- ── Table : users ─────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS users (
    id                UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    email             TEXT        NOT NULL UNIQUE
                                  CHECK (length(email) <= 320 AND email ~ '^[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}$'),
    username          TEXT        NOT NULL UNIQUE
                                  CHECK (length(username) BETWEEN 2 AND 32 AND username ~ '^[A-Za-z0-9_\-]+$'),
    password_hash     TEXT        NOT NULL DEFAULT '',
    display_name      TEXT        CHECK (display_name IS NULL OR length(display_name) BETWEEN 2 AND 32),
    -- Profil enrichi
    profile_image_url TEXT        CHECK (profile_image_url IS NULL OR length(profile_image_url) <= 2048),
    active_title      TEXT        NOT NULL DEFAULT 'Chasseur de Trophées'
                                  CHECK (length(active_title) <= 256),
    -- Système de points & ligues (mis à jour automatiquement par trigger)
    total_points      BIGINT      NOT NULL DEFAULT 0 CHECK (total_points >= 0),
    league            league_tier NOT NULL DEFAULT 'Orbit',
    -- Méta
    is_active         BOOLEAN     NOT NULL DEFAULT true,
    last_login_at     TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_users_email         ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_username      ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_league        ON users(league);
CREATE INDEX IF NOT EXISTS idx_users_total_points  ON users(total_points DESC);
CREATE INDEX IF NOT EXISTS idx_users_last_login    ON users(last_login_at DESC);

-- Trigger : updated_at
CREATE OR REPLACE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Trigger : synchroniser la ligue quand total_points change
CREATE OR REPLACE FUNCTION sync_user_league()
RETURNS TRIGGER AS $$
BEGIN
    NEW.league     := get_league(NEW.total_points);
    NEW.updated_at := NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_sync_league ON users;
CREATE TRIGGER trg_sync_league
    BEFORE UPDATE OF total_points ON users
    FOR EACH ROW
    EXECUTE FUNCTION sync_user_league();

-- ── Table : platform_connections ──────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS platform_connections (
    id                UUID          PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id           UUID          NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    platform          platform_type NOT NULL,
    platform_user_id  TEXT          NOT NULL,
    platform_username TEXT,
    access_token      TEXT,
    refresh_token     TEXT,
    token_expires_at  TIMESTAMPTZ,
    last_synced_at    TIMESTAMPTZ,
    created_at        TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, platform),
    UNIQUE (platform, platform_user_id)
);

CREATE INDEX IF NOT EXISTS idx_platform_connections_user ON platform_connections(user_id);

CREATE OR REPLACE TRIGGER update_platform_connections_updated_at
    BEFORE UPDATE ON platform_connections
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ── Table : games ─────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS games (
    id               UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    title            TEXT        NOT NULL CHECK (length(title) <= 512),
    normalized_title TEXT        NOT NULL CHECK (length(normalized_title) <= 512),
    cover_image_url  TEXT        CHECK (cover_image_url IS NULL OR length(cover_image_url) <= 2048),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_games_normalized_title ON games USING gin(normalized_title gin_trgm_ops);

CREATE OR REPLACE TRIGGER update_games_updated_at
    BEFORE UPDATE ON games
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ── Table : game_platform_ids ─────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS game_platform_ids (
    id                 UUID          PRIMARY KEY DEFAULT uuid_generate_v4(),
    game_id            UUID          NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    platform           platform_type NOT NULL,
    platform_game_id   TEXT          NOT NULL,
    platform_name      TEXT,
    total_achievements INT           NOT NULL DEFAULT 0,
    UNIQUE (platform, platform_game_id)
);

CREATE INDEX IF NOT EXISTS idx_game_platform_ids_game ON game_platform_ids(game_id);

-- ── Table : achievements ──────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS achievements (
    id                      UUID                PRIMARY KEY DEFAULT uuid_generate_v4(),
    game_platform_id        UUID                NOT NULL REFERENCES game_platform_ids(id) ON DELETE CASCADE,
    platform_achievement_id TEXT                NOT NULL CHECK (length(platform_achievement_id) <= 256),
    name                    TEXT                NOT NULL CHECK (length(name) <= 512),
    description             TEXT                CHECK (description IS NULL OR length(description) <= 2000),
    icon_url                TEXT                CHECK (icon_url IS NULL OR length(icon_url) <= 2048),
    is_hidden               BOOLEAN             NOT NULL DEFAULT false,
    global_unlock_pct       REAL,
    -- Rareté et points (points dénormalisé depuis rareté, géré par trigger)
    rarity                  achievement_rarity  NOT NULL DEFAULT 'common',
    points                  INTEGER             NOT NULL DEFAULT 5,
    created_at              TIMESTAMPTZ         NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ         NOT NULL DEFAULT NOW(),
    UNIQUE (game_platform_id, platform_achievement_id)
);

CREATE INDEX IF NOT EXISTS idx_achievements_game_platform ON achievements(game_platform_id);

CREATE OR REPLACE TRIGGER update_achievements_updated_at
    BEFORE UPDATE ON achievements
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Trigger : synchroniser achievements.points avec la rareté
CREATE OR REPLACE FUNCTION sync_achievement_points()
RETURNS TRIGGER AS $$
BEGIN
    NEW.points := rarity_points(NEW.rarity);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_sync_achievement_points ON achievements;
CREATE TRIGGER trg_sync_achievement_points
    BEFORE INSERT OR UPDATE OF rarity ON achievements
    FOR EACH ROW
    EXECUTE FUNCTION sync_achievement_points();

-- ── Table : user_achievements ─────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS user_achievements (
    id             UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id        UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    achievement_id UUID        NOT NULL REFERENCES achievements(id) ON DELETE CASCADE,
    unlocked_at    TIMESTAMPTZ,
    is_unlocked    BOOLEAN     NOT NULL DEFAULT false,
    synced_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, achievement_id)
);

CREATE INDEX IF NOT EXISTS idx_user_achievements_user     ON user_achievements(user_id);
CREATE INDEX IF NOT EXISTS idx_user_achievements_unlocked ON user_achievements(user_id, is_unlocked);

-- Trigger : recalculer total_points quand un succès est débloqué/retiré
CREATE OR REPLACE FUNCTION sync_user_points()
RETURNS TRIGGER AS $$
DECLARE
    v_points INTEGER;
BEGIN
    SELECT rarity_points(a.rarity)
    INTO   v_points
    FROM   achievements a
    WHERE  a.id = COALESCE(NEW.achievement_id, OLD.achievement_id);

    IF TG_OP = 'INSERT' AND NEW.is_unlocked THEN
        UPDATE users SET total_points = total_points + v_points WHERE id = NEW.user_id;

    ELSIF TG_OP = 'UPDATE' THEN
        IF NEW.is_unlocked AND NOT OLD.is_unlocked THEN
            UPDATE users SET total_points = total_points + v_points WHERE id = NEW.user_id;
        ELSIF NOT NEW.is_unlocked AND OLD.is_unlocked THEN
            UPDATE users SET total_points = GREATEST(0, total_points - v_points) WHERE id = NEW.user_id;
        END IF;

    ELSIF TG_OP = 'DELETE' AND OLD.is_unlocked THEN
        UPDATE users SET total_points = GREATEST(0, total_points - v_points) WHERE id = OLD.user_id;
    END IF;

    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_sync_points ON user_achievements;
CREATE TRIGGER trg_sync_points
    AFTER INSERT OR UPDATE OF is_unlocked OR DELETE ON user_achievements
    FOR EACH ROW
    EXECUTE FUNCTION sync_user_points();

-- ── Table : user_game_stats ───────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS user_game_stats (
    id                    UUID         PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id               UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    game_id               UUID         NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    achievements_unlocked INTEGER      NOT NULL DEFAULT 0,
    achievements_total    INTEGER      NOT NULL DEFAULT 0,
    completion_pct        NUMERIC(5,2) NOT NULL DEFAULT 0.00,
    playtime_minutes      INTEGER,
    last_played_at        TIMESTAMPTZ,
    updated_at            TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, game_id)
);

CREATE INDEX IF NOT EXISTS idx_user_game_stats_user ON user_game_stats(user_id);
CREATE INDEX IF NOT EXISTS idx_user_game_stats_pct  ON user_game_stats(user_id, completion_pct DESC);

-- ── Table : titles ────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS titles (
    id               UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    name             TEXT        NOT NULL UNIQUE,
    description      TEXT,
    unlock_condition TEXT,
    required_league  league_tier,
    required_points  BIGINT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── Table : user_titles ───────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS user_titles (
    user_id   UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title_id  UUID        NOT NULL REFERENCES titles(id) ON DELETE CASCADE,
    earned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, title_id)
);

CREATE INDEX IF NOT EXISTS idx_user_titles_user ON user_titles(user_id);

-- ── Table : leaderboard_cache ─────────────────────────────────────────────────
-- Recalculé périodiquement (cron) pour éviter les agrégats en temps réel.

CREATE TABLE IF NOT EXISTS leaderboard_cache (
    user_id            UUID         PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    rank_global        INTEGER,
    total_points       BIGINT       NOT NULL DEFAULT 0,
    total_achievements INTEGER      NOT NULL DEFAULT 0,
    completion_avg     NUMERIC(5,2) NOT NULL DEFAULT 0.00,
    league             league_tier  NOT NULL DEFAULT 'Orbit',
    refreshed_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_leaderboard_cache_rank ON leaderboard_cache(rank_global);

-- ── Données initiales : titres débloquables ───────────────────────────────────

INSERT INTO titles (name, description, unlock_condition, required_league, required_points) VALUES
    ('Chasseur de Trophées',  'Titre de départ',                           'Créer un compte',         NULL,          NULL),
    ('Explorateur',           'A connecté sa première plateforme',          'Connecter 1 plateforme',  NULL,          NULL),
    ('Collectionneur',        'A débloqué 100 succès',                     'Obtenir 100 succès',       'Horizon',     1000),
    ('Chasseur d''Élite',     'A atteint la ligue Nebula',                 'Atteindre Nebula',         'Nebula',      5000),
    ('Maître Platine',        'A obtenu 10 platines',                      'Obtenir 10 platines',      'Pulsar',      NULL),
    ('Légende',               'A atteint la ligue Zenith',                 'Atteindre Zenith',         'Zenith',      35000),
    ('Fantôme Numérique',     'A atteint la ligue Eclipse',                'Atteindre Eclipse',        'Eclipse',     75000),
    ('Supernova',             'A atteint la ligue Supernova',              'Atteindre Supernova',      'Supernova',   150000),
    ('Dieu des Succès',       'A atteint la ligue Apex',                   'Atteindre Apex',           'Apex',        300000),
    ('Singularité',           'Le titre le plus rare — ligue Singularity', 'Atteindre Singularity',   'Singularity', 1000000)
ON CONFLICT (name) DO NOTHING;
