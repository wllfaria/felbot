CREATE TABLE IF NOT EXISTS allowed_roles (
    id uuid DEFAULT uuid_generate_v4 () PRIMARY KEY,
    role_id bigint NOT NULL,
    name varchar(255) NOT NULL,
    is_admin boolean NOT NULL DEFAULT FALSE,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW()
);

INSERT INTO allowed_roles (role_id, name, is_admin)
    VALUES (277212035652124672, 'FELPS', TRUE),
    (258661569200652289, 'Admins', TRUE),
    (258661943966040064, 'Mods', FALSE),
    (649703184033513493, 'Subs da Twitch', FALSE),
    (689249333450899460, 'Twitch Subscriber: Tier 1', FALSE),
    (689249333450899467, 'Twitch Subscriber: Tier 2', FALSE),
    (689249333450899474, 'Twitch Subscriber: Tier 3', FALSE),
    (1244727194455117917, 'Membro do Youtube', FALSE),
    (1244727194455117916, 'Bapo+', FALSE),
    (1382199267115925515, 'Role Teste', TRUE);

CREATE TABLE IF NOT EXISTS allowed_channels (
    id uuid DEFAULT uuid_generate_v4 () PRIMARY KEY,
    channel_id bigint NOT NULL UNIQUE,
    name varchar(255) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW()
);

INSERT INTO allowed_channels (channel_id, name)
    VALUES (1140461199553740872, 'Chat dos subs');

CREATE TABLE IF NOT EXISTS allowed_guilds (
    id uuid DEFAULT uuid_generate_v4 () PRIMARY KEY,
    guild_id bigint NOT NULL,
    name varchar(255) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW()
);

INSERT INTO allowed_guilds (guild_id, name)
    VALUES (258648784039313408, 'Server do Felpinho'),
    (1355012226355957780, 'Server Teste');

