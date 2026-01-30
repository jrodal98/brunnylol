function searchTable() {
  var input, filter;
  input = document.getElementById("search");
  filter = input.value.toUpperCase();

  // Search both personal and global tables
  var tables = ["personal-aliases", "aliases"];

  tables.forEach(function(tableId) {
    var table = document.getElementById(tableId);
    if (!table) return;

    var tr = table.getElementsByTagName("tr");

    for (var i = 0; i < tr.length; i++) {
      var td1 = tr[i].getElementsByTagName("td")[0];
      var td2 = tr[i].getElementsByTagName("td")[1];
      if (td1 || td2) {
        var txtValue1 = td1 ? (td1.textContent || td1.innerText) : "";
        var txtValue2 = td2 ? (td2.textContent || td2.innerText) : "";
        if ((txtValue1.toUpperCase().indexOf(filter) > -1) || (txtValue2.toUpperCase().indexOf(filter) > -1)) {
          tr[i].style.display = "";
        } else {
          tr[i].style.display = "none";
        }
      }
    }
  });
}

// Wire up search input
document.addEventListener('DOMContentLoaded', function() {
  var searchInput = document.getElementById('search');
  if (searchInput) {
    searchInput.addEventListener('keyup', searchTable);
  }
});
