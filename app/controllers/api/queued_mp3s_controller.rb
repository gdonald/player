# frozen_string_literal: true

module Api
  class QueuedMp3sController < Api::ApplicationController
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
      @queued_mp3 = QueuedMp3.find_by(id: params[:id])
      @queued_mp3&.destroy

      @queued_mp3s = QueuedMp3.ordered
      render :index
    end

    private

    def queued_mp3_params
      params.require(:queued_mp3).permit(:mp3_id)
    end
  end
end
