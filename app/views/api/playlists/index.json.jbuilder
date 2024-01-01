# frozen_string_literal: true

json.playlists @playlists do |playlist|
  json.id playlist.id
  json.name playlist.name
  json.mp3s_count playlist.playlist_mp3s_count
end
