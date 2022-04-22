# frozen_string_literal: true

class CreatePlaylists < ActiveRecord::Migration[7.0]
  def change
    create_table :playlists do |t|
      t.string :name, null: false
      t.integer :playlist_mp3s_count, null: false, default: 0
      t.timestamps
    end
  end
end
