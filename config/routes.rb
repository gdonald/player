# frozen_string_literal: true

Rails.application.routes.draw do
  resources :playlists
  resources :playlist_mp3s do
    member do
      post :move_up
      post :move_down
    end
  end
  resources :mp3s, only: %i[index edit update] do
    collection do
      get :search
      get :sync
    end
    member do
      get :play
      get :src
    end
  end

  # mount GoodJob::Engine => 'good_job'

  root 'mp3s#index'
end
