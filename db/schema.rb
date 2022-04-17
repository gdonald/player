# This file is auto-generated from the current state of the database. Instead
# of editing this file, please use the migrations feature of Active Record to
# incrementally modify your database, and then regenerate this schema definition.
#
# This file is the source Rails uses to define your schema when running `bin/rails
# db:schema:load`. When creating a new database, `bin/rails db:schema:load` tends to
# be faster and is potentially less error prone than running all of your
# migrations from scratch. Old migrations may fail to apply correctly if those
# migrations use external dependencies or application code.
#
# It's strongly recommended that you check this file into your version control system.

ActiveRecord::Schema[7.0].define(version: 2022_04_17_150228) do
  # These are extensions that must be enabled in order to support this database
  enable_extension "plpgsql"

  create_table "mp3s", force: :cascade do |t|
    t.string "filepath", null: false
    t.string "title"
    t.string "artist"
    t.string "album"
    t.string "genre"
    t.integer "year"
    t.integer "track"
    t.integer "length"
    t.text "comment"
    t.index ["album"], name: "index_mp3s_on_album"
    t.index ["artist"], name: "index_mp3s_on_artist"
    t.index ["genre"], name: "index_mp3s_on_genre"
    t.index ["title"], name: "index_mp3s_on_title"
    t.index ["year"], name: "index_mp3s_on_year"
  end

end