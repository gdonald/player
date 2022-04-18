# frozen_string_literal: true

require 'taglib'

class Mp3 < ApplicationRecord
  validates :filepath, presence: true

  scope :ordered, -> { order(:artist, :album, :title) }

  scope :search, lambda { |query|
    return all if query.blank?

    query = query.downcase.gsub(/[^-_0-9a-z ]/, '')
    parts = query.include?(' ') ? query.split : [query]
    ors = parts.map { |p| "artist ILIKE '%#{p}%' OR album ILIKE '%#{p}%' OR title ILIKE '%#{p}%'" }
    where(ors.join(' OR '))
  }

  def self.scan(truncate: false)
    Mp3.destroy_all if truncate

    files = Dir.glob("#{ENV['MP3S_PATH']}/**/*.mp3", File::FNM_CASEFOLD)

    bad = []

    files.each do |filepath|
      TagLib::FileRef.open(filepath) do |ref|
        if ref.nil?
          bad << filepath
          next
        end

        mp3 = Mp3.find_by(filepath:)
        if mp3
          mp3.do_update(ref)
        else
          create_mp3(filepath, ref)
        end
      end
    end

    puts "bad files: #{bad}"
    nil
  end

  def do_update(ref)
    tag = ref.tag
    properties = ref.audio_properties

    update!(title: tag.title,
            artist: tag.artist,
            album: tag.album,
            genre: tag.genre,
            year: tag.year,
            track: tag.track,
            length: properties.length_in_seconds,
            comment: tag.comment)
  end

  def self.create_mp3(filepath, ref)
    tag = ref.tag
    properties = ref.audio_properties

    Mp3.create!(filepath:,
                title: tag.title,
                artist: tag.artist,
                album: tag.album,
                genre: tag.genre,
                year: tag.year,
                track: tag.track,
                length: properties.length_in_seconds,
                comment: tag.comment)
  end

  def duration
    Time.at(length).utc.strftime('%M:%S')
  end
end
