# frozen_string_literal: true

class ApplicationController < ActionController::Base
  before_action :current_playlist

  def current_playlist
    @current_playlist = Playlist.find_by(id: session[:current_playlist])
    @current_playlist ||= Playlist.last
    @current_playlist ||= Playlist.create(name: 'New Playlist')
  end
end
