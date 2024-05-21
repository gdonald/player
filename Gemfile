# frozen_string_literal: true

source 'https://rubygems.org'
git_source(:github) { |repo| "https://github.com/#{repo}.git" }

ruby '3.3.0'

gem 'aasm', '~> 5.5.0'
gem 'bcrypt', '~> 3.1.18'
gem 'rails', '~> 7.1.3'

gem 'cssbundling-rails'
gem 'jbuilder'
gem 'jsbundling-rails'
gem 'pg'
gem 'sprockets-rails'

gem 'acts_as_list'
gem 'good_job', '~> 3.28.3'
gem 'nokogiri', '>= 1.13.6'

gem 'taglib-ruby'

group :development, :test do
  gem 'debug', platforms: %i[mri mingw x64_mingw]
  gem 'dotenv-rails'
  gem 'pry'
  gem 'puma', '~> 6.4.2'
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
  gem 'selenium-webdriver', '~> 4.21.1'
end
