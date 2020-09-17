CREATE TABLE IF NOT EXISTS script_infos (
    id integer PRIMARY KEY NOT NULL,
    name text NOT NULL UNIQUE,
    category varchar(10) NOT NULL,
    tags text NOT NULL,
    write_time timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_time timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

