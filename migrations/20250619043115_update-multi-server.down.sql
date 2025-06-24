DROP TABLE telegram_groups;

ALTER TABLE allowed_roles
    DROP CONSTRAINT IF EXISTS allowed_roles_guild_id_fkey;

ALTER TABLE allowed_channels
    DROP CONSTRAINT IF EXISTS allowed_channels_guild_id_fkey;

ALTER TABLE allowed_roles
    DROP COLUMN guild_id;

ALTER TABLE allowed_channels
    DROP COLUMN guild_id;

DROP FUNCTION IF EXISTS get_guild_id (bigint);

DROP TABLE IF EXISTS telegram_groups;

DELETE FROM allowed_guilds
WHERE guild_id = 1125142982052560957;

ALTER TABLE user_links
    ADD COLUMN added_to_group_at timestamptz;

ALTER TABLE user_links
    ADD COLUMN last_subscription_check timestamptz;

ALTER TABLE oauth_states
    DROP COLUMN group_name;

ALTER TABLE allowed_guilds
    DROP COLUMN OWNER;

