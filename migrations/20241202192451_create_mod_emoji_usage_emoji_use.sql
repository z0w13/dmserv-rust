-- TODO: Check if we error on overflowing 9 223 372 036 854 775 807
CREATE TABLE mod_emoji_usage_emoji_use (
    id SERIAL PRIMARY KEY,
    guild_id BIGINT NOT NULL,
    emoji_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    created_at TIMESTAMP NOT NULL
);
