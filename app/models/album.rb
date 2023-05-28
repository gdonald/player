# frozen_string_literal: true

class Album < ApplicationRecord
  UNKNOWN = 'Unknown'

  has_many :mp3s, dependent: :destroy
  belongs_to :artist

  validates :name, presence: true, uniqueness: { scope: :artist_id }

  scope :ordered, -> { order(:name) }

  CLEAN = /[^-_0-9a-zA-Z .,()"]/

  scope :search, lambda { |query|
    return all if query.blank?

    query = query.gsub(/album:"(.+?)"/, '').strip
    query = query.gsub(/artist:"(.+?)"/, '').strip

    query = query.downcase.gsub(CLEAN, '')
    phrases = query.scan(/"(.+?)"/).flatten
    query = query.gsub(/"(.+?)"/, '').strip

    parts = query.include?(' ') ? query.split : [query]
    parts += phrases unless phrases.empty?
    parts = parts.select(&:present?)

    result = Album

    ors = parts.map { |p| "albums.name ILIKE '%#{p}%'" }
    result = result.where(ors.join(' OR ')) if ors.any?

    result
  }

  def self.find_unknown(artist:, name:)
    name.strip!
    name = UNKNOWN if name.blank?

    artist ||= Artist.find_by(name: Artist::UNKNOWN)
    raise 'Unknown artist not found' unless artist

    album = Album.find_by(artist:, name:)
    album ||= Album.create!(artist:, name:)

    album
  end

  def to_s
    name
  end
end
