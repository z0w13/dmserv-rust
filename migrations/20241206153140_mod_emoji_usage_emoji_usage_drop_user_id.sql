-- NOTE: this module is used for tracking emoji statistics for the server
--       tracking individual user's emoji usage feels invasive
ALTER TABLE mod_emoji_usage_emoji_use DROP COLUMN user_id;
