# frozen_string_literal: true

class Artist < ApplicationRecord
  UNKNOWN = 'Unknown'

  has_many :albums, dependent: :destroy
  has_many :mp3s, dependent: :destroy

  validates :name, presence: true, uniqueness: true

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
