# frozen_string_literal: true

json.message @message if @message

json.source do
  json.id @source.id
  json.path @source.path
end
