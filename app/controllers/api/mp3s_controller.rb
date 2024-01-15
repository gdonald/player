# frozen_string_literal: true

module Api
  class Mp3sController < Api::ApplicationController
    def index
      @mp3s = Mp3.ordered(params)
    end

    def search
      @mp3s = Mp3.search(params)
      render :index
    end

    def play
      mp3 = Mp3.find_by(id: params[:id])
      send_file(mp3.filepath)
    end

    def show
      @mp3 = Mp3.find_by(id: params[:id])
    end

    def update
      artist = find_artist
      album = find_album(artist:)
      @mp3 = update_mp3(artist:, album:)
      raise StandardError, @mp3.errors.messages unless @mp3.valid?

      update_tag(mp3: @mp3)
      @message = 'MP3 updated'
      render :show
    rescue StandardError
      render json: { errors: @mp3.errors,
                     message: 'Failed to update mp3' },
             status: :unprocessable_entity
    end

    private

    def find_artist
      artist = Artist.find_by(name: mp3_params[:artist])
      artist ||= Artist.create!(name: mp3_params[:artist])
      artist
    end

    def find_album(artist:)
      return unless artist

      album = artist.albums.find_by(name: mp3_params[:album])
      album ||= artist.albums.create!(name: mp3_params[:album])
      album
    end

    def update_mp3(artist:, album:)
      return unless artist && album

      mp3 = Mp3.find_by(id: params[:id])
      return unless mp3

      mp3.artist = artist
      mp3.album = album
      mp3.title = mp3_params[:title]
      mp3.track = mp3_params[:track]
      mp3.save

      mp3
    end

    def update_tag(mp3:)
      TagLib::MPEG::File.open(mp3.filepath) do |file|
        tag = file.id3v2_tag

        tag.title = mp3.title
        tag.album = mp3.album.name
        tag.artist = mp3.artist.name

        file.save
      end
    end

    def mp3_params
      params.require(:mp3).permit(:title, :album, :artist, :track)
    end
  end
end
