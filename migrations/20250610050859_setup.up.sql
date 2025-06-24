CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE OR REPLACE FUNCTION set_updated_at ()
    RETURNS TRIGGER
    AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$
LANGUAGE plpgsql;

CREATE TABLE IF NOT EXISTS user_links (
    id uuid DEFAULT uuid_generate_v4 () PRIMARY KEY,
    discord_id bigint NOT NULL UNIQUE,
    telegram_id bigint NOT NULL UNIQUE,
    added_to_group_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW(),
    last_subscription_check timestamptz
);

CREATE TABLE IF NOT EXISTS oauth_states (
    id uuid DEFAULT uuid_generate_v4 () PRIMARY KEY,
    state_token varchar(36) NOT NULL,
    telegram_id bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW(),
    expires_at timestamptz NOT NULL DEFAULT NOW() + interval '15 minutes'
);

CREATE INDEX idx_user_links_discord_id ON user_links (discord_id);

CREATE INDEX idx_user_links_telegram_id ON user_links (telegram_id);

CREATE INDEX idx_oauth_states_expires ON oauth_states (expires_at);

CREATE INDEX idx_oauth_states_token ON oauth_states (state_token);

DO $$
DECLARE
    tbl RECORD;
BEGIN
    FOR tbl IN
    SELECT
        table_schema,
        table_name
    FROM
        information_schema.columns
    WHERE
        column_name = 'updated_at'
        AND table_schema = 'public' LOOP
            EXECUTE format('CREATE TRIGGER trg_set_updated_at
             BEFORE UPDATE ON %I.%I
             FOR EACH ROW
             EXECUTE FUNCTION set_updated_at();', tbl.table_schema, tbl.table_name);
        END LOOP;
END;
$$;

