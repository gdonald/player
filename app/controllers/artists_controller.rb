# frozen_string_literal: true

class ArtistsController < ApplicationController
  def index
    @artists = Artist.ordered
  end

  def search
    @artists = Artist.search(params[:q]).ordered
    session[:artists_q] = params[:q]
  end
end
