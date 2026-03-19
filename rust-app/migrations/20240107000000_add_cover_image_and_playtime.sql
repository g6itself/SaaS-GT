-- Ajout cover_image_url dans game_platform_ids (image de couverture par plateforme)
ALTER TABLE game_platform_ids
    ADD COLUMN IF NOT EXISTS cover_image_url TEXT;

-- Ajout playtime_minutes dans user_game_stats (temps de jeu cumulé)
ALTER TABLE user_game_stats
    ADD COLUMN IF NOT EXISTS playtime_minutes INTEGER NOT NULL DEFAULT 0;
