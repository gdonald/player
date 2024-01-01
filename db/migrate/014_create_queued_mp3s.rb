# frozen_string_literal: true

class CreateQueuedMp3s < ActiveRecord::Migration[7.1]
  def change
    create_table :queued_mp3s do |t|
      t.integer :mp3_id
      t.integer :position, null: false, default: 0
      t.timestamps
    end
    add_index :queued_mp3s, :position, unique: true
  end
end
