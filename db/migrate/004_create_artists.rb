# frozen_string_literal: true

class CreateArtists < ActiveRecord::Migration[7.0]
  def change
    create_table :artists do |t|
      t.string :name, null: false
      t.integer :mp3s_count, null: false, default: 0
      t.timestamps
    end
    add_index :artists, :name, unique: true
  end
end
