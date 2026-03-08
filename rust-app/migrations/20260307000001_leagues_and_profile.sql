-- ─── Migration : Ligues, Profil & Système de Points ──────────────────────────
-- Ajoute le système de ligues, les champs profil enrichis, la rareté des
-- succès, le calcul automatique des points, et les titres débloquables.
-- ─────────────────────────────────────────────────────────────────────────────

-- ── 1. Enums ─────────────────────────────────────────────────────────────────

-- Les 10 ligues dans l'ordre croissant
DO $$ BEGIN
    CREATE TYPE league_tier AS ENUM (
        'Orbit', 'Horizon', 'Nebula', 'Pulsar', 'Zenith',
        'Eclipse', 'Supernova', 'Apex', 'Quasar', 'Singularity'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- Rareté des succès (détermine la valeur en points)
DO $$ BEGIN
    CREATE TYPE achievement_rarity AS ENUM (
        'common', 'rare', 'epic', 'legendary', 'mythic'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- ── 2. Extension table users ─────────────────────────────────────────────────

-- Image de profil (URL relative ou CDN)
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS profile_image_url TEXT;

-- Titre actif affiché sous le pseudo
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS active_title       TEXT NOT NULL DEFAULT 'Chasseur de Trophées';

-- Points cumulés (mis à jour automatiquement)
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS total_points       BIGINT NOT NULL DEFAULT 0;

-- Ligue calculée depuis total_points (mise à jour automatiquement)
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS league             league_tier NOT NULL DEFAULT 'Orbit';

-- Dernière connexion (pour tri d'activité)
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS last_login_at      TIMESTAMPTZ;

-- Unicité du username (renforcement)
DO $$ BEGIN
    ALTER TABLE users ADD CONSTRAINT users_username_unique UNIQUE (username);
EXCEPTION WHEN duplicate_table THEN NULL;
END $$;

-- ── 3. Extension table achievements ─────────────────────────────────────────

-- Rareté explicite (peut venir de la plateforme ou être calculée)
ALTER TABLE achievements
    ADD COLUMN IF NOT EXISTS rarity achievement_rarity NOT NULL DEFAULT 'common';

-- Points attribués à l'obtention (dénormalisé pour la perf)
ALTER TABLE achievements
    ADD COLUMN IF NOT EXISTS points INTEGER NOT NULL DEFAULT 5;

-- ── 4. Table des titres débloquables ─────────────────────────────────────────

CREATE TABLE IF NOT EXISTS titles (
    id               UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name             TEXT NOT NULL UNIQUE,
    description      TEXT,
    -- Condition de déblocage (ex: "Atteindre la ligue Apex")
    unlock_condition TEXT,
    -- Ligue minimale requise (NULL = pas de contrainte de ligue)
    required_league  league_tier,
    -- Points minimaux requis (NULL = pas de contrainte de points)
    required_points  BIGINT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Titres gagnés par les utilisateurs
CREATE TABLE IF NOT EXISTS user_titles (
    user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title_id  UUID NOT NULL REFERENCES titles(id) ON DELETE CASCADE,
    earned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, title_id)
);

CREATE INDEX IF NOT EXISTS idx_user_titles_user ON user_titles(user_id);

-- ── 5. Statistiques par jeu (complétion, temps de jeu) ───────────────────────

CREATE TABLE IF NOT EXISTS user_game_stats (
    id                      UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                 UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    game_id                 UUID NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    achievements_unlocked   INTEGER NOT NULL DEFAULT 0,
    achievements_total      INTEGER NOT NULL DEFAULT 0,
    completion_pct          NUMERIC(5,2) NOT NULL DEFAULT 0.00,
    playtime_minutes        INTEGER,
    last_played_at          TIMESTAMPTZ,
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, game_id)
);

CREATE INDEX IF NOT EXISTS idx_user_game_stats_user    ON user_game_stats(user_id);
CREATE INDEX IF NOT EXISTS idx_user_game_stats_pct     ON user_game_stats(user_id, completion_pct DESC);

-- ── 6. Leaderboard snapshot (cache) ─────────────────────────────────────────
-- Recalculé périodiquement (cron / Cloud Scheduler) pour éviter les agrégats
-- en temps réel sur une table potentiellement volumineuse.

CREATE TABLE IF NOT EXISTS leaderboard_cache (
    user_id              UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    rank_global          INTEGER,
    total_points         BIGINT  NOT NULL DEFAULT 0,
    total_achievements   INTEGER NOT NULL DEFAULT 0,
    completion_avg       NUMERIC(5,2) NOT NULL DEFAULT 0.00,
    league               league_tier NOT NULL DEFAULT 'Orbit',
    refreshed_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_leaderboard_cache_rank ON leaderboard_cache(rank_global);

-- ── 7. Nouveaux index sur users ──────────────────────────────────────────────

CREATE INDEX IF NOT EXISTS idx_users_league        ON users(league);
CREATE INDEX IF NOT EXISTS idx_users_total_points  ON users(total_points DESC);
CREATE INDEX IF NOT EXISTS idx_users_username      ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_last_login    ON users(last_login_at DESC);

-- ── 8. Fonction : calcul de la ligue depuis les points ───────────────────────
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

-- ── 9. Fonction : valeur en points d'un rarity ───────────────────────────────

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

-- ── 10. Trigger : mettre à jour league quand total_points change ─────────────

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

-- ── 11. Trigger : recalculer points utilisateur quand un succès est débloqué ─
-- Incrémente/décrémente total_points sur users lors de l'insertion ou
-- de la mise à jour de user_achievements.

CREATE OR REPLACE FUNCTION sync_user_points()
RETURNS TRIGGER AS $$
DECLARE
    v_points INTEGER;
BEGIN
    -- Récupère la valeur en points du succès concerné
    SELECT rarity_points(a.rarity)
    INTO   v_points
    FROM   achievements a
    WHERE  a.id = COALESCE(NEW.achievement_id, OLD.achievement_id);

    IF TG_OP = 'INSERT' AND NEW.is_unlocked THEN
        UPDATE users SET total_points = total_points + v_points WHERE id = NEW.user_id;

    ELSIF TG_OP = 'UPDATE' THEN
        IF NEW.is_unlocked AND NOT OLD.is_unlocked THEN
            -- Succès nouvellement débloqué
            UPDATE users SET total_points = total_points + v_points WHERE id = NEW.user_id;
        ELSIF NOT NEW.is_unlocked AND OLD.is_unlocked THEN
            -- Succès retiré (sync correction)
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

-- ── 12. Trigger : synchroniser achievements.points avec la rareté ────────────

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

-- ── 13. Titres initiaux du système ───────────────────────────────────────────

INSERT INTO titles (name, description, unlock_condition, required_league, required_points) VALUES
    ('Chasseur de Trophées',  'Titre de départ',                          'Créer un compte',                    NULL,           NULL),
    ('Explorateur',           'A connecté sa première plateforme',         'Connecter 1 plateforme',             NULL,           NULL),
    ('Collectionneur',        'A débloqué 100 succès',                    'Obtenir 100 succès',                 'Horizon',      1000),
    ('Chasseur d''Élite',     'A atteint la ligue Nebula',                'Atteindre Nebula',                   'Nebula',       5000),
    ('Maître Platine',        'A obtenu 10 platines',                     'Obtenir 10 platines',                'Pulsar',       NULL),
    ('Légende',               'A atteint la ligue Zenith',                'Atteindre Zenith',                   'Zenith',       35000),
    ('Fantôme Numérique',     'A atteint la ligue Eclipse',               'Atteindre Eclipse',                  'Eclipse',      75000),
    ('Supernova',             'A atteint la ligue Supernova',             'Atteindre Supernova',                'Supernova',    150000),
    ('Dieu des Succès',       'A atteint la ligue Apex',                  'Atteindre Apex',                     'Apex',         300000),
    ('Singularité',           'Le titre le plus rare — ligue Singularity','Atteindre Singularity',              'Singularity',  1000000)
ON CONFLICT (name) DO NOTHING;
