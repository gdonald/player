import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  connect() {
    const url = this.data.get('url')
    this.search(url)
  }

  search(url) {
    $.ajax({
      url: url + '?q=' + $('#search_query').val(),
      dataType: 'script'
    })
  }
}
