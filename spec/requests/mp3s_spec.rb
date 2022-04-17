# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Mp3s', type: :request do
  describe 'GET /index' do
    it 'returns http success' do
      get '/mp3s/index'
      expect(response).to have_http_status(:success)
    end
  end

  describe 'GET /edit' do
    it 'returns http success' do
      get '/mp3s/edit'
      expect(response).to have_http_status(:success)
    end
  end

  describe 'GET /update' do
    it 'returns http success' do
      get '/mp3s/update'
      expect(response).to have_http_status(:success)
    end
  end
end
