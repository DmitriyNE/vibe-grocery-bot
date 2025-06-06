-- Add column to track active delete DM message
ALTER TABLE delete_session ADD COLUMN dm_message_id INTEGER;
