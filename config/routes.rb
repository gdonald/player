# frozen_string_literal: true

Rails.application.routes.draw do
  resources :mp3s, only: %i[index edit update] do
    collection do
      get :search
    end
  end

  root 'mp3s#index'
end
