ALTER TABLE script_infos ADD COLUMN hash INTEGER NOT NULL DEFAULT 0;

-- TODO: I would like to remove the default value here, but sqlite wouldn't let me
