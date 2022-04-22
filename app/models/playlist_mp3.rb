# frozen_string_literal: true

class PlaylistMp3 < ApplicationRecord
  belongs_to :playlist, counter_cache: true
  acts_as_list scope: :playlist

  belongs_to :mp3
end
