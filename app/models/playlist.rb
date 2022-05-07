# frozen_string_literal: true

class Playlist < ApplicationRecord
  RECENT = 'Recently Played'

  has_many :playlist_mp3s, -> { order(:position) }, dependent: :destroy
  has_many :mp3s, through: :playlist_mp3s

  validates :name, presence: true, uniqueness: true

  scope :ordered, -> { order(:name) }

  def self.select_options
    ordered.collect { |p| [p.name, p.id] }
  end

  def next_mp3(mp3)
    current = playlist_mp3s.find_by(mp3:)
    lower_item = current&.lower_item
    return playlist_mp3s.first.mp3 unless lower_item

    lower_item.mp3
  end

  def prev_mp3(mp3)
    current = playlist_mp3s.find_by(mp3:)
    higher_item = current&.higher_item
    return playlist_mp3s.last.mp3 unless higher_item

    higher_item.mp3
  end
end
