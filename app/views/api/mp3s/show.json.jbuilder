# frozen_string_literal: true

json.message @message if @message

json.mp3 do
  json.id @mp3.id
  json.title @mp3.title
  json.artist_name @mp3.artist_name
  json.album_name @mp3.album_name
end
