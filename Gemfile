# frozen_string_literal: true

source 'https://rubygems.org'
git_source(:github) { |repo| "https://github.com/#{repo}.git" }

ruby '3.3.0'

gem 'rails', '~> 7.1.0'

gem 'cssbundling-rails'
gem 'jsbundling-rails'
gem 'pg'
gem 'puma', '~> 6.4.2'
gem 'sprockets-rails'
gem 'stimulus-rails', '~> 1.3.0'

gem 'acts_as_list'
gem 'good_job', '~> 3.22.0'
gem 'nokogiri', '>= 1.13.6'
gem 'simple_form'
gem 'taglib-ruby'

group :development, :test do
  gem 'debug', platforms: %i[mri mingw x64_mingw]
  gem 'dotenv-rails'
  gem 'pry'
  gem 'rspec-rails'
end

group :development do
  gem 'web-console'
end

group :test do
  gem 'capybara'
  gem 'rubocop'
  gem 'rubocop-rails'
  gem 'rubocop-rspec'
  gem 'selenium-webdriver', '~> 4.16.0'
end
