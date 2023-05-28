import { Controller } from "@hotwired/stimulus"

export default class extends Controller {
  connect() {
    const url = this.data.get('url')

    $('#search_query').keyup((event) => {
      if ( event.which == 13 ) { event.preventDefault() }
      this.search(url)
    })

    this.search(url)

    $(document).on('change', 'input#all', function() {
      var checkboxes = $('tbody.albums').find(':checkbox');
      checkboxes.prop('checked', $(this).is(':checked'));
    });
  }

  search(url) {
    $.ajax({
      url: url + '?q=' + $('#search_query').val(),
      dataType: 'script'
    })
  }

  doSearch(event) {
    const entity = event.params.entity
    const term = event.params.term
    $('#search_query').val(entity + ':"' + term + '"')

    const url = this.data.get('url')
    this.search(url)
  }
}
