-- Ajout du champ last_seen_at pour le statut en ligne/hors ligne
ALTER TABLE users ADD COLUMN IF NOT EXISTS last_seen_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_users_last_seen_at ON users(last_seen_at);

-- Initialiser avec last_login_at pour les utilisateurs existants
UPDATE users SET last_seen_at = last_login_at WHERE last_seen_at IS NULL AND last_login_at IS NOT NULL;
