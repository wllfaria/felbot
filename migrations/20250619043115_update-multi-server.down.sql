ALTER TABLE allowed_roles
    DROP CONSTRAINT IF EXISTS allowed_roles_guild_id_fkey;

ALTER TABLE allowed_channels
    DROP CONSTRAINT IF EXISTS allowed_channels_guild_id_fkey;

ALTER TABLE allowed_roles
    DROP COLUMN guild_id;

ALTER TABLE allowed_channels
    DROP COLUMN guild_id;

DROP FUNCTION IF EXISTS get_guild_id (bigint);

