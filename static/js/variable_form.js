// Variable form JavaScript

document.addEventListener('DOMContentLoaded', function() {
    const variableForm = document.getElementById('variable-form');
    if (variableForm) {
        variableForm.addEventListener('submit', function(e) {
            e.preventDefault();
            const formData = new FormData(e.target);
            const params = new URLSearchParams();
            for (const [key, value] of formData.entries()) {
                if (value) {
                    params.append(key, value);
                }
            }
            const alias = variableForm.dataset.alias;
            window.location.href = '/f/' + alias + '?' + params.toString();
        });
    }
});
