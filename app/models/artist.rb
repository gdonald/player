# frozen_string_literal: true

class Artist < ApplicationRecord
  UNKNOWN = 'Unknown'

  has_many :albums, dependent: :destroy
  has_many :mp3s, dependent: :destroy

  validates :name, presence: true, uniqueness: true

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

    result = Artist

    ors = parts.map { |p| "artists.name ILIKE '%#{p}%'" }
    result = result.where(ors.join(' OR ')) if ors.any?

    result
  }

  def self.find_unknown(name:)
    name.strip!
    name = UNKNOWN if name.blank?

    artist = Artist.find_by(name:)
    artist ||= Artist.create!(name:)

    artist
  end

  def to_s
    name
  end
end
