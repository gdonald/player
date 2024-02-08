# frozen_string_literal: true

json.mp3s @mp3s do |mp3|
  json.call(mp3, :id, :title, :track, :artist_name, :album_name, :length, :file_hash)
end
