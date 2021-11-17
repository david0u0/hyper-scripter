CREATE TABLE IF NOT EXISTS events (
    id integer PRIMARY KEY NOT NULL,
    script_id integer NOT NULL,
    type int8 NOT NULL,
    cmd text NOT NULL,
    content text,
    args text,
    time datetime NOT NULL
);

