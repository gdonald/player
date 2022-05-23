# frozen_string_literal: true

class Album < ApplicationRecord
  UNKNOWN = 'Unknown'

  has_many :mp3s, dependent: :destroy
  belongs_to :artist

  validates :name, presence: true, uniqueness: { scope: :artist_id }

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
