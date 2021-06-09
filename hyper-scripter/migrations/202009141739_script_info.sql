CREATE TABLE IF NOT EXISTS script_infos (
    id integer PRIMARY KEY AUTOINCREMENT NOT NULL,
    name text NOT NULL UNIQUE,
    ty varchar(10) NOT NULL,
    tags text NOT NULL,
    created_time datetime NOT NULL DEFAULT (STRFTIME ('%Y-%m-%d %H:%M:%f', 'NOW'))
);

