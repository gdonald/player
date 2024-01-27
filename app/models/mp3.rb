# frozen_string_literal: true

require 'taglib'

class Mp3 < ApplicationRecord
  belongs_to :source, counter_cache: true
  belongs_to :album, counter_cache: true
  belongs_to :artist, counter_cache: true

  has_many :playlist_mp3s, dependent: :destroy

  validates :filepath, presence: true
  validates :title, presence: true
  validates :track, numericality: { only_integer: true, greater_than: -1, allow_nil: true }

  CLEAN = /[^-_0-9a-zA-Z .,():'"]/

  scope :search, lambda { |params| # rubocop:disable Metrics/BlockLength
    query = params[:q]

    result = Mp3.includes(:artist, :album).references(:artists, :album)

    return result if query.blank?

    artist = query.scan(/artist:"(.+?)"/)
    query = query.gsub(/artist:"(.+?)"/, '').strip

    album = query.scan(/album:"(.+?)"/)
    query = query.gsub(/album:"(.+?)"/, '').strip

    query = query.downcase.gsub(CLEAN, '')
    phrases = query.scan(/"(.+?)"/).flatten
    query = query.gsub(/"(.+?)"/, '').strip

    parts = query.include?(' ') ? query.split : [query]
    parts += phrases unless phrases.empty?
    parts = parts.select(&:present?)

    ors = []
    ['artists.name', 'albums.name', 'mp3s.title'].each do |field|
      q = []
      parts.each do |p|
        p = p.gsub("'", "''")
        q << "#{field} ILIKE '%#{p}%'"
      end
      q = q.join(' AND ')
      ors << "(#{q})" if q.present?
    end
    result = result.where(ors.join(' OR ')) if ors.any?

    if artist.any?
      artist = artist.first.first.gsub(CLEAN, '').gsub("'", "''")
      result = result.where("artists.name ILIKE '#{artist}'")
    end

    if album.any?
      album = album.first.first.gsub(CLEAN, '').gsub("'", "''")
      result = result.where("albums.name ILIKE '#{album}'")
    end

    result.order(order_by(params[:sort]))
  }

  scope :ordered, ->(params) { includes(:artist, :album).references(:artists, :album).order(order_by(params[:sort])) }

  def self.order_by(param) # rubocop:disable Metrics/CyclomaticComplexity
    parts = param&.split('_')
    dir = parts&.last&.upcase

    case parts&.first
    when 'artist'
      "artists.name #{dir}"
    when 'album'
      "albums.name #{dir}"
    when 'track'
      "mp3s.track #{dir}"
    when 'title'
      "mp3s.title #{dir}"
    else
      'artists.name, albums.name, mp3s.track, mp3s.title'
    end
  end

  def duration
    Time.at(length).utc.strftime('%M:%S')
  end

  def self.create_mp3(source, filepath, ref) # rubocop:disable Metrics/AbcSize
    tag = ref.tag
    properties = ref.audio_properties
    title = tag.title
    artist_name = tag.artist

    if title.blank? || artist_name.blank?
      parts = filepath.split('/')
      filename = parts.last.gsub('.mp3', '')
      mp3_parts = filename.split('-').map(&:strip)

      if mp3_parts.size == 2
        artist_name = mp3_parts.first
        title = mp3_parts.last
      else
        title = filename
      end
    end

    artist = Artist.find_unknown(name: artist_name)
    album = Album.find_unknown(artist:, name: tag.album)
    length = properties.length_in_seconds

    begin
      Mp3.create!(filepath:, source:, artist:, album:, title:, length:,
                  genre: tag.genre,
                  year: tag.year,
                  track: tag.track,
                  comment: tag.comment)
    rescue StandardError => e
      Rails.logger.error("Error creating mp3: #{e.message}")
      raise e
    end
  end

  def do_update(ref) # rubocop:disable Metrics/AbcSize
    tag = ref.tag
    properties = ref.audio_properties

    title = tag.title
    artist_name = tag.artist

    if title.blank? || artist_name.blank?
      parts = filepath.split('/')
      filename = parts.last.gsub('.mp3', '')
      mp3_parts = filename.split('-').map(&:strip)

      if mp3_parts.size == 2
        artist_name = mp3_parts.first
        title = mp3_parts.last
      end
    end

    artist = Artist.find_unknown(name: artist_name)
    album = Album.find_unknown(artist:, name: tag.album)
    length = properties.length_in_seconds

    update!(artist:, album:, title:, length:,
            genre: tag.genre,
            year: tag.year,
            track: tag.track,
            comment: tag.comment)
  end

  def artist_name
    artist&.name
  end

  def album_name
    album&.name
  end
end
