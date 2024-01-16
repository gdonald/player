# frozen_string_literal: true

class CreateUsers < ActiveRecord::Migration[7.1]
  def change
    create_table :users do |t|
      t.string :username, null: false, limit: 32
      t.string :p_salt, limit: 80
      t.string :p_hash, limit: 80
      t.timestamps null: false
    end
    add_index :users, :username, unique: true
  end
end
