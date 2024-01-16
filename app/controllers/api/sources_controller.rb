# frozen_string_literal: true

module Api
  class SourcesController < Api::ApplicationController
    before_action :current_user

    def index
      @sources = Source.ordered
    end

    def show
      @source = Source.find(params[:id])
    end

    def update
      @source = Source.find(params[:id])
      if @source.update(source_params)
        @message = 'Source updated'
        render :show
      else
        render json: { errors: @source.errors,
                       message: 'Failed to update source' },
               status: :unprocessable_entity
      end
    end

    def scan
      @source = Source.find_by(id: params[:id])
      ScanMp3sJob.perform_later(@source.id)
      head :ok
    end

    private

    def source_params
      params.require(:source).permit(:path)
    end
  end
end
