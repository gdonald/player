# frozen_string_literal: true

class Playlist < ApplicationRecord
  has_many :playlist_mp3s, -> { order(:position) }, dependent: :destroy
  has_many :mp3s, through: :playlist_mp3s

  validates :name, presence: true
end
