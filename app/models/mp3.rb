# frozen_string_literal: true

require 'taglib'

class Mp3 < ApplicationRecord
  validates :filepath, presence: true

  scope :ordered, -> { order(:artist, :album, :title) }

  scope :search, lambda { |query|
    return all if query.blank?

    query = query.downcase.gsub(/[^-_0-9a-z "]/, '')

    phrases = query.scan(/"(.+?)"/).flatten
    query = query.gsub(/"(.+?)"/, '').strip

    parts = query.include?(' ') ? query.split : [query]
    parts += phrases unless phrases.empty?
    parts = parts.select(&:present?)

    ors = parts.map { |p| "artist ILIKE '%#{p}%' OR album ILIKE '%#{p}%' OR title ILIKE '%#{p}%'" }
    where(ors.join(' OR '))
  }

  def self.sync(truncate: false)
    if truncate
      Mp3.destroy_all
    else
      check_known
    end

    scan
  end

  def self.scan
    Dir.glob("#{ENV['MP3S_PATH']}/**/*.mp3", File::FNM_CASEFOLD).each do |filepath|
      TagLib::FileRef.open(filepath) do |ref|
        if ref.nil?
          logger.warn("Cannot read file #{filepath}")
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
  end

  def self.check_known
    Mp3.find_each do |mp3|
      unless File.readable?(mp3.filepath)
        logger.warn("Cannot find known file #{mp3.filepath}")
        mp3.destroy
      end
    end
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

  def duration
    Time.at(length).utc.strftime('%M:%S')
  end
end
