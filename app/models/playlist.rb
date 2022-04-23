# frozen_string_literal: true

class Playlist < ApplicationRecord
  has_many :playlist_mp3s, -> { order(:position) }, dependent: :destroy
  has_many :mp3s, through: :playlist_mp3s

  validates :name, presence: true

  scope :ordered, -> { order(:name) }

  def self.select_options
    ordered.collect { |p| [p.name, p.id] }
  end

  def next_mp3(mp3)
    current = playlist_mp3s.find_by(mp3:)
    lower_item = current.lower_item
    return playlist_mp3s.first.mp3 unless lower_item

    lower_item.mp3
  end
end
