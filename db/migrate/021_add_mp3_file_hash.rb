# frozen_string_literal: true

class AddMp3FileHash < ActiveRecord::Migration[7.1]
  def change
    add_column :mp3s, :file_hash, :string
  end
end
