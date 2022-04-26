# frozen_string_literal: true

class SourcesController < ApplicationController
  def index
    @sources = Source.ordered
  end

  def new
    @source = Source.new
  end

  def create
    @source = Source.create(source_params)
  end

  def edit
    @source = Source.find_by(id: params[:id])
  end

  def update
    @source = Source.find_by(id: params[:id])
    @source.update(source_params)
  end

  def destroy
    @source = Source.find_by(id: params[:id])
    @source.destroy
  end

  def sync
    @source = Source.find_by(id: params[:id])
    SyncMp3sJob.perform_later(@source.id)
    head :ok
  end

  private

  def source_params
    params.require(:source).permit(:path)
  end
end
