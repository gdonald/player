import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  connect() {
    const url = this.data.get('url')

    $("#search_query").keyup( () => {
      if ( event.which == 13 ) { event.preventDefault() }
      this.search(url)
    })

    this.search(url)
  }

  search(url) {
    $.ajax({
      url: url + '?q=' + $('#search_query').val(),
      dataType: 'script'
    })
  }
}
