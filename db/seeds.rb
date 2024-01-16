# frozen_string_literal: true

artist = Artist.create!(name: Artist::UNKNOWN)
Album.create!(artist:, name: Album::UNKNOWN)
Playlist.create!(name: Playlist::RECENT)

User.create(username: 'gd',
            password: 'changeme',
            password_confirmation: 'changeme')
