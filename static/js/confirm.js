// Themed confirmation modal for htmx requests.
//
// Destructive actions (delete a bookmark, delete a nested
// sub-command, revoke an API token, delete a user) are guarded by
// hx-confirm attributes. Stock htmx answers those with the
// browser's native confirm() dialog, which clashes with the site's
// theme and cannot be driven by browser automation. htmx fires an
// "htmx:confirm" event before falling back to the native dialog;
// intercepting it here lets us present the site's own modal
// instead. The hx-confirm attributes are unchanged — this only
// replaces the presentation. Pages without the modal markup keep
// the native behavior.

(function () {
    var pendingIssueRequest = null;

    function getModal() {
        return document.getElementById('confirm-modal');
    }

    function closeModal(confirmed) {
        var modal = getModal();
        if (modal) {
            modal.hidden = true;
        }
        var issue = pendingIssueRequest;
        pendingIssueRequest = null;
        if (confirmed && issue) {
            // true = skip htmx's own confirmation (we just did it).
            issue(true);
        }
    }

    document.addEventListener('htmx:confirm', function (e) {
        var modal = getModal();
        if (!modal) {
            return; // No modal on this page: let htmx use its native dialog.
        }
        e.preventDefault();
        document.getElementById('confirm-modal-message').textContent = e.detail.question;
        pendingIssueRequest = e.detail.issueRequest;
        modal.hidden = false;
        var okButton = document.getElementById('confirm-modal-ok');
        if (okButton) {
            okButton.focus();
        }
    });

    document.addEventListener('DOMContentLoaded', function () {
        var modal = getModal();
        if (!modal) {
            return;
        }
        document
            .getElementById('confirm-modal-ok')
            .addEventListener('click', function () {
                closeModal(true);
            });
        document
            .getElementById('confirm-modal-cancel')
            .addEventListener('click', function () {
                closeModal(false);
            });
        // Clicking the backdrop dismisses without confirming.
        modal.addEventListener('click', function (e) {
            if (e.target === modal) {
                closeModal(false);
            }
        });
        document.addEventListener('keydown', function (e) {
            if (e.key === 'Escape' && !modal.hidden) {
                closeModal(false);
            }
        });
    });
})();
