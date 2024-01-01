# frozen_string_literal: true

json.sources @sources do |source|
  json.id source.id
  json.path source.path
  json.mp3s_count source.mp3s_count
end
