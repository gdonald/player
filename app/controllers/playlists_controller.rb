# frozen_string_literal: true

class PlaylistsController < ApplicationController
  def index
    @playlists = Playlist.ordered
  end

  def show; end

  def new
    @playlist = Playlist.new
  end

  def edit
    @playlist = Playlist.find_by(id: params[:id])
  end

  def create
    @playlist = Playlist.create(playlist_params)
  end

  def update
    @playlist = Playlist.find_by(id: params[:id])
    @playlist.update(playlist_params)
  end

  def destroy; end

  def play
    @playlist = Playlist.includes(mp3s: %i[album artist]).where(id: params[:id]).first
    session[:current_playlist_id] = @playlist.id
  end

  def next
    @playlist = Playlist.find_by(id: session[:current_playlist_id])
    @current_mp3 = Mp3.find_by(id: session[:current_mp3_id])
    @mp3 = @playlist.next_mp3(@current_mp3)
    session[:current_mp3_id] = @mp3.id
  end

  def prev
    @playlist = Playlist.find_by(id: session[:current_playlist_id])
    @current_mp3 = Mp3.find_by(id: session[:current_mp3_id])
    @mp3 = @playlist.prev_mp3(@current_mp3)
    session[:current_mp3_id] = @mp3.id
  end

  private

  def playlist_params
    params.require(:playlist).permit(:name)
  end
end
