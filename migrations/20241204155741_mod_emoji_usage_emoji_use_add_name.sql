-- Add migration script here
ALTER TABLE mod_emoji_usage_emoji_use ADD COLUMN emoji_name VARCHAR(32) NOT NULL;
