# frozen_string_literal: true

class ScanMp3sJob < ApplicationJob
  queue_as :default

  def perform(id)
    source = Source.find_by(id:)
    return unless source

    source.start_scan!
    begin
      source.scan
      source.done!
    rescue StandardError => e
      Rails.logger.debug e.backtrace
      source.error!
    end
  end
end
