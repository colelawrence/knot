CREATE SEQUENCE global_id_sequence;

CREATE OR REPLACE FUNCTION id_generator(OUT result bigint) AS $$
DECLARE
    our_epoch bigint := 1314220021721;
    seq_id bigint;
    now_millis bigint;
    -- the id of this DB shard, must be set for each
    -- schema shard you have - you could pass this as a parameter too
    shard_id int := 1;
BEGIN
    SELECT nextval('global_id_sequence') % 1024 INTO seq_id;

    SELECT FLOOR(EXTRACT(EPOCH FROM clock_timestamp()) * 1000) INTO now_millis;
    result := (now_millis - our_epoch) << 23;
    result := result | (shard_id << 10);
    result := result | (seq_id);
END;
$$ LANGUAGE PLPGSQL;

-- https://stackoverflow.com/questions/12575022/generating-an-instagram-or-youtube-like-unguessable-string-id-in-ruby-activerec/12590064#12590064
CREATE OR REPLACE FUNCTION stringify_bigint(n bigint) RETURNS TEXT
    LANGUAGE plpgsql IMMUTABLE STRICT AS $$
DECLARE
 alphabet TEXT:='abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
 base INT:=length(alphabet); 
 _n BIGINT:=abs(n);
 output TEXT:='';
BEGIN
 LOOP
   output := output || substr(alphabet, 1+(_n%base)::INT, 1);
   _n := _n / base; 
   EXIT WHEN _n=0;
 END LOOP;
 RETURN output;
END $$;

-- Personal information
CREATE TABLE users (
    id TEXT PRIMARY KEY DEFAULT stringify_bigint(id_generator()),
    display_name TEXT NOT NULL CONSTRAINT users_name_not_empty CHECK (display_name <> ''),
    full_name TEXT,
    photo_url TEXT,
    is_person BOOL NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- No personal information
CREATE TABLE user_logins (
    -- External ID: "goog|people/109727288588076782324"
    login_key TEXT PRIMARY KEY,
    -- Optional (Null if they have not signed up)
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE
);
