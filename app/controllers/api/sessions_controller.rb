# frozen_string_literal: true

module Api
  class SessionsController < ApplicationController
    before_action :current_user, only: %i[active]

    layout 'admin'

    def create
      @user = User.where(username: session_params[:username]).first
      if @user && User.authenticate(@user.username, session_params[:password])
        session[:user_id] = @user.id
        render json: {}, status: :ok
      else
        render json: {}, status: :unauthorized
      end
    end

    def destroy
      session[:user_id] = nil
      render json: {}, status: :ok
    end

    def active
      if @current_user
        render json: {}, status: :ok
      else
        render json: {}, status: :unauthorized
      end
    end

    private

    def session_params
      params.require(:session).permit(:username, :password)
    end
  end
end
