CREATE TABLE sources (
    id BIGSERIAL PRIMARY KEY,
    path VARCHAR NOT NULL,
    state VARCHAR NOT NULL DEFAULT 'unscanned'
        CHECK (state IN ('unscanned', 'scanning', 'scanned', 'errored')),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX index_sources_on_path ON sources (path);
CREATE INDEX index_sources_on_state ON sources (state);

CREATE TABLE artists (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX index_artists_on_name ON artists (name);

CREATE TABLE albums (
    id BIGSERIAL PRIMARY KEY,
    artist_id BIGINT NOT NULL REFERENCES artists (id) ON DELETE CASCADE,
    name VARCHAR NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX index_albums_on_artist_id_and_name ON albums (artist_id, name);
CREATE INDEX index_albums_on_artist_id ON albums (artist_id);

CREATE TABLE mp3s (
    id BIGSERIAL PRIMARY KEY,
    source_id BIGINT NOT NULL REFERENCES sources (id) ON DELETE CASCADE,
    artist_id BIGINT NOT NULL REFERENCES artists (id) ON DELETE CASCADE,
    album_id BIGINT NOT NULL REFERENCES albums (id) ON DELETE CASCADE,
    filepath VARCHAR NOT NULL,
    title VARCHAR,
    genre VARCHAR,
    year INTEGER,
    track INTEGER,
    length INTEGER,
    comment TEXT,
    file_hash VARCHAR,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX index_mp3s_on_artist_id_and_album_id_and_title_and_length
    ON mp3s (artist_id, album_id, title, length);
CREATE INDEX index_mp3s_on_album_id ON mp3s (album_id);
CREATE INDEX index_mp3s_on_artist_id ON mp3s (artist_id);
CREATE INDEX index_mp3s_on_filepath ON mp3s (filepath);
CREATE INDEX index_mp3s_on_genre ON mp3s (genre);
CREATE INDEX index_mp3s_on_length ON mp3s (length);
CREATE INDEX index_mp3s_on_source_id ON mp3s (source_id);
CREATE INDEX index_mp3s_on_title ON mp3s (title);
CREATE INDEX index_mp3s_on_track ON mp3s (track);
CREATE INDEX index_mp3s_on_year ON mp3s (year);

CREATE TABLE playlists (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX index_playlists_on_name ON playlists (name);

CREATE TABLE playlist_mp3s (
    id BIGSERIAL PRIMARY KEY,
    playlist_id BIGINT NOT NULL REFERENCES playlists (id) ON DELETE CASCADE,
    mp3_id BIGINT NOT NULL REFERENCES mp3s (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT playlist_mp3s_position_unique UNIQUE (playlist_id, position)
        DEFERRABLE INITIALLY DEFERRED
);

CREATE UNIQUE INDEX index_playlist_mp3s_on_playlist_id_and_mp3_id
    ON playlist_mp3s (playlist_id, mp3_id);
CREATE INDEX index_playlist_mp3s_on_mp3_id ON playlist_mp3s (mp3_id);

CREATE TABLE queued_mp3s (
    id BIGSERIAL PRIMARY KEY,
    mp3_id BIGINT NOT NULL REFERENCES mp3s (id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT queued_mp3s_position_unique UNIQUE (position)
        DEFERRABLE INITIALLY DEFERRED
);

CREATE INDEX index_queued_mp3s_on_mp3_id ON queued_mp3s (mp3_id);

CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(32) NOT NULL,
    p_salt VARCHAR(80),
    p_hash VARCHAR(80),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX index_users_on_username ON users (username);
