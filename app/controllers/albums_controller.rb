# frozen_string_literal: true

class AlbumsController < ApplicationController
  def index
    @albums = Album.ordered
  end

  def search
    @albums = Album.search(params[:q]).ordered
    session[:albums_q] = params[:q]
  end
end
