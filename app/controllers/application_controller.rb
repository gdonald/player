# frozen_string_literal: true

class ApplicationController < ActionController::Base
  before_action :current_playlist, :ensure_recent_exists

  def current_playlist
    @current_playlist = Playlist.find_by(id: session[:current_playlist_id])
    @current_playlist ||= Playlist.find_by(name: Playlist::RECENT)
  end

  def ensure_recent_exists
    Playlist.find_or_create_by(name: 'Recently Played')
  end
end
