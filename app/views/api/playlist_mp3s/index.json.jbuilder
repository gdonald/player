# frozen_string_literal: true

json.playlist_mp3s @playlist_mp3s do |playlist_mp3|
  json.id playlist_mp3.id
  json.first playlist_mp3 == @first
  json.last playlist_mp3 == @last
  json.mp3 do
    json.id playlist_mp3.mp3_id
    json.title playlist_mp3.mp3.title
    json.artist_name playlist_mp3.mp3.artist_name
    json.album_name playlist_mp3.mp3.album_name
    json.track playlist_mp3.mp3.track
    json.length playlist_mp3.mp3.length
    json.file_hash playlist_mp3.mp3.file_hash
  end
end
