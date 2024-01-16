# frozen_string_literal: true

module Api
  class ApplicationController < ActionController::Base
    skip_before_action :verify_authenticity_token

    def require_login
      return if session[:user_id].nil?

      @current_user = User.where(id: session[:user_id]).first
    end
  end
end
