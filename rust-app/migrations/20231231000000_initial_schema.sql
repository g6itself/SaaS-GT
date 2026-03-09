-- ─── Schéma initial complet ───────────────────────────────────────────────────
-- Migration unique regroupant l'intégralité du schéma de l'application.
-- Remplace les 14 migrations individuelles précédentes.
--
-- DÉPLOIEMENT VIA L'APPLICATION RUST (recommandé) :
--   sqlx applique automatiquement toutes les migrations au démarrage.
--   Chaque migration est wrappée dans une transaction atomique par sqlx.
--
-- DÉPLOIEMENT MANUEL VIA PSQL (production / CI) :
--   psql -U <user> -d <dbname> --single-transaction \
--        -f 20231231000000_initial_schema.sql
--   Le flag --single-transaction garantit l'atomicité complète.
--
-- PRÉREQUIS POSTGRES :
--   PostgreSQL 14+ requis (CREATE OR REPLACE TRIGGER, NUMERIC, TIMESTAMPTZ).
--   L'utilisateur doit avoir les droits SUPERUSER ou accès à CREATE EXTENSION.
-- ─────────────────────────────────────────────────────────────────────────────


-- ══════════════════════════════════════════════════════════════════════════════
-- 1. EXTENSIONS
-- ══════════════════════════════════════════════════════════════════════════════

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";  -- uuid_generate_v4()
CREATE EXTENSION IF NOT EXISTS "pg_trgm";    -- gin_trgm_ops (recherche floue)


-- ══════════════════════════════════════════════════════════════════════════════
-- 2. FONCTIONS UTILITAIRES
-- ══════════════════════════════════════════════════════════════════════════════

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;


-- ══════════════════════════════════════════════════════════════════════════════
-- 3. TYPES ÉNUMÉRÉS
-- ══════════════════════════════════════════════════════════════════════════════

DO $$ BEGIN
    CREATE TYPE platform_type AS ENUM ('steam', 'gog', 'epic');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE league_tier AS ENUM (
        'Orbit', 'Horizon', 'Nebula', 'Pulsar', 'Zenith',
        'Eclipse', 'Supernova', 'Apex', 'Quasar', 'Singularity'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE achievement_rarity AS ENUM (
        'common', 'rare', 'epic', 'legendary', 'mythic'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;


-- ══════════════════════════════════════════════════════════════════════════════
-- 4. FONCTIONS DOMAINE
-- ══════════════════════════════════════════════════════════════════════════════

-- Retourne la ligue correspondant à un total de points
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

-- Retourne les points associés à une rareté de succès
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


-- ══════════════════════════════════════════════════════════════════════════════
-- 5. TABLE USERS
-- ══════════════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS users (
    id                          UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    username                    TEXT        NOT NULL CONSTRAINT users_username_unique UNIQUE,
    email                       TEXT        NOT NULL UNIQUE,
    password_hash               TEXT        NOT NULL DEFAULT '',
    display_name                TEXT,
    avatar_url                  TEXT,
    is_active                   BOOLEAN     NOT NULL DEFAULT true,
    profile_image_url           TEXT,
    active_title                TEXT        NOT NULL DEFAULT 'Chasseur de Trophées',
    total_points                BIGINT      NOT NULL DEFAULT 0,
    league                      league_tier NOT NULL DEFAULT 'Orbit',
    last_login_at               TIMESTAMPTZ,
    steam_api_key_enc           TEXT,
    total_achievements_count    BIGINT      NOT NULL DEFAULT 0,
    games_completed             INTEGER     NOT NULL DEFAULT 0,
    total_possible_achievements BIGINT      NOT NULL DEFAULT 0,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Trigger : updated_at automatique
DROP TRIGGER IF EXISTS update_users_updated_at ON users;
CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Trigger : synchronisation de la ligue quand total_points change
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


-- ══════════════════════════════════════════════════════════════════════════════
-- 6. CONNEXIONS PLATEFORMES
-- ══════════════════════════════════════════════════════════════════════════════

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
    UNIQUE(user_id, platform),
    UNIQUE(platform, platform_user_id)
);

DROP TRIGGER IF EXISTS update_platform_connections_updated_at ON platform_connections;
CREATE TRIGGER update_platform_connections_updated_at
    BEFORE UPDATE ON platform_connections
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();


-- ══════════════════════════════════════════════════════════════════════════════
-- 7. JEUX
-- ══════════════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS games (
    id               UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    title            TEXT        NOT NULL,
    normalized_title TEXT        NOT NULL,
    cover_image_url  TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS game_platform_ids (
    id                 UUID          PRIMARY KEY DEFAULT uuid_generate_v4(),
    game_id            UUID          NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    platform           platform_type NOT NULL,
    platform_game_id   TEXT          NOT NULL,
    platform_name      TEXT,
    total_achievements INT           NOT NULL DEFAULT 0,
    UNIQUE(platform, platform_game_id)
);


-- ══════════════════════════════════════════════════════════════════════════════
-- 8. ACHIEVEMENTS
-- ══════════════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS achievements (
    id                      UUID               PRIMARY KEY DEFAULT uuid_generate_v4(),
    game_platform_id        UUID               NOT NULL REFERENCES game_platform_ids(id) ON DELETE CASCADE,
    platform_achievement_id TEXT               NOT NULL,
    name                    TEXT               NOT NULL,
    description             TEXT,
    icon_url                TEXT,
    is_hidden               BOOLEAN            NOT NULL DEFAULT false,
    global_unlock_pct       REAL,
    rarity                  achievement_rarity NOT NULL DEFAULT 'common',
    points                  INTEGER            NOT NULL DEFAULT 5,
    created_at              TIMESTAMPTZ        NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ        NOT NULL DEFAULT NOW(),
    UNIQUE(game_platform_id, platform_achievement_id)
);

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


-- ══════════════════════════════════════════════════════════════════════════════
-- 9. USER_ACHIEVEMENTS
-- ══════════════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS user_achievements (
    id             UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id        UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    achievement_id UUID        NOT NULL REFERENCES achievements(id) ON DELETE CASCADE,
    unlocked_at    TIMESTAMPTZ,
    is_unlocked    BOOLEAN     NOT NULL DEFAULT false,
    synced_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, achievement_id)
);

-- Trigger : total_points via rareté des achievements
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

-- Trigger : total_achievements_count
CREATE OR REPLACE FUNCTION sync_user_achievement_count()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.is_unlocked THEN
            UPDATE users SET total_achievements_count = total_achievements_count + 1 WHERE id = NEW.user_id;
        END IF;
    ELSIF TG_OP = 'UPDATE' THEN
        IF NEW.is_unlocked AND NOT OLD.is_unlocked THEN
            UPDATE users SET total_achievements_count = total_achievements_count + 1 WHERE id = NEW.user_id;
        ELSIF NOT NEW.is_unlocked AND OLD.is_unlocked THEN
            UPDATE users SET total_achievements_count = GREATEST(0, total_achievements_count - 1) WHERE id = NEW.user_id;
        END IF;
    ELSIF TG_OP = 'DELETE' THEN
        IF OLD.is_unlocked THEN
            UPDATE users SET total_achievements_count = GREATEST(0, total_achievements_count - 1) WHERE id = OLD.user_id;
        END IF;
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_sync_achievement_count ON user_achievements;
CREATE TRIGGER trg_sync_achievement_count
    AFTER INSERT OR UPDATE OF is_unlocked OR DELETE ON user_achievements
    FOR EACH ROW
    EXECUTE FUNCTION sync_user_achievement_count();


-- ══════════════════════════════════════════════════════════════════════════════
-- 10. TITRES DÉBLOQUABLES
-- ══════════════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS titles (
    id               UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    name             TEXT        NOT NULL UNIQUE,
    description      TEXT,
    unlock_condition TEXT,
    required_league  league_tier,
    required_points  BIGINT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS user_titles (
    user_id   UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title_id  UUID        NOT NULL REFERENCES titles(id) ON DELETE CASCADE,
    earned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, title_id)
);


-- ══════════════════════════════════════════════════════════════════════════════
-- 11. STATISTIQUES PAR JEU
-- ══════════════════════════════════════════════════════════════════════════════

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

-- Trigger : games_completed
CREATE OR REPLACE FUNCTION sync_user_games_completed()
RETURNS TRIGGER AS $$
DECLARE
    v_was_completed BOOLEAN;
    v_is_completed  BOOLEAN;
BEGIN
    IF TG_OP = 'DELETE' THEN
        v_was_completed := OLD.achievements_total > 0
                       AND OLD.achievements_unlocked >= OLD.achievements_total;
        IF v_was_completed THEN
            UPDATE users SET games_completed = GREATEST(0, games_completed - 1) WHERE id = OLD.user_id;
        END IF;
        RETURN OLD;
    END IF;

    v_is_completed := NEW.achievements_total > 0
                  AND NEW.achievements_unlocked >= NEW.achievements_total;

    IF TG_OP = 'INSERT' THEN
        IF v_is_completed THEN
            UPDATE users SET games_completed = games_completed + 1 WHERE id = NEW.user_id;
        END IF;
    ELSIF TG_OP = 'UPDATE' THEN
        v_was_completed := OLD.achievements_total > 0
                       AND OLD.achievements_unlocked >= OLD.achievements_total;
        IF v_is_completed AND NOT v_was_completed THEN
            UPDATE users SET games_completed = games_completed + 1 WHERE id = NEW.user_id;
        ELSIF NOT v_is_completed AND v_was_completed THEN
            UPDATE users SET games_completed = GREATEST(0, games_completed - 1) WHERE id = NEW.user_id;
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_sync_games_completed ON user_game_stats;
CREATE TRIGGER trg_sync_games_completed
    AFTER INSERT OR UPDATE OF achievements_unlocked, achievements_total OR DELETE
    ON user_game_stats
    FOR EACH ROW
    EXECUTE FUNCTION sync_user_games_completed();

-- Trigger : total_possible_achievements
CREATE OR REPLACE FUNCTION sync_user_possible_achievements()
RETURNS TRIGGER AS $$
DECLARE
    v_delta BIGINT;
BEGIN
    IF TG_OP = 'DELETE' THEN
        UPDATE users
        SET total_possible_achievements = GREATEST(0, total_possible_achievements - OLD.achievements_total)
        WHERE id = OLD.user_id;
        RETURN OLD;
    END IF;

    IF TG_OP = 'INSERT' THEN
        v_delta := NEW.achievements_total;
    ELSE
        v_delta := NEW.achievements_total - OLD.achievements_total;
    END IF;

    IF v_delta <> 0 THEN
        UPDATE users
        SET total_possible_achievements = GREATEST(0, total_possible_achievements + v_delta)
        WHERE id = NEW.user_id;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_sync_possible_achievements ON user_game_stats;
CREATE TRIGGER trg_sync_possible_achievements
    AFTER INSERT OR UPDATE OF achievements_total OR DELETE
    ON user_game_stats
    FOR EACH ROW
    EXECUTE FUNCTION sync_user_possible_achievements();


-- ══════════════════════════════════════════════════════════════════════════════
-- 12. LEADERBOARD CACHE
-- ══════════════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS leaderboard_cache (
    user_id            UUID         PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    rank_global        INTEGER,
    total_points       BIGINT       NOT NULL DEFAULT 0,
    total_achievements INTEGER      NOT NULL DEFAULT 0,
    completion_avg     NUMERIC(5,2) NOT NULL DEFAULT 0.00,
    league             league_tier  NOT NULL DEFAULT 'Orbit',
    refreshed_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);


-- ══════════════════════════════════════════════════════════════════════════════
-- 13. CATALOGUE RÉTRO
-- ══════════════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS retro_consoles (
    id           SERIAL      PRIMARY KEY,
    name         TEXT        NOT NULL,
    manufacturer TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_retro_consoles_name ON retro_consoles(name);

CREATE TABLE IF NOT EXISTS retro_games (
    id              SERIAL      PRIMARY KEY,
    console_id      INTEGER     NOT NULL REFERENCES retro_consoles(id) ON DELETE CASCADE,
    name            TEXT        NOT NULL,
    description     TEXT,
    cover_image_url TEXT,
    release_date    DATE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_retro_games_console_name ON retro_games(console_id, name);
CREATE INDEX        IF NOT EXISTS idx_retro_games_console_id   ON retro_games(console_id);

CREATE TABLE IF NOT EXISTS retro_game_covers (
    id         SERIAL      PRIMARY KEY,
    game_id    INTEGER     NOT NULL REFERENCES retro_games(id) ON DELETE CASCADE,
    region     TEXT        NOT NULL,
    url        TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_retro_game_covers_game_region ON retro_game_covers(game_id, region);
CREATE INDEX        IF NOT EXISTS idx_retro_game_covers_game_id     ON retro_game_covers(game_id);


-- ══════════════════════════════════════════════════════════════════════════════
-- 14. INDEX
-- ══════════════════════════════════════════════════════════════════════════════

-- users
CREATE INDEX IF NOT EXISTS idx_users_league       ON users(league);
CREATE INDEX IF NOT EXISTS idx_users_total_points ON users(total_points DESC);
CREATE INDEX IF NOT EXISTS idx_users_last_login   ON users(last_login_at DESC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email ON users(email);

-- platform_connections
CREATE INDEX IF NOT EXISTS idx_platform_connections_user ON platform_connections(user_id);

-- games
CREATE INDEX IF NOT EXISTS idx_game_platform_ids_game ON game_platform_ids(game_id);
CREATE INDEX IF NOT EXISTS idx_games_normalized_title  ON games USING gin(normalized_title gin_trgm_ops);

-- achievements
CREATE INDEX IF NOT EXISTS idx_achievements_game_platform ON achievements(game_platform_id);

-- user_achievements
CREATE INDEX IF NOT EXISTS idx_user_achievements_user     ON user_achievements(user_id);
CREATE INDEX IF NOT EXISTS idx_user_achievements_unlocked ON user_achievements(user_id, is_unlocked);

-- user_titles
CREATE INDEX IF NOT EXISTS idx_user_titles_user ON user_titles(user_id);

-- user_game_stats
CREATE INDEX IF NOT EXISTS idx_user_game_stats_user ON user_game_stats(user_id);
CREATE INDEX IF NOT EXISTS idx_user_game_stats_pct  ON user_game_stats(user_id, completion_pct DESC);

-- leaderboard_cache
CREATE INDEX IF NOT EXISTS idx_leaderboard_cache_rank ON leaderboard_cache(rank_global);


-- ══════════════════════════════════════════════════════════════════════════════
-- 15. DONNÉES DE RÉFÉRENCE
-- ══════════════════════════════════════════════════════════════════════════════

-- ── Titres du système ─────────────────────────────────────────────────────────

INSERT INTO titles (name, description, unlock_condition, required_league, required_points) VALUES
    ('Chasseur de Trophées',  'Titre de départ',                           'Créer un compte',        NULL,           NULL),
    ('Explorateur',           'A connecté sa première plateforme',          'Connecter 1 plateforme', NULL,           NULL),
    ('Collectionneur',        'A débloqué 100 succès',                     'Obtenir 100 succès',     'Horizon',      1000),
    ('Chasseur d''Élite',     'A atteint la ligue Nebula',                 'Atteindre Nebula',       'Nebula',       5000),
    ('Maître Platine',        'A obtenu 10 platines',                      'Obtenir 10 platines',    'Pulsar',       NULL),
    ('Légende',               'A atteint la ligue Zenith',                 'Atteindre Zenith',       'Zenith',       35000),
    ('Fantôme Numérique',     'A atteint la ligue Eclipse',                'Atteindre Eclipse',      'Eclipse',      75000),
    ('Supernova',             'A atteint la ligue Supernova',              'Atteindre Supernova',    'Supernova',    150000),
    ('Dieu des Succès',       'A atteint la ligue Apex',                   'Atteindre Apex',         'Apex',         300000),
    ('Singularité',           'Le titre le plus rare — ligue Singularity', 'Atteindre Singularity',  'Singularity',  1000000)
ON CONFLICT (name) DO NOTHING;

-- ── Consoles rétro ────────────────────────────────────────────────────────────

INSERT INTO retro_consoles (name, manufacturer) VALUES
    ('32X',                                         'SEGA'),
    ('3DO',                                         '3DO Company'),
    ('Amiga CD32',                                  'Commodore'),
    ('Atari 2600',                                  'Atari'),
    ('Atari 5200',                                  'Atari'),
    ('Atari 7800',                                  'Atari'),
    ('CD-i',                                        'Philips'),
    ('CDTV',                                        'Commodore'),
    ('ColecoVision',                                'CBS'),
    ('Dreamcast',                                   'SEGA'),
    ('FM Towns Marty',                              'Fujitsu'),
    ('Game Boy / Pocket / Color',                   'Nintendo'),
    ('Game Boy Advance / SP',                       'Nintendo'),
    ('GameCube',                                    'Nintendo'),
    ('Game Gear',                                   'SEGA'),
    ('Gizmondo',                                    'Tiger Telematics'),
    ('GP32',                                        'GamePark'),
    ('GX4000',                                      'Amstrad'),
    ('Intellivision',                               'Mattel'),
    ('Atari Jaguar',                                'Atari'),
    ('Atari Lynx',                                  'Atari'),
    ('Master System',                               'SEGA'),
    ('Mega-CD',                                     'SEGA'),
    ('Mega Drive / Genesis',                        'SEGA'),
    ('Konix Multi-System',                          'Konix'),
    ('Multi-Mega',                                  'SEGA'),
    ('N-Gage',                                      'Nokia'),
    ('PC Engine CD-ROM² / Super CD-ROM²',           'NEC'),
    ('Neo Geo AES',                                 'SNK'),
    ('Neo Geo CD / CDZ',                            'SNK'),
    ('Neo Geo Pocket',                              'SNK'),
    ('Neptune',                                     'SEGA'),
    ('NES / Famicom',                               'Nintendo'),
    ('Nintendo 64',                                 'Nintendo'),
    ('Nintendo DSi',                                'Nintendo'),
    ('Nomad',                                       'SEGA'),
    ('Magnavox Odyssey',                            'Magnavox'),
    ('Odyssey 2 / Videopac',                        'Philips'),
    ('PC Engine / CoreGrafx',                       'NEC'),
    ('PC Engine GT',                                'NEC'),
    ('PC Engine LT',                                'NEC'),
    ('Pippin',                                      'Bandai / Apple'),
    ('Playdia',                                     'Bandai'),
    ('PlayStation',                                 'Sony'),
    ('PlayStation 2',                               'Sony'),
    ('PSP',                                         'Sony'),
    ('Saturn',                                      'SEGA'),
    ('Sega Genesis 3',                              'SEGA'),
    ('SG-1000',                                     'SEGA'),
    ('Super Cassette Vision',                       'Yeno / Epoch'),
    ('Super NES / Super Famicom',                   'Nintendo'),
    ('SuperGrafx',                                  'NEC'),
    ('Atari XEGS',                                  'Atari'),
    ('Vectrex',                                     'Milton Bradley'),
    ('Virtual Boy',                                 'Nintendo'),
    ('WonderMega',                                  'SEGA'),
    ('WonderSwan',                                  'Bandai'),
    ('Xbox',                                        'Microsoft')
ON CONFLICT (name) DO NOTHING;

-- ── Jeux Sega 32X ────────────────────────────────────────────────────────────

INSERT INTO retro_games (console_id, name, description)
SELECT rc.id, g.name, g.description
FROM retro_consoles rc
CROSS JOIN (VALUES
    ('After Burner Complete',
     'Prenez les commandes du F-14 Thunder Cat dans l''expérience d''arcade la plus fidèle jamais réalisée. Verrouillez vos cibles et déchaînez une puissance de feu dévastatrice à 60 images par seconde.'),
    ('BC Racers',
     'Lancez-vous dans une course préhistorique effrénée ! 32 circuits truffés de pièges où seule la loi du plus fort (et du plus rapide) compte.'),
    ('Blackthorne',
     'Incarnez Kyle Vlaros, un mercenaire armé d''un fusil à pompe, dans une quête pour reprendre son trône sur une planète sombre. Un action-platformer aux animations d''un réalisme saisissant.'),
    ('Brutal: Paws of Fury Special Edition',
     'Apprenez la philosophie des arts martiaux avec les maîtres les plus sauvages du monde animal. Une édition spéciale boostée par la puissance du 32X.'),
    ('Cosmic Carnage',
     'Le combat pour la survie commence aux confins de la galaxie. Utilisez des armures destructibles et des zooms spectaculaires pour écraser vos adversaires extraterrestres.'),
    ('Darxide',
     'L''humanité est au bord de l''extinction. Pilotez votre chasseur en 3D totale et détruisez les astéroïdes et les bases ennemies avant qu''il ne soit trop tard.'),
    ('Doom',
     'Le FPS légendaire arrive sur 32X. Explorez des bases martiennes infestées de démons avec une vitesse et une fluidité de calcul sans précédent.'),
    ('FIFA Soccer 96',
     'Entrez dans le stade virtuel ! Avec la technologie Virtual Stadium, vivez le football avec une liberté de mouvement totale et des statistiques réelles.'),
    ('Golf Magazine: 36 Holes Starring Fred Couples',
     'Le réalisme du golf professionnel entre vos mains. 36 trous numérisés et les conseils de Fred Couples pour parfaire votre swing.'),
    ('Knuckles'' Chaotix',
     'L''aventure Sonic la plus innovante ! Utilisez l''anneau de puissance pour créer un effet élastique entre vos personnages et atteindre des vitesses incroyables.'),
    ('Kolibri',
     'Incarnez le protecteur de la nature. Un shoot''em up unique aux graphismes luxuriants où vous devez sauver l''écosystème terrestre d''une menace de l''espace.'),
    ('Metal Head',
     'Prenez le contrôle d''une machine de guerre 3D texturée. Traversez des villes en proie au chaos et remplissez vos missions de pacification armée.'),
    ('Mortal Kombat II',
     'Rien n''est plus réel. Le kombat ultime revient avec des graphismes et des sons directement issus de la borne d''arcade. Finish Him !'),
    ('Motocross Championship',
     'La boue, la vitesse et la gloire. Affrontez les meilleurs pilotes mondiaux sur des circuits accidentés où chaque saut peut être le dernier.'),
    ('NBA Jam Tournament Edition',
     'Il est en feu ! Découvrez le basket-ball d''arcade avec des dunks qui défient les lois de la physique et des bonus délirants.'),
    ('NFL Quarterback Club',
     'Devenez une légende de la NFL. Gérez votre équipe et exécutez les jeux les plus complexes grâce à une précision graphique 32 bits.'),
    ('Pitfall: The Mayan Adventure',
     'Aidez Harry Jr. à secourir son père dans la jungle maya. Un voyage périlleux aux animations de qualité cinéma.'),
    ('Primal Rage',
     'Les dieux primordiaux s''affrontent pour le contrôle de la Terre. Choisissez votre créature et dévorez vos fidèles pour regagner de l''énergie.'),
    ('RBI Baseball ''95',
     'La simulation de baseball la plus complète, incluant les vrais joueurs, les vraies équipes et des graphismes photo-réalistes.'),
    ('Sangokushi IV',
     'Écrivez l''histoire de la Chine ancienne. Un jeu de stratégie épique où la diplomatie est aussi tranchante que l''épée.'),
    ('Shadow Squadron',
     'Pilotez l''élite des chasseurs stellaires. Engagez-vous dans des combats spatiaux en 3D totale pour protéger la flotte de l''alliance.'),
    ('Space Harrier',
     'Bienvenue dans la Zone Fantastique ! Un classique de l''arcade porté avec une perfection absolue : volez, tirez et survivez.'),
    ('Spider-Man: Web of Fire',
     'L''homme-araignée fait équipe avec Daredevil pour déjouer les plans de l''HYDRA. Une action non-stop dans un univers de comics vibrant.'),
    ('Star Trek: Starfleet Academy',
     'Cadet, votre formation commence ici. Commandez les vaisseaux les plus célèbres de Starfleet dans des simulations de combat spatial intenses.'),
    ('Star Wars Arcade',
     'Que la Force soit avec vous. Revivez l''attaque de l''Étoile de la Mort et les batailles spatiales iconiques de la saga Star Wars en 3D.'),
    ('T-Mek',
     'Le sport de combat du futur. Pilotez des tanks aéroglisseurs et éliminez vos rivaux dans des arènes closes ultra-rapides.'),
    ('Tempo',
     'Laissez-vous emporter par le rythme ! Un jeu de plateforme musical où chaque mouvement de Tempo fait vibrer l''écran de couleurs.'),
    ('Toughman Contest',
     'Entrez sur le ring pour le tournoi de boxe amateur le plus sauvage. Esquivez, frappez et devenez le champion du Toughman.'),
    ('Virtua Fighter',
     'L''évolution du combat. La technologie d''arcade Model 1 arrive chez vous : des combats en 3D d''une fluidité et d''une technicité inégalées.'),
    ('Virtua Racing Deluxe',
     'La course automobile redéfinie. De nouvelles voitures, de nouveaux circuits et une sensation de vitesse que seul le 32X peut offrir.'),
    ('World Series Baseball ''95',
     'Le coup de circuit assuré. Une immersion totale sur le terrain avec des angles de vue dynamiques et une jouabilité parfaite.'),
    ('Wrestlemania: The Arcade Game',
     'C''est bien plus que du catch. C''est de l''action pure où les superstars de la WWF utilisent des pouvoirs surhumains pour la victoire.'),
    ('Zaxxon''s Motherbase 2000',
     'Le futur de Zaxxon est arrivé. Infiltrez la base mère ennemie dans ce shooter isométrique révolutionnaire mêlant 2D et 3D.'),
    -- ── Exclusifs Sega CD-32X ─────────────────────────────────────────────────
    ('Corpse Killer',
     'Une île tropicale, des zombies et votre fidèle fusil. Vivez un film d''action interactif avec une qualité d''image cinématographique 32 bits.'),
    ('Fahrenheit',
     'Face aux flammes, chaque seconde compte. Un thriller interactif où vous incarnez un pompier d''élite au cœur du brasier.'),
    ('Night Trap',
     'Protégez les invités de la maison à l''aide de caméras cachées et de pièges. La version ultime du jeu le plus controversé de l''histoire.'),
    ('Slam City with Scottie Pippen',
     'Défiez Scottie Pippen sur son propre terrain. Du basket-ball de rue en vidéo plein écran avec une réactivité instantanée.'),
    ('Supreme Warrior',
     'Maîtrisez les secrets du Kung-Fu. Affrontez des guerriers légendaires dans des combats filmés où seul votre timing vous sauvera.'),
    ('Surgical Strike',
     'Aux commandes d''un véhicule d''assaut high-tech, infiltrez le territoire ennemi pour une frappe chirurgicale. L''action FMV à son apogée.')
) AS g(name, description)
WHERE rc.name = '32X'
ON CONFLICT (console_id, name) DO NOTHING;

-- ── Jaquettes Sega 32X ───────────────────────────────────────────────────────

INSERT INTO retro_game_covers (game_id, region, url)
SELECT rg.id, c.region, c.url
FROM retro_consoles rc
JOIN retro_games rg ON rg.console_id = rc.id
JOIN (VALUES
    ('After Burner Complete',   'Europe',      'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/After%20Burner%20Complete%20(Europe).png'),
    ('After Burner Complete',   'Japan / USA', 'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/After%20Burner%20Complete%20(Japan%2C%20USA)%20(En).png'),
    ('BC Racers',               'USA',         'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/BC%20Racers%20(USA).png'),
    ('Blackthorne',             'USA',         'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Blackthorne%20(USA).png'),
    ('Brutal: Paws of Fury Special Edition', 'USA', 'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Brutal%20-%20Above%20the%20Claw%20(USA).png'),
    ('Cosmic Carnage',          'Europe',      'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Cosmic%20Carnage%20(Europe).png'),
    ('Cosmic Carnage',          'Japan / USA', 'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Cosmic%20Carnage%20(Japan%2C%20USA)%20(En).png'),
    ('Darxide',                 'Europe',      'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Darxide%20(Europe)%20(En%2CFr%2CDe%2CEs).png'),
    ('Doom',                    'Europe',      'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Doom%20(Europe).png'),
    ('Doom',                    'Japan / USA', 'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Doom%20(Japan%2C%20USA)%20(En).png'),
    ('FIFA Soccer 96',          'Europe',      'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/FIFA%20Soccer%2096%20(Europe)%20(En%2CFr%2CDe%2CEs%2CIt%2CSv).png'),
    ('Golf Magazine: 36 Holes Starring Fred Couples', 'Europe',      'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Golf%20Magazine%20Presents%20-%2036%20Great%20Holes%20Starring%20Fred%20Couples%20(Europe).png'),
    ('Golf Magazine: 36 Holes Starring Fred Couples', 'Japan / USA', 'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Golf%20Magazine%20Presents%20-%2036%20Great%20Holes%20Starring%20Fred%20Couples%20(Japan%2C%20USA)%20(En).png'),
    ('Knuckles'' Chaotix',      'Europe',      'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Knuckles%27%20Chaotix%20(Europe).png'),
    ('Knuckles'' Chaotix',      'Japan / USA', 'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Knuckles%27%20Chaotix%20(Japan%2C%20USA)%20(En).png'),
    ('Kolibri',                 'USA / Europe','https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Kolibri%20(USA%2C%20Europe).png'),
    ('Metal Head',              'Europe',      'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Metal%20Head%20(Europe)%20(En%2CJa).png'),
    ('Metal Head',              'Japan / USA', 'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Metal%20Head%20(Japan%2C%20USA)%20(En%2CJa).png'),
    ('Mortal Kombat II',        'Europe',      'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Mortal%20Kombat%20II%20(Europe).png'),
    ('Mortal Kombat II',        'Japan / USA', 'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Mortal%20Kombat%20II%20(Japan%2C%20USA)%20(En).png'),
    ('Motocross Championship',  'Europe',      'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Motocross%20Championship%20(Europe).png'),
    ('Motocross Championship',  'USA',         'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Motocross%20Championship%20(USA).png'),
    ('NBA Jam Tournament Edition','World',     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/NBA%20Jam%20-%20Tournament%20Edition%20(World).png'),
    ('NFL Quarterback Club',    'World',       'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/NFL%20Quarterback%20Club%20(World).png'),
    ('Pitfall: The Mayan Adventure','USA',     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Pitfall%20-%20The%20Mayan%20Adventure%20(USA).png'),
    ('Primal Rage',             'USA / Europe','https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Primal%20Rage%20(USA%2C%20Europe).png'),
    ('RBI Baseball ''95',       'USA',         'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/RBI%20Baseball%20%2795%20(USA).png'),
    ('Sangokushi IV',           'Japan',       'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Sangokushi%20IV%20(Japan).png'),
    ('Shadow Squadron',         'Japan',       'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Stellar%20Assault%20(Japan).png'),
    ('Shadow Squadron',         'USA / Europe','https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Stellar%20Assault%20(USA%2C%20Europe).png'),
    ('Space Harrier',           'Europe',      'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Space%20Harrier%20(Europe).png'),
    ('Space Harrier',           'Japan / USA', 'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Space%20Harrier%20(Japan%2C%20USA)%20(En).png'),
    ('Spider-Man: Web of Fire', 'USA',         'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Amazing%20Spider-Man%2C%20The%20-%20Web%20of%20Fire%20(USA).png'),
    ('Star Trek: Starfleet Academy','USA',     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Star%20Trek%20-%20Starfleet%20Academy%20-%20Starship%20Bridge%20Simulator%20(USA).png'),
    ('Star Wars Arcade',        'Europe',      'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Star%20Wars%20Arcade%20(Europe).png'),
    ('Star Wars Arcade',        'Japan',       'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Star%20Wars%20Arcade%20(Japan).png'),
    ('Star Wars Arcade',        'USA',         'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Star%20Wars%20Arcade%20(USA).png'),
    ('T-Mek',                   'USA / Europe','https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/T-MEK%20(USA%2C%20Europe).png'),
    ('Tempo',                   'Japan / USA', 'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Tempo%20(Japan%2C%20USA)%20(En).png'),
    ('Toughman Contest',        'USA / Europe','https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Toughman%20Contest%20(USA%2C%20Europe).png'),
    ('Virtua Fighter',          'Europe',      'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Virtua%20Fighter%20(Europe).png'),
    ('Virtua Fighter',          'Japan / USA', 'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Virtua%20Fighter%20(Japan%2C%20USA)%20(En).png'),
    ('Virtua Racing Deluxe',    'Europe',      'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Virtua%20Racing%20Deluxe%20(Europe).png'),
    ('Virtua Racing Deluxe',    'Japan',       'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Virtua%20Racing%20Deluxe%20(Japan).png'),
    ('Virtua Racing Deluxe',    'USA',         'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Virtua%20Racing%20Deluxe%20(USA).png'),
    ('World Series Baseball ''95','USA',       'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/World%20Series%20Baseball%20Starring%20Deion%20Sanders%20(USA).png'),
    ('Wrestlemania: The Arcade Game','USA',    'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/WWF%20WrestleMania%20-%20The%20Arcade%20Game%20(USA).png'),
    ('Zaxxon''s Motherbase 2000','Japan / USA','https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Zaxxon%27s%20Motherbase%202000%20(Japan%2C%20USA)%20(En).png')
) AS c(game_name, region, url) ON rg.name = c.game_name
WHERE rc.name = '32X'
ON CONFLICT (game_id, region) DO UPDATE SET url = EXCLUDED.url;
