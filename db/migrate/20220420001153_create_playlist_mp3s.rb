# frozen_string_literal: true

class CreatePlaylistMp3s < ActiveRecord::Migration[7.0]
  def change
    create_table :playlist_mp3s do |t|
      t.references :playlist
      t.references :mp3
      t.integer :position
      t.timestamps
    end
    add_index :playlist_mp3s, %i[playlist_id mp3_id], unique: true
  end
end
