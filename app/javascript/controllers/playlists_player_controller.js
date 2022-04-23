import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  connect() {
    const audio = $('audio').get(0)
    const url = this.data.get('url')

    if (audio !== undefined) {
      audio.addEventListener('ended', (event) => {
        $.ajax({
          url: url,
          dataType: 'script'
        })
      })
    }
  }
}
