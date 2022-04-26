import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  connect() {
    const audio = $('audio').get(0)
    const url = this.data.get('url')
    this.play(audio, url)
  }

  play(audio, url) {
    if (audio === undefined) {
      $.ajax({
        url: url,
        dataType: 'script'
      })
    }
  }
}
