# frozen_string_literal: true

module Api
  class ApplicationController < ActionController::Base
    skip_before_action :verify_authenticity_token

    def current_user
      if session[:user_id].nil?
        @current_user = nil
      else
        @current_user ||= User.where(id: session[:user_id]).first
      end

      render json: {}, status: :unauthorized if @current_user.nil?
    end
  end
end
