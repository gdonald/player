# Player
Web-based MP3 Player written in Ruby using Ruby on Rails, ReactJS, and Bootstrap.

![MP3 Player](https://raw.githubusercontent.com/gdonald/player/main/ss.png "MP3 Player")

## Installation

```bash
bundle install
bundle exec rails db:migrate
yarn install
yarn build
```

## Bundler Note:

On macOS I use the `TAGLIB_DIR` environment variable to get bundler to install the taglib-ruby gem:

```bash
brew install taglib

TAGLIB_DIR=/opt/homebrew/Cellar/taglib/1.13.1 bundle install
```

## Running

```bash
bundle exec rails server
```

Add a "source" directory to the root of your music collections.  Then "scan" it from the web interface.  Re-scan as needed.

## License

[![GitHub](https://img.shields.io/github/license/gdonald/player?color=0000bb)](https://github.com/gdonald/player/blob/main/LICENSE)
