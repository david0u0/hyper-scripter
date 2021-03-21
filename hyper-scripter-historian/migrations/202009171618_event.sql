CREATE TABLE IF NOT EXISTS events (
    id integer PRIMARY KEY NOT NULL,
    script_id integer NOT NULL,
    type varchar(10) NOT NULL,
    cmd text NOT NULL,
    content text,
    args text,
    time datetime NOT NULL DEFAULT (STRFTIME ('%Y-%m-%d %H:%M:%f', 'NOW'))
);

