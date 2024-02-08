# frozen_string_literal: true

json.queued_mp3s @queued_mp3s do |queued_mp3|
  json.id queued_mp3.id
  json.position queued_mp3.position
  json.mp3 do
    json.id queued_mp3.mp3_id
    json.title queued_mp3.mp3.title
    json.artist_name queued_mp3.mp3.artist_name
    json.album_name queued_mp3.mp3.album_name
    json.track queued_mp3.mp3.track
    json.length queued_mp3.mp3.length
    json.file_hash queued_mp3.mp3.file_hash
  end
end
