# frozen_string_literal: true

class QueuedMp3 < ApplicationRecord
  acts_as_list

  belongs_to :mp3
  has_one :artist, through: :mp3

  scope :ordered, -> { includes(:mp3, :artist).order(:position) }
end
