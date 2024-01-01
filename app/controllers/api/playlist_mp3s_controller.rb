# frozen_string_literal: true

module Api
  class PlaylistMp3sController < Api::ApplicationController
    def index
      playlist = Playlist.find_by(id: params[:playlist_id])
      @playlist_mp3s = playlist.playlist_mp3s.includes(mp3: %i[artist album]).order(:position)
      @first = @playlist_mp3s.min_by(&:position)
      @last = @playlist_mp3s.max_by(&:position)
    end

    %w[move_higher move_lower destroy].each do |action|
      define_method(action) do
        do_action(params[:id], action.to_sym)
      end
    end

    private

    def do_action(id, action)
      playlist_mp3 = PlaylistMp3.find_by(id:)
      playlist_mp3.send(action)
      playlist = playlist_mp3.playlist
      @playlist_mp3s = playlist.playlist_mp3s
      render :index
    end
  end
end
