# frozen_string_literal: true

class PlaylistMp3sController < ApplicationController
  def index; end

  def show; end

  def new; end

  def add
    playlist = Playlist.find_by(id: params['playlist_mp3s']['playlist_id'])
    head :ok unless playlist

    params['mp3s'].each do |id|
      mp3 = Mp3.find_by(id:)
      next unless mp3

      playlist.mp3s << mp3
    end

    head :ok
  end

  def edit; end

  def create
    playlist = current_playlist
    mp3 = Mp3.find_by(id: params[:mp3_id])
    if mp3
      PlaylistMp3.find_or_create_by(playlist:, mp3:)
    else
      head :ok
    end
  end

  def move_up
    playlist_mp3 = PlaylistMp3.find_by(id: params[:id])
    playlist_mp3.move_higher
    @playlist = playlist_mp3.playlist
  end

  def move_down
    playlist_mp3 = PlaylistMp3.find_by(id: params[:id])
    playlist_mp3.move_lower
    @playlist = playlist_mp3.playlist
  end

  def update; end

  def destroy
    playlist_mp3 = PlaylistMp3.find_by(id: params[:id])
    @playlist = playlist_mp3.playlist
    playlist_mp3.destroy
  end
end
