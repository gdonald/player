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

  def edit; end

  def update; end
end
