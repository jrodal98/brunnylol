// Homepage search form JavaScript

document.addEventListener('DOMContentLoaded', function() {
    const searchForm = document.getElementById('search-form-Brunnylol');
    if (searchForm) {
        searchForm.addEventListener('submit', function(e) {
            e.preventDefault();
            const query = document.getElementById('search-bar-Brunnylol').value;
            if (query) {
                window.location.href = '/search?q=' + encodeURIComponent(query);
            }
        });
    }
});
