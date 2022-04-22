class Source < ApplicationRecord
  scope :ordered, -> { order(:path) }

  has_many :mp3s, -> { order(:filepath) }, dependent: :destroy

  def sync(truncate: false)
    if truncate
      Mp3.destroy_all
    else
      check_known
    end

    scan
  end

  private

  def scan
    Dir.glob("#{path}/**/*.mp3", File::FNM_CASEFOLD).each do |filepath|
      TagLib::FileRef.open(filepath) do |ref|
        if ref.nil?
          logger.warn("Cannot read file #{filepath}")
          next
        end

        mp3 = Mp3.find_by(filepath:)
        if mp3
          mp3.do_update(ref)
        else
          create_mp3(filepath, ref)
        end
      end
    end
  end

  def check_known
    Mp3.find_each do |mp3|
      unless File.readable?(mp3.filepath)
        logger.warn("Cannot find known file #{mp3.filepath}")
        mp3.destroy
      end
    end
  end

  def create_mp3(filepath, ref)
    tag = ref.tag
    properties = ref.audio_properties

    Mp3.create!(filepath:,
                source: self,
                title: tag.title,
                artist: tag.artist,
                album: tag.album,
                genre: tag.genre,
                year: tag.year,
                track: tag.track,
                length: properties.length_in_seconds,
                comment: tag.comment)
  end

  def do_update(ref)
    tag = ref.tag
    properties = ref.audio_properties

    update!(title: tag.title,
            artist: tag.artist,
            album: tag.album,
            genre: tag.genre,
            year: tag.year,
            track: tag.track,
            length: properties.length_in_seconds,
            comment: tag.comment)
  end
end
