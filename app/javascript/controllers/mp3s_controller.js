import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  connect() {
    const url = this.data.get('url')

    $("#search_query").keyup(function() {
      if ( event.which == 13 ) { event.preventDefault() }

      $.ajax({
        url: url + '?q=' + $('#search_query').val(),
        dataType: 'script'
      })
    })
  }
}
