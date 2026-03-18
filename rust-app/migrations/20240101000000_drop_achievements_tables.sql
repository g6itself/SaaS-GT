-- Migration : suppression des tables achievements et user_achievements
-- Ces données sont désormais récupérées directement depuis l'API Steam à l'affichage.
-- Seuls les totaux agrégés sont conservés dans users et user_game_stats.

-- Supprimer d'abord les triggers qui dépendent de user_achievements
DROP TRIGGER IF EXISTS trg_sync_points ON user_achievements;
DROP TRIGGER IF EXISTS trg_sync_achievements ON user_achievements;

-- Supprimer les tables (user_achievements référence achievements, donc user_achievements en premier)
DROP TABLE IF EXISTS user_achievements CASCADE;
DROP TABLE IF EXISTS achievements CASCADE;

-- Supprimer les colonnes de user_game_stats devenues inutiles
-- (les données temps de jeu et dernière partie viennent maintenant de l'API Steam)
ALTER TABLE user_game_stats
    DROP COLUMN IF EXISTS playtime_minutes,
    DROP COLUMN IF EXISTS last_played_at;

-- Supprimer la colonne cover_image_url de games (générée à la volée depuis Steam CDN)
ALTER TABLE games
    DROP COLUMN IF EXISTS cover_image_url;
