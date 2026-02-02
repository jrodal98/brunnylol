function showTab(tabName) {
    // Hide all tabs
    document.querySelectorAll('.tab-content').forEach(content => {
        content.classList.remove('active');
    });
    document.querySelectorAll('.tab').forEach(tab => {
        tab.classList.remove('active');
    });

    // Show selected tab
    document.getElementById(tabName).classList.add('active');
    event.target.classList.add('active');
}

function toggleFields() {
    const bookmarkType = document.getElementById('bookmark_type').value;
    const templatedFields = document.getElementById('templated-fields');
    const nestedInfo = document.getElementById('nested-info');

    // Hide all dynamic fields
    templatedFields.classList.remove('show');
    nestedInfo.classList.remove('show');

    // Show relevant fields
    if (bookmarkType === 'templated') {
        templatedFields.classList.add('show');
    } else if (bookmarkType === 'nested') {
        nestedInfo.classList.add('show');
    }
}

let nestedCounter = 0;

function addNestedRow() {
    console.log('addNestedRow called, counter:', nestedCounter);
    const container = document.getElementById('nested-commands-list');
    console.log('Container found:', container);
    if (!container) {
        alert('ERROR: nested-commands-list not found!');
        return;
    }
    const rowId = `nested-row-${nestedCounter++}`;
    console.log('Creating row:', rowId);

    const row = document.createElement('div');
    row.id = rowId;
    row.style.border = '2px solid #667eea';
    row.style.padding = '20px';
    row.style.marginBottom = '15px';
    row.style.borderRadius = '8px';
    row.style.backgroundColor = '#2a2d3a';
    row.style.boxShadow = '0 2px 8px rgba(102, 126, 234, 0.3)';

    row.innerHTML = `
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 15px;">
            <h4 style="margin: 0; color: #8b9aff;">Sub-command #${nestedCounter}</h4>
            <button type="button" class="btn-danger remove-nested-row" data-row-id="${rowId}" style="padding: 6px 12px; font-size: 13px;">Remove</button>
        </div>

        <div class="form-group">
            <label>Sub-alias:</label>
            <input type="text" name="nested_alias[]" required placeholder="sub1" style="width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px;" maxlength="50">
        </div>

        <div class="form-group">
            <label>URL:</label>
            <input type="url" name="nested_url[]" required placeholder="https://example.com" style="width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px;">
        </div>

        <div class="form-group">
            <label>Description:</label>
            <input type="text" name="nested_description[]" required placeholder="What this sub-command does" style="width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px;">
        </div>

        <div class="form-group">
            <label>Template (optional):</label>
            <input type="text" name="nested_command_template[]" placeholder="{url}/search?q={query}" style="width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px;">
            <small>Leave empty for simple redirect. Use {var}, {var?}, {var=default} for variables. Add |!encode to disable URL encoding.</small>
        </div>
    `;

    container.appendChild(row);

    // Wire up event listeners for the newly created row
    const removeBtn = row.querySelector('.remove-nested-row');
    removeBtn.addEventListener('click', function() {
        removeNestedRow(this.dataset.rowId);
    });

    console.log('Row appended to container, total children:', container.children.length);
}

function removeNestedRow(rowId) {
    document.getElementById(rowId).remove();
}

function toggleNestedTemplate(selectElement) {
    const row = selectElement.closest('div[id^="nested-row-"]');
    const templateField = row.querySelector('.nested-template-field');
    const encodeField = row.querySelector('.nested-encode-field');

    if (selectElement.value === 'templated') {
        templateField.style.display = 'block';
        encodeField.style.display = 'block';
    } else {
        templateField.style.display = 'none';
        encodeField.style.display = 'none';
    }
}

function showNestedManager(bookmarkId, alias) {
    document.getElementById('nested-manager-title').textContent = `Manage Nested Commands for '${alias}'`;

    // Load existing nested commands via HTMX
    const contentDiv = document.getElementById('nested-manager-content');
    contentDiv.innerHTML = `
        <div id="nested-list" hx-get="/manage/bookmark/${bookmarkId}/nested/list"
             hx-trigger="load, nested-added from:body"
             hx-swap="innerHTML">
            Loading...
        </div>

        <h4 style="margin-top: 20px;">Add New Sub-command</h4>
        <div id="nested-add-result" style="margin-bottom: 15px;"></div>
        <form hx-post="/manage/bookmark/${bookmarkId}/nested"
              hx-target="#nested-add-result"
              hx-swap="innerHTML"
              hx-on::after-request="if(event.detail.successful) { htmx.trigger('#nested-list', 'nested-added'); this.reset(); }"
              style="margin-top: 15px;">
            <input type="hidden" name="parent_id" value="${bookmarkId}">

            <div class="form-group">
                <label>Sub-alias:</label>
                <input type="text" name="alias" required placeholder="e.g., r">
            </div>

            <div class="form-group">
                <label>URL:</label>
                <input type="url" name="url" required placeholder="https://example.com">
            </div>

            <div class="form-group">
                <label>Description:</label>
                <input type="text" name="description" required>
            </div>

            <div class="form-group">
                <label>Template (optional):</label>
                <input type="text" name="command_template" placeholder="{url}/search?q={query}">
                <small>Leave empty for simple redirect. Use {var}, {var?}, {var=default} for variables. Add |!encode to disable URL encoding.</small>
            </div>

            <button type="submit" class="btn-primary">Add Sub-command</button>
        </form>
    `;

    document.getElementById('nested-manager').style.display = 'block';

    // Re-initialize HTMX on the new content
    htmx.process(contentDiv);
}

function hideNestedManager() {
    document.getElementById('nested-manager').style.display = 'none';
}

function showEditForm(id, alias, url, description, template) {
    document.getElementById('edit-id').value = id;
    document.getElementById('edit-alias').value = alias;
    document.getElementById('edit-url').value = url;
    document.getElementById('edit-description').value = description;
    document.getElementById('edit-template').value = template || '';

    // Set the form action URL
    document.getElementById('edit-form').setAttribute('hx-put', `/manage/bookmark/${id}`);
    htmx.process(document.getElementById('edit-form'));

    document.getElementById('edit-modal').style.display = 'block';
}

function hideEditForm() {
    document.getElementById('edit-modal').style.display = 'none';
    document.getElementById('edit-result').innerHTML = '';
    // Reset readonly state
    document.getElementById('edit-alias').readOnly = false;
}

function showEditFormGlobal(alias, url, template, description) {
    // Reuse edit form but for global bookmarks
    document.getElementById('edit-id').value = ''; // No ID for global
    document.getElementById('edit-alias').value = alias;
    document.getElementById('edit-alias').readOnly = false; // Allow changing alias
    document.getElementById('edit-url').value = url;
    document.getElementById('edit-description').value = description;
    document.getElementById('edit-template').value = template || '';

    // Set form action for global bookmark
    document.getElementById('edit-form').setAttribute('hx-put', `/admin/bookmark/${alias}`);
    htmx.process(document.getElementById('edit-form'));

    document.getElementById('edit-modal').style.display = 'block';
}

function toggleImportFields() {
    const source = document.getElementById('import-source').value;
    const pasteDiv = document.getElementById('import-paste');
    const urlDiv = document.getElementById('import-url');

    if (source === 'paste') {
        pasteDiv.style.display = 'block';
        urlDiv.style.display = 'none';
    } else if (source === 'url') {
        pasteDiv.style.display = 'none';
        urlDiv.style.display = 'block';
    }
}

// Initialize
document.addEventListener('DOMContentLoaded', function() {
    console.log('DOMContentLoaded fired');
    toggleFields();

    // Wire up tab buttons
    document.querySelectorAll('.tab').forEach(function(tab) {
        tab.addEventListener('click', function(event) {
            const tabName = this.dataset.tab;
            // Hide all tabs
            document.querySelectorAll('.tab-content').forEach(content => {
                content.classList.remove('active');
            });
            document.querySelectorAll('.tab').forEach(t => {
                t.classList.remove('active');
            });
            // Show selected tab
            document.getElementById(tabName).classList.add('active');
            this.classList.add('active');
        });
    });

    // Wire up bookmark type select
    const bookmarkTypeSelect = document.getElementById('bookmark_type');
    if (bookmarkTypeSelect) {
        bookmarkTypeSelect.addEventListener('change', toggleFields);
    }

    // Wire up add nested button
    const addNestedBtn = document.getElementById('add-nested-btn');
    if (addNestedBtn) {
        addNestedBtn.addEventListener('click', addNestedRow);
    }

    // Wire up import source select
    const importSourceSelect = document.getElementById('import-source');
    if (importSourceSelect) {
        importSourceSelect.addEventListener('change', toggleImportFields);
    }

    // Wire up export buttons
    document.querySelectorAll('.export-btn').forEach(function(btn) {
        btn.addEventListener('click', function() {
            window.location.href = this.dataset.exportUrl;
        });
    });

    // Wire up nested manager close button
    const closeNestedManager = document.getElementById('close-nested-manager');
    if (closeNestedManager) {
        closeNestedManager.addEventListener('click', hideNestedManager);
    }

    // Wire up edit modal cancel button
    const cancelEditBtn = document.getElementById('cancel-edit-btn');
    if (cancelEditBtn) {
        cancelEditBtn.addEventListener('click', hideEditForm);
    }

    // Add submit event listener to prepare nested commands BEFORE HTMX processes
    const createForm = document.getElementById('create-bookmark-form');
    console.log('Create form found:', createForm !== null);
    if (createForm) {
        // Use CAPTURE PHASE (third parameter = true) to run before HTMX's listeners
        createForm.addEventListener('submit', function(event) {
            console.log('submit event fired (CAPTURE PHASE)');
            const bookmarkType = document.getElementById('bookmark_type').value;

            if (bookmarkType === 'nested') {
                // Collect nested commands data
                const nestedCommands = [];
                const container = document.getElementById('nested-commands-list');
                const rows = container.querySelectorAll('div[id^="nested-row-"]');
                console.log('Found rows:', rows.length);

                rows.forEach((row, index) => {
                    const aliasInput = row.querySelector('input[name="nested_alias[]"]');
                    const urlInput = row.querySelector('input[name="nested_url[]"]');
                    const descInput = row.querySelector('input[name="nested_description[]"]');
                    const templateInput = row.querySelector('input[name="nested_command_template[]"]');

                    nestedCommands.push({
                        alias: aliasInput ? aliasInput.value : '',
                        url: urlInput ? urlInput.value : '',
                        description: descInput ? descInput.value : '',
                        command_template: templateInput && templateInput.value ? templateInput.value : null
                    });
                });

                const jsonValue = JSON.stringify(nestedCommands);
                console.log('Prepared nested commands:', nestedCommands);
                console.log('JSON value:', jsonValue);

                // Set the hidden field SYNCHRONOUSLY before HTMX reads the form
                const hiddenField = document.getElementById('nested_commands_json');
                hiddenField.value = jsonValue;
                console.log('Set hidden field value to:', hiddenField.value);
            }
        }, true); // USE CAPTURE PHASE - this is the key!
        console.log('Submit event listener added (capture phase)');
    }

    // Wire up edit buttons using data attributes (XSS-safe)
    document.querySelectorAll('.btn-edit').forEach(function(btn) {
        btn.addEventListener('click', function() {
            showEditForm(
                this.dataset.id,
                this.dataset.alias,
                this.dataset.url,
                this.dataset.description,
                this.dataset.template
            );
        });
    });

    // Wire up global edit buttons for admins
    document.querySelectorAll('.btn-edit-global').forEach(function(btn) {
        btn.addEventListener('click', function() {
            showEditFormGlobal(
                this.dataset.alias,
                this.dataset.url,
                this.dataset.template,
                this.dataset.description
            );
        });
    });

    // Wire up nested manager buttons using data attributes (XSS-safe)
    document.querySelectorAll('.btn-nested-manager').forEach(function(btn) {
        btn.addEventListener('click', function() {
            showNestedManager(this.dataset.id, this.dataset.alias);
        });
    });

    // Re-attach event listeners after HTMX swaps content
    document.body.addEventListener('htmx:afterSwap', function(event) {
        // Re-wire global edit buttons after row swap
        document.querySelectorAll('.btn-edit-global').forEach(function(btn) {
            if (!btn.hasAttribute('data-listener-attached')) {
                btn.setAttribute('data-listener-attached', 'true');
                btn.addEventListener('click', function() {
                    showEditFormGlobal(
                        this.dataset.alias,
                        this.dataset.url,
                        this.dataset.template,
                        this.dataset.description
                    );
                });
            }
        });
    });
});
