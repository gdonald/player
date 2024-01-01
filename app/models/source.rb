# frozen_string_literal: true

class Source < ApplicationRecord
  include AASM

  aasm column: 'state' do
    state :unscanned, initial: true
    state :scanning
    state :scanned
    state :errored

    event :start_scan do
      transitions from: %i[unscanned scanned errored], to: :scanning
    end

    event :done do
      transitions from: :scanning, to: :scanned
    end

    event :error do
      transitions from: :scanning, to: :errored
    end
  end

  scope :ordered, -> { order(:path) }

  has_many :mp3s, -> { order(:filepath) }, dependent: :destroy, inverse_of: :source

  validates :path, presence: true, uniqueness: true

  def scan(truncate: false)
    if truncate
      Mp3.destroy_all
      Album.destroy_all
      Artist.destroy_all
      Playlist.destroy_all
    end

    check_known
    do_scan
  end

  private

  def do_scan
    Dir.glob("#{path}/**/*.mp3", File::FNM_CASEFOLD).each do |filepath|
      TagLib::FileRef.open(filepath) do |ref|
        raise StandardError, "Cannot read file #{filepath}" if ref.nil?

        mp3 = Mp3.find_by(filepath:)
        if mp3
          mp3.do_update(ref)
        else
          Mp3.create_mp3(self, filepath, ref)
        end
      end
    end
  end

  def check_known
    Mp3.find_each do |mp3|
      unless File.readable?(mp3.filepath)
        logger.warn("Cannot find known file #{mp3.filepath}")
        mp3.destroy!
      end
    end
  end
end
