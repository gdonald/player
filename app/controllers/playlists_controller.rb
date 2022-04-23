class PlaylistsController < ApplicationController
  def index
    @playlists = Playlist.ordered
  end

  def show
  end

  def new
  end

  def create
  end

  def edit
    @playlist = Playlist.find_by(id: params[:id])
  end

  def update
    @playlist = Playlist.find_by(id: params[:id])
    @playlist.update(playlist_params)
  end

  def destroy
  end

  def play
    @playlist = Playlist.find_by(id: params[:id])
    session[:current_playlist_id] = @playlist.id
    @mp3 = @playlist.mp3s.first
    session[:current_playlist_mp3_id] = @mp3.id
  end

  def next
    @playlist = Playlist.find_by(id: session[:current_playlist_id])
    @current_mp3 = Mp3.find_by(id: session[:current_playlist_mp3_id])
    @mp3 = @playlist.next_mp3(@current_mp3)
    session[:current_playlist_mp3_id] = @mp3.id
  end

  private

  def playlist_params
    params.require(:playlist).permit(:name)
  end

end
