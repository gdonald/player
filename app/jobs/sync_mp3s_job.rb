# frozen_string_literal: true

class SyncMp3sJob < ApplicationJob
  queue_as :default

  def perform(*_args)
    Mp3.sync
  end
end
