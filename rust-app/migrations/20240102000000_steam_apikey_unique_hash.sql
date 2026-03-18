-- Migration : unicité de la clé API Steam par compte
-- Ajoute une colonne de hash SHA-256 de la clé pour permettre la vérification
-- d'unicité sans avoir à déchiffrer toutes les clés existantes.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS steam_api_key_hash TEXT DEFAULT NULL;

-- Contrainte UNIQUE : une clé API Steam ne peut être associée qu'à un seul compte.
-- NULL est autorisé (UNIQUE n'applique pas sur NULL en PostgreSQL).
CREATE UNIQUE INDEX IF NOT EXISTS uq_users_steam_api_key_hash
    ON users (steam_api_key_hash)
    WHERE steam_api_key_hash IS NOT NULL;
