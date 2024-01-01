# frozen_string_literal: true

class AddSourceState < ActiveRecord::Migration[7.1]
  def change
    add_column :sources, :state, :string, null: false, default: 'unscanned'
    add_index :sources, :state
  end
end
