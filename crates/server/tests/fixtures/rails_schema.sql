-- The application tables from old_rails_app/db/schema.rb (version 21), as
-- Rails created them: no foreign keys, nullable list columns, counter caches.
CREATE TABLE albums (
    id BIGSERIAL PRIMARY KEY,
    artist_id BIGINT NOT NULL,
    name VARCHAR NOT NULL,
    mp3s_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP(6) NOT NULL,
    updated_at TIMESTAMP(6) NOT NULL
);

CREATE TABLE artists (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    mp3s_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP(6) NOT NULL,
    updated_at TIMESTAMP(6) NOT NULL
);

CREATE TABLE mp3s (
    id BIGSERIAL PRIMARY KEY,
    source_id BIGINT NOT NULL,
    artist_id BIGINT NOT NULL,
    album_id BIGINT NOT NULL,
    filepath VARCHAR NOT NULL,
    title VARCHAR,
    genre VARCHAR,
    year INTEGER,
    track INTEGER,
    length INTEGER,
    comment TEXT,
    created_at TIMESTAMP(6) NOT NULL,
    updated_at TIMESTAMP(6) NOT NULL,
    file_hash VARCHAR
);

CREATE TABLE playlist_mp3s (
    id BIGSERIAL PRIMARY KEY,
    playlist_id BIGINT,
    mp3_id BIGINT,
    position INTEGER,
    created_at TIMESTAMP(6) NOT NULL,
    updated_at TIMESTAMP(6) NOT NULL
);

CREATE TABLE playlists (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    playlist_mp3s_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP(6) NOT NULL,
    updated_at TIMESTAMP(6) NOT NULL
);

CREATE TABLE queued_mp3s (
    id BIGSERIAL PRIMARY KEY,
    mp3_id INTEGER,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP(6) NOT NULL,
    updated_at TIMESTAMP(6) NOT NULL
);

CREATE TABLE sources (
    id BIGSERIAL PRIMARY KEY,
    path VARCHAR NOT NULL,
    mp3s_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP(6) NOT NULL,
    updated_at TIMESTAMP(6) NOT NULL,
    state VARCHAR NOT NULL DEFAULT 'unscanned'
);

CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(32) NOT NULL,
    p_salt VARCHAR(80),
    p_hash VARCHAR(80),
    created_at TIMESTAMP(6) NOT NULL,
    updated_at TIMESTAMP(6) NOT NULL
);
