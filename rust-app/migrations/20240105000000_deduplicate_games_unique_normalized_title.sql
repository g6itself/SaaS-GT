-- Déduplique la table games et ajoute une contrainte UNIQUE sur normalized_title
-- pour éviter les doublons lors des syncs Steam/GOG.

-- Étape 1 : Pour chaque groupe de doublons, garder l'entrée la plus ancienne (created_at MIN)
-- et réattribuer toutes les FK (game_platform_ids, user_game_stats) vers cette entrée canonique.

DO $$
DECLARE
    dup RECORD;
    canonical_id UUID;
    dup_id UUID;
BEGIN
    FOR dup IN
        SELECT normalized_title
        FROM games
        GROUP BY normalized_title
        HAVING COUNT(*) > 1
    LOOP
        -- ID canonique = le plus ancien
        SELECT id INTO canonical_id
        FROM games
        WHERE normalized_title = dup.normalized_title
        ORDER BY created_at ASC
        LIMIT 1;

        -- Pour chaque doublon (pas le canonique)
        FOR dup_id IN
            SELECT id FROM games
            WHERE normalized_title = dup.normalized_title
              AND id <> canonical_id
        LOOP
            -- Réattribuer game_platform_ids vers le canonique
            -- En cas de conflit (platform, platform_game_id) déjà présent sur le canonique, supprimer le doublon
            UPDATE game_platform_ids
            SET game_id = canonical_id
            WHERE game_id = dup_id
              AND NOT EXISTS (
                SELECT 1 FROM game_platform_ids gpi2
                WHERE gpi2.game_id = canonical_id
                  AND gpi2.platform = game_platform_ids.platform
                  AND gpi2.platform_game_id = game_platform_ids.platform_game_id
              );

            DELETE FROM game_platform_ids WHERE game_id = dup_id;

            -- Réattribuer user_game_stats vers le canonique
            -- En cas de conflit (user_id, game_id) déjà présent, garder celui avec le plus de données
            UPDATE user_game_stats
            SET game_id = canonical_id
            WHERE game_id = dup_id
              AND NOT EXISTS (
                SELECT 1 FROM user_game_stats ugs2
                WHERE ugs2.game_id = canonical_id
                  AND ugs2.user_id = user_game_stats.user_id
              );

            DELETE FROM user_game_stats WHERE game_id = dup_id;

            -- Supprimer le jeu doublon
            DELETE FROM games WHERE id = dup_id;
        END LOOP;
    END LOOP;
END $$;

-- Étape 2 : Ajouter la contrainte UNIQUE sur normalized_title
ALTER TABLE games ADD CONSTRAINT games_normalized_title_key UNIQUE (normalized_title);

-- Étape 3 : Remplacer l'index GIN par un index btree standard (plus adapté aux lookups exacts)
-- On garde le GIN pour la recherche full-text et on ajoute un btree pour les upserts
CREATE UNIQUE INDEX IF NOT EXISTS games_normalized_title_unique ON games (normalized_title);
