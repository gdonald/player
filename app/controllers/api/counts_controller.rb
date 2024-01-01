# frozen_string_literal: true

module Api
  class CountsController < Api::ApplicationController
    def index
      mp3s_count = Mp3.count
      playlists_count = Playlist.count
      sources_count = Source.count

      render json: { mp3s_count:, playlists_count:, sources_count: }
    end
  end
end
