-- ── Migration : index manquants + contrainte unique platform_connections ──────
--
-- Index ajoutés pour corriger les full-scans identifiés lors de l'audit :
-- 1. platform_connections.platform  — filtrage fréquent sur la plateforme
-- 2. user_game_stats.game_id        — JOIN fréquent avec game_platform_ids
-- 3. user_game_stats.user_id        — déjà couvert si FK existe, sinon ajouter
-- 4. game_platform_ids.(platform, game_id) — JOIN composite
--
-- Contrainte unique sur platform_connections.platform_user_id par plateforme
-- pour empêcher plusieurs comptes d'associer le même ID de joueur Steam/GOG.

-- Index sur platform_connections.platform (séparé de la PK)
CREATE INDEX IF NOT EXISTS idx_platform_connections_platform
    ON platform_connections (platform);

-- Index sur user_game_stats.game_id pour les JOINs
CREATE INDEX IF NOT EXISTS idx_user_game_stats_game_id
    ON user_game_stats (game_id);

-- Index composite user_game_stats (user_id, game_id) — requêtes typiques
CREATE INDEX IF NOT EXISTS idx_user_game_stats_user_game
    ON user_game_stats (user_id, game_id);

-- Index sur game_platform_ids pour les JOINs avec platform
CREATE INDEX IF NOT EXISTS idx_game_platform_ids_platform_game
    ON game_platform_ids (platform, game_id);

-- Index sur game_platform_ids.platform_game_id — recherche par appid Steam
CREATE INDEX IF NOT EXISTS idx_game_platform_ids_platform_game_id
    ON game_platform_ids (platform, platform_game_id);

-- Contrainte unique : un platform_user_id ne peut être lié qu'à un seul compte
-- par plateforme (ex: un SteamID64 ne peut appartenir qu'à un seul utilisateur).
-- Si la contrainte existe déjà sous un autre nom, DROP+CREATE.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'uq_platform_connections_platform_user_id'
    ) THEN
        ALTER TABLE platform_connections
            ADD CONSTRAINT uq_platform_connections_platform_user_id
            UNIQUE (platform, platform_user_id);
    END IF;
END $$;
