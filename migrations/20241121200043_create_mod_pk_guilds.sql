-- TODO: Check if we error on overflowing 9 223 372 036 854 775 807
CREATE TABLE mod_pk_guilds (
    guild_id BIGINT PRIMARY KEY,
    user_id BIGINT NOT NULL,
    token CHAR(64),

    UNIQUE(guild_id, user_id)
);
