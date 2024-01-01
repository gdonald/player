# frozen_string_literal: true

Rails.application.routes.draw do
  namespace :api do
    get :counts, to: 'counts#index'
    resources :queued_mp3s, only: %i[index create destroy]
    resources :mp3s, only: %i[index show update] do
      get :search, on: :collection
      get :play, on: :member
    end
    resources :playlists, only: %i[index show update create destroy] do
      post :enqueue, on: :member
      resources :playlist_mp3s, only: %i[index destroy] do
        member do
          post :move_higher
          post :move_lower
        end
      end
    end
    resources :sources, only: %i[index show update] do
      get :scan, on: :member
    end
  end

  mount GoodJob::Engine => 'good_job'

  root 'home#index'
end
