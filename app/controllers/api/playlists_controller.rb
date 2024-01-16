# frozen_string_literal: true

module Api
  class PlaylistsController < Api::ApplicationController
    before_action :current_user

    def index
      @playlists = Playlist.ordered
    end

    def show
      @playlist = Playlist.find(params[:id])
    end

    def create
      @playlist = Playlist.new(playlist_params)
      if @playlist.save
        @message = 'Playlist created'
        render :show
      else
        render json: { errors: playlist.errors,
                       message: 'Failed to create playlist' },
               status: :unprocessable_entity
      end
    end

    def update
      @playlist = Playlist.find(params[:id])
      if @playlist.update(playlist_params)
        @message = 'Playlist updated'
        render :show
      else
        render json: { errors: @playlist.errors,
                       message: 'Failed to update playlist' },
               status: :unprocessable_entity
      end
    end

    def destroy
      playlist = Playlist.find(params[:id])
      playlist.destroy
      render json: { message: 'Playlist deleted' }
    end

    def enqueue
      playlist = Playlist.find_by(id: params[:id])
      playlist&.enqueue

      @queued_mp3s = QueuedMp3.ordered
      render 'api/queued_mp3s/index'
    end

    private

    def playlist_params
      params.require(:playlist).permit(:name, playlist_mp3s_attributes: %i[mp3_id])
    end
  end
end
