# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'PlaylistMp3s', type: :request do
  describe 'GET /index' do
    it 'returns http success' do
      get '/playlist_mp3s/index'
      expect(response).to have_http_status(:success)
    end
  end

  describe 'GET /show' do
    it 'returns http success' do
      get '/playlist_mp3s/show'
      expect(response).to have_http_status(:success)
    end
  end

  describe 'GET /new' do
    it 'returns http success' do
      get '/playlist_mp3s/new'
      expect(response).to have_http_status(:success)
    end
  end

  describe 'GET /create' do
    it 'returns http success' do
      get '/playlist_mp3s/create'
      expect(response).to have_http_status(:success)
    end
  end

  describe 'GET /edit' do
    it 'returns http success' do
      get '/playlist_mp3s/edit'
      expect(response).to have_http_status(:success)
    end
  end

  describe 'GET /update' do
    it 'returns http success' do
      get '/playlist_mp3s/update'
      expect(response).to have_http_status(:success)
    end
  end

  describe 'GET /destroy' do
    it 'returns http success' do
      get '/playlist_mp3s/destroy'
      expect(response).to have_http_status(:success)
    end
  end
end
