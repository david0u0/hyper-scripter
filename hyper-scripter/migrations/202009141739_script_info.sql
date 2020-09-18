CREATE TABLE IF NOT EXISTS script_infos (
    id integer PRIMARY KEY AUTOINCREMENT NOT NULL,
    name text NOT NULL UNIQUE,
    category varchar(10) NOT NULL,
    tags text NOT NULL,
    write_time datetime NOT NULL DEFAULT (STRFTIME ('%Y-%m-%d %H:%M:%f', 'NOW')),
    created_time datetime NOT NULL DEFAULT (STRFTIME ('%Y-%m-%d %H:%M:%f', 'NOW'))
);

