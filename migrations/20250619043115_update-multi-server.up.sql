-- helper function to get a discord server id on query-time to allow us to
-- query for an ID on DEFAULT clauses
CREATE OR REPLACE FUNCTION get_guild_id (guild_id_param bigint)
    RETURNS uuid
    AS $$
BEGIN
    RETURN (
        SELECT
            id
        FROM
            allowed_guilds
        WHERE
            guild_id = guild_id_param);
END;
$$
LANGUAGE plpgsql;

-- Adding guild_id column to allowed_roles and allowed_channels, using our helper function
ALTER TABLE allowed_roles
    ADD COLUMN guild_id uuid NOT NULL DEFAULT get_guild_id (258648784039313408);

ALTER TABLE allowed_channels
    ADD COLUMN guild_id uuid NOT NULL DEFAULT get_guild_id (258648784039313408);

-- Since we are defaulting to felps server, we need to update the guild_id for the test role
-- and test channel on the test server
UPDATE
    allowed_roles
SET
    guild_id = get_guild_id (1355012226355957780)
WHERE
    role_id = 1382199267115925515;

UPDATE
    allowed_channels
SET
    guild_id = get_guild_id (1355012226355957780)
WHERE
    channel_id = 1381783902338945217;

-- Dropping default values for guild_id as we will not be using DEFAULT on the
-- next roles created
ALTER TABLE allowed_roles
    ALTER COLUMN guild_id DROP DEFAULT;

ALTER TABLE allowed_roles
    ADD CONSTRAINT allowed_roles_guild_id_fkey FOREIGN KEY (guild_id) REFERENCES allowed_guilds (id) ON DELETE CASCADE;

ALTER TABLE allowed_channels
    ALTER COLUMN guild_id DROP DEFAULT;

ALTER TABLE allowed_channels
    ADD CONSTRAINT allowed_channels_guild_id_fkey FOREIGN KEY (guild_id) REFERENCES allowed_guilds (id) ON DELETE CASCADE;

