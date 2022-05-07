# frozen_string_literal: true

require 'taglib'

class Mp3 < ApplicationRecord
  belongs_to :source, counter_cache: true
  belongs_to :album, counter_cache: true
  belongs_to :artist, counter_cache: true

  validates :filepath, presence: true

  scope :ordered, -> { includes(:artist, :album).order('artists.name, albums.name, mp3s.title') }

  scope :search, lambda { |query|
    return all if query.blank?

    query = query.downcase.gsub(/[^-_0-9a-z "]/, '')

    phrases = query.scan(/"(.+?)"/).flatten
    query = query.gsub(/"(.+?)"/, '').strip

    parts = query.include?(' ') ? query.split : [query]
    parts += phrases unless phrases.empty?
    parts = parts.select(&:present?)

    ors = parts.map { |p| "artists.name ILIKE '%#{p}%' OR albums.name ILIKE '%#{p}%' OR mp3s.title ILIKE '%#{p}%'" }
    includes(:artist, :album).where(ors.join(' OR '))
  }

  def duration
    Time.at(length).utc.strftime('%M:%S')
  end

  def self.create_mp3(source, filepath, ref)
    tag = ref.tag
    properties = ref.audio_properties
    artist = Artist.find_unknown(name: tag.artist)
    album = Album.find_unknown(artist:, name: tag.album)
    title = tag.title
    length = properties.length_in_seconds

    begin
      Mp3.create!(filepath:, source:, artist:, album:, title:, length:,
                  genre: tag.genre,
                  year: tag.year,
                  track: tag.track,
                  comment: tag.comment)
    rescue ActiveRecord::RecordNotUnique
      logger.warn "Duplicate: artist: #{artist} album: #{album} title: #{title} length: #{length}"
    end
  end

  def do_update(ref)
    tag = ref.tag
    properties = ref.audio_properties
    artist = Artist.find_unknown(name: tag.artist)
    album = Album.find_unknown(artist:, name: tag.album)
    title = tag.title
    length = properties.length_in_seconds

    begin
      update!(artist:, album:, title:, length:,
              genre: tag.genre,
              year: tag.year,
              track: tag.track,
              comment: tag.comment)
    rescue ActiveRecord::RecordNotUnique
      logger.warn "Duplicate: artist: #{artist} album: #{album} title: #{title} length: #{length}"
    end
  end
end
