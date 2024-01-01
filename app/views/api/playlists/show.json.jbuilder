# frozen_string_literal: true

json.message @message if @message

json.playlist do
  json.id @playlist.id
  json.name @playlist.name
end
