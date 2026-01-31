// Variable form JavaScript

document.addEventListener('DOMContentLoaded', function() {
    const variableForm = document.getElementById('variable-form');
    if (variableForm) {
        variableForm.addEventListener('submit', function(e) {
            e.preventDefault();
            const formData = new FormData(e.target);
            const alias = variableForm.dataset.alias;

            // Submit form via fetch POST
            fetch('/f/' + alias, {
                method: 'POST',
                body: new URLSearchParams(formData),
                headers: {
                    'Content-Type': 'application/x-www-form-urlencoded'
                }
            }).then(response => response.text())
              .then(url => {
                  // Server returns the redirect URL in the response body
                  window.location.href = url;
              });
        });
    }
});
