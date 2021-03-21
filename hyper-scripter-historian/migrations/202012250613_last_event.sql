CREATE TABLE IF NOT EXISTS last_events (
    script_id integer NOT NULL,
    type varchar(10) NOT NULL,
    cmd text NOT NULL,
    content text,
    time datetime NOT NULL DEFAULT (STRFTIME ('%Y-%m-%d %H:%M:%f', 'NOW')),
    args text,
    UNIQUE (script_id, TYPE)
);

