# frozen_string_literal: true

json.playlist_mp3s @playlist_mp3s do |playlist_mp3|
  json.id playlist_mp3.id
  json.mp3_id playlist_mp3.mp3_id
  json.title playlist_mp3.mp3.title
  json.artist_name playlist_mp3.mp3.artist_name
  json.album_name playlist_mp3.mp3.album_name
  json.track playlist_mp3.mp3.track
  json.first playlist_mp3 == @first
  json.last playlist_mp3 == @last
end
