# frozen_string_literal: true

module Api
  class QueuedMp3sController < Api::ApplicationController
    before_action :current_user

    def index
      @queued_mp3s = QueuedMp3.ordered
    end

    def create
      @queued_mp3 = QueuedMp3.new(queued_mp3_params)
      if @queued_mp3.save
        @queued_mp3s = QueuedMp3.ordered
        render :index
      else
        render json: @queued_mp3.errors.full_messages, status: :unprocessable_entity
      end
    end

    def destroy
      QueuedMp3.transaction do
        @queued_mp3 = QueuedMp3.find_by(id: params[:id])
        @queued_mp3&.destroy
      rescue ActiveRecord::RecordNotUnique
        Rails.logger.info 'ActiveRecord::RecordNotUnique occurred when deleting queued_mp3'
      end

      @queued_mp3s = QueuedMp3.ordered
      render :index
    end

    private

    def queued_mp3_params
      params.require(:queued_mp3).permit(:mp3_id)
    end
  end
end
