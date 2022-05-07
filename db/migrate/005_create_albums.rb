# frozen_string_literal: true

class CreateAlbums < ActiveRecord::Migration[7.0]
  def change
    create_table :albums do |t|
      t.references :artist, null: false
      t.string :name, null: false
      t.integer :mp3s_count, null: false, default: 0
      t.timestamps
    end
    add_index :albums, %i[artist_id name], unique: true
  end
end
