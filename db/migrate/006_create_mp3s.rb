# frozen_string_literal: true

class CreateMp3s < ActiveRecord::Migration[7.0]
  def change # rubocop:disable Metrics/AbcSize
    create_table :mp3s do |t|
      t.references :source, null: false
      t.references :artist, null: false
      t.references :album, null: false
      t.string :filepath, null: false, unique: true
      t.string :title
      t.string :genre
      t.integer :year
      t.integer :track
      t.integer :length
      t.text :comment
      t.timestamps
    end
    add_index :mp3s, :title
    add_index :mp3s, %i[artist_id album_id title length], unique: true
    add_index :mp3s, :genre
    add_index :mp3s, :year
    add_index :mp3s, :track
    add_index :mp3s, :length
  end
end
