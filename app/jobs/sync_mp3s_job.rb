# frozen_string_literal: true

class SyncMp3sJob < ApplicationJob
  queue_as :default

  def perform(id)
    source = Source.find_by(id:)
    source.sync
  end
end
