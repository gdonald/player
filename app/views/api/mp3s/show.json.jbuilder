# frozen_string_literal: true

json.message @message if @message

json.mp3 do
  json.id @mp3.id
  json.title @mp3.title
  json.artist_name @mp3.artist_name
  json.album_name @mp3.album_name
  json.track @mp3.track
  json.length @mp3.length
  json.file_hash @mp3.file_hash
end
