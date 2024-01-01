# frozen_string_literal: true

class Playlist < ApplicationRecord
  RECENT = 'Recently Played'

  has_many :playlist_mp3s, -> { order(:position) }, dependent: :destroy, inverse_of: :playlist
  has_many :mp3s, through: :playlist_mp3s
  has_many :albums, through: :mp3s
  has_many :artists, through: :mp3s

  accepts_nested_attributes_for :playlist_mp3s, allow_destroy: false, reject_if: :already_in_playlist?

  validates :name, presence: true, uniqueness: true

  after_create :reorder_using_tracks

  scope :ordered, lambda {
    where(name: RECENT) +
      where.not(name: RECENT).order(:name)
  }

  def self.select_options
    ordered.collect { |p| [p.name, p.id] }
  end

  def enqueue
    return unless playlist_mp3s.any?

    playlist_mp3s.each do |playlist_mp3|
      QueuedMp3.create(mp3_id: playlist_mp3.mp3_id)
    end
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

  private

  def already_in_playlist?(attrs)
    PlaylistMp3.find_by(mp3_id: attrs['mp3_id'], playlist_id: id)
  end

  def reorder_using_tracks
    playlist_mp3s.each do |playlist_mp3|
      playlist_mp3.update(position: playlist_mp3.mp3.track || position)
    end
  end
end
