import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  connect() {
    $(window).on('popstate', function(event) {
      const audio = $('audio').get(0)
      if (audio !== undefined) { audio.remove() }
    })
  }
}
