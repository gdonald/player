# frozen_string_literal: true

class CreateMp3s < ActiveRecord::Migration[7.0]
  def change
    create_table :mp3s do |t|
      t.references :source, null: false
      t.string :filepath, null: false, unique: true
      t.string :title
      t.string :artist
      t.string :album
      t.string :genre
      t.integer :year
      t.integer :track
      t.integer :length
      t.text :comment
    end
    add_index :mp3s, :title
    add_index :mp3s, :artist
    add_index :mp3s, :album
    add_index :mp3s, :genre
    add_index :mp3s, :year
  end
end
