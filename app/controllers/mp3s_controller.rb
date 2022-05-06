# frozen_string_literal: true

class Mp3sController < ApplicationController
  def index
    @mp3s = Mp3.ordered
  end

  def search
    @mp3s = Mp3.search(params[:q]).ordered
    session[:q] = params[:q]
  end

  def play
    @mp3 = Mp3.find_by(id: params[:id])
  end

  def src
    @mp3 = Mp3.find_by(id: params[:id])
    send_file(@mp3.filepath)
  end

  def edit
    @mp3 = Mp3.find_by(id: params[:id])
  end

  def update
    @mp3 = Mp3.find_by(id: params[:id])
    @mp3.update(mp3_params)

    TagLib::MPEG::File.open(@mp3.filepath) do |file|
      tag = file.id3v2_tag

      tag.title = @mp3.title
      tag.album = @mp3.album
      tag.artist = @mp3.artist

      file.save
    end
  end

  private

  def mp3_params
    params.require(:mp3).permit(:title, :album, :artist)
  end
end
