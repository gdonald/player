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

  private

  def playlist_params
    params.require(:playlist).permit(:name)
  end

end
