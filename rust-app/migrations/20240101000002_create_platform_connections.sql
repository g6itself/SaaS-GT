-- Type enum pour les plateformes supportees
CREATE TYPE platform_type AS ENUM ('steam', 'gog', 'epic');

-- Connexions aux plateformes gaming
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

-- Trigger updated_at
CREATE OR REPLACE TRIGGER update_platform_connections_updated_at
    BEFORE UPDATE ON platform_connections
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
