# frozen_string_literal: true

source 'https://rubygems.org'
git_source(:github) { |repo| "https://github.com/#{repo}.git" }

ruby '3.1.2'

gem 'rails', '~> 7.1.0'

gem 'cssbundling-rails'
gem 'jsbundling-rails'
gem 'pg'
gem 'puma', '~> 6.4.0'
gem 'sprockets-rails'
gem 'stimulus-rails', '~> 1.2.2'
gem 'tzinfo-data', platforms: %i[mingw mswin x64_mingw jruby]

gem 'acts_as_list'
gem 'good_job', '~> 3.19.4'
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
  gem 'rubocop', '~> 1.56.1'
  gem 'rubocop-rails'
  gem 'rubocop-rspec', '~> 2.24.1'
  gem 'selenium-webdriver', '~> 4.13.1'
end
