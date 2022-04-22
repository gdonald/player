# frozen_string_literal: true

require 'taglib'

class Mp3 < ApplicationRecord
  belongs_to :source, counter_cache: true

  validates :filepath, presence: true

  scope :ordered, -> { order(:artist, :album, :title) }

  scope :search, lambda { |query|
    return all if query.blank?

    query = query.downcase.gsub(/[^-_0-9a-z "]/, '')

    phrases = query.scan(/"(.+?)"/).flatten
    query = query.gsub(/"(.+?)"/, '').strip

    parts = query.include?(' ') ? query.split : [query]
    parts += phrases unless phrases.empty?
    parts = parts.select(&:present?)

    ors = parts.map { |p| "artist ILIKE '%#{p}%' OR album ILIKE '%#{p}%' OR title ILIKE '%#{p}%'" }
    where(ors.join(' OR '))
  }

  def duration
    Time.at(length).utc.strftime('%M:%S')
  end
end
