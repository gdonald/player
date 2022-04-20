class PlaylistMp3sController < ApplicationController
  def index
  end

  def show
  end

  def new
  end

  def create
    playlist = current_playlist
    mp3 = Mp3.find_by(id: params[:mp3_id])
    if mp3
      PlaylistMp3.find_or_create_by(playlist:, mp3:)
    else
      head :ok
    end
  end

  def edit
  end

  def update
  end

  def destroy
  end
end
