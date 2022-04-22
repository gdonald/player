class CreateSources < ActiveRecord::Migration[7.0]
  def change
    create_table :sources do |t|
      t.string :path, null: false
      t.integer :mp3s_count, null: false, default: 0
      t.timestamps
    end
    add_index :sources, :path, unique: true
  end
end
