# frozen_string_literal: true

json.queued_mp3s @queued_mp3s do |queued_mp3|
  json.id queued_mp3.id
  json.mp3_id queued_mp3.mp3_id
  json.title queued_mp3.mp3.title
  json.artist_name queued_mp3.mp3.artist_name
  json.position queued_mp3.position
end
