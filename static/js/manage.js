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
            <input type="text" name="nested_alias[]" required placeholder="e.g., r" style="width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px;">
        </div>

        <div class="form-group">
            <label>Type:</label>
            <select name="nested_type[]" class="nested-type-select" style="width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px;">
                <option value="simple">Simple Bookmark</option>
                <option value="templated">Search Template</option>
            </select>
        </div>

        <div class="form-group">
            <label>URL:</label>
            <input type="url" name="nested_url[]" required placeholder="https://example.com" style="width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px;">
        </div>

        <div class="form-group">
            <label>Description:</label>
            <input type="text" name="nested_description[]" required placeholder="What this sub-command does" style="width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px;">
        </div>

        <div class="form-group nested-template-field" style="display: none;">
            <label>Template (with {}):</label>
            <input type="text" name="nested_template[]" placeholder="https://example.com/search?q={}" style="width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px;">
        </div>

        <div class="form-group nested-encode-field" style="display: none;">
            <label>
                <input type="checkbox" name="nested_encode[]" value="true" checked>
                URL-encode query
            </label>
        </div>
    `;

    container.appendChild(row);

    // Wire up event listeners for the newly created row
    const removeBtn = row.querySelector('.remove-nested-row');
    removeBtn.addEventListener('click', function() {
        removeNestedRow(this.dataset.rowId);
    });

    const typeSelect = row.querySelector('.nested-type-select');
    typeSelect.addEventListener('change', function() {
        toggleNestedTemplate(this);
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

function prepareNestedCommands() {
    console.log('prepareNestedCommands called');
    const bookmarkType = document.getElementById('bookmark_type').value;
    console.log('Bookmark type:', bookmarkType);

    if (bookmarkType !== 'nested') {
        // Not a nested bookmark, clear the JSON field
        document.getElementById('nested_commands_json').value = '';
        console.log('Not nested, cleared JSON');
        return;
    }

    // Collect all nested command data
    const nestedCommands = [];
    const container = document.getElementById('nested-commands-list');
    const rows = container.querySelectorAll('div[id^="nested-row-"]');
    console.log('Found rows:', rows.length);

    rows.forEach((row, index) => {
        const aliasInput = row.querySelector('input[name="nested_alias[]"]');
        const typeSelect = row.querySelector('select[name="nested_type[]"]');
        const urlInput = row.querySelector('input[name="nested_url[]"]');
        const descInput = row.querySelector('input[name="nested_description[]"]');
        const templateInput = row.querySelector('input[name="nested_template[]"]');
        const encodeCheckbox = row.querySelector('input[name="nested_encode[]"]');

        nestedCommands.push({
            alias: aliasInput ? aliasInput.value : '',
            type: typeSelect ? typeSelect.value : 'simple',
            url: urlInput ? urlInput.value : '',
            description: descInput ? descInput.value : '',
            template: templateInput ? templateInput.value : null,
            encode: encodeCheckbox ? encodeCheckbox.checked : true
        });
    });

    // Serialize to JSON and set in hidden field
    const jsonValue = JSON.stringify(nestedCommands);
    document.getElementById('nested_commands_json').value = jsonValue;
    console.log('Prepared nested commands:', nestedCommands);
    console.log('JSON value:', jsonValue);
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
                <label>Type:</label>
                <select name="nested_type" class="nested-form-type-select">
                    <option value="simple">Simple Bookmark</option>
                    <option value="templated">Search Template</option>
                </select>
            </div>

            <div class="form-group">
                <label>URL:</label>
                <input type="url" name="url" required placeholder="https://example.com">
            </div>

            <div class="form-group">
                <label>Description:</label>
                <input type="text" name="description" required>
            </div>

            <div class="form-group nested-template-field" style="display: none;">
                <label>Template (with {}):</label>
                <input type="text" name="command_template" placeholder="https://example.com/search?q={query}">
                <small>Use {var}, {var?}, {var=default} for variables. Add |!encode to disable URL encoding.</small>
            </div>

            <button type="submit" class="btn-primary">Add Sub-command</button>
        </form>
    `;

    document.getElementById('nested-manager').style.display = 'block';

    // Re-initialize HTMX on the new content
    htmx.process(contentDiv);

    // Wire up event listener for the nested form type select
    const nestedFormTypeSelect = contentDiv.querySelector('.nested-form-type-select');
    if (nestedFormTypeSelect) {
        nestedFormTypeSelect.addEventListener('change', function() {
            toggleNestedFormFields(this);
        });
    }
}

function toggleNestedFormFields(selectElement) {
    const form = selectElement.closest('form');
    const templateField = form.querySelector('.nested-template-field');

    if (selectElement.value === 'templated') {
        templateField.style.display = 'block';
    } else {
        templateField.style.display = 'none';
    }
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

// Multi-select functions for personal bookmarks
function toggleAllPersonal(checkbox) {
    const checkboxes = document.querySelectorAll('.personal-checkbox');
    checkboxes.forEach(cb => cb.checked = checkbox.checked);
    updatePersonalSelection();
}

function updatePersonalSelection() {
    const checkboxes = document.querySelectorAll('.personal-checkbox:checked');
    const count = checkboxes.length;
    const deleteBtn = document.getElementById('delete-selected-btn');
    const countSpan = document.getElementById('personal-selected-count');

    deleteBtn.disabled = count === 0;
    countSpan.textContent = count > 0 ? `${count} selected` : '';
}

function deleteSelectedPersonal() {
    const checkboxes = document.querySelectorAll('.personal-checkbox:checked');
    const ids = Array.from(checkboxes).map(cb => cb.value);

    if (ids.length === 0) return;

    if (!confirm(`Delete ${ids.length} selected bookmark(s)?`)) return;

    // Trigger individual delete buttons via HTMX (reuse existing working functionality)
    ids.forEach(id => {
        const row = document.getElementById(`bookmark-${id}`);
        if (row) {
            const deleteButton = row.querySelector('button[hx-delete]');
            if (deleteButton) {
                // Programmatically trigger the HTMX delete button
                htmx.trigger(deleteButton, 'click');
            }
        }
    });

    // Uncheck select all after a short delay
    setTimeout(() => {
        document.getElementById('select-all-personal').checked = false;
        updatePersonalSelection();
    }, 300);
}

// Multi-select functions for global bookmarks
function toggleAllGlobal(checkbox) {
    const checkboxes = document.querySelectorAll('.global-checkbox');
    checkboxes.forEach(cb => cb.checked = checkbox.checked);
    updateGlobalSelection();
}

function updateGlobalSelection() {
    const checkboxes = document.querySelectorAll('.global-checkbox:checked');
    const count = checkboxes.length;
    const disableBtn = document.getElementById('disable-selected-btn');
    const enableBtn = document.getElementById('enable-selected-btn');
    const countSpan = document.getElementById('global-selected-count');

    disableBtn.disabled = count === 0;
    enableBtn.disabled = count === 0;
    countSpan.textContent = count > 0 ? `${count} selected` : '';
}

function disableSelectedGlobal() {
    const checkboxes = document.querySelectorAll('.global-checkbox:checked');
    const aliases = Array.from(checkboxes).map(cb => cb.value);

    if (aliases.length === 0) return;

    if (!confirm(`Disable ${aliases.length} selected bookmark(s)?`)) return;

    // Trigger individual disable buttons via HTMX (reuse existing functionality)
    aliases.forEach(alias => {
        const row = document.getElementById(`global-${alias}`);
        if (row) {
            const statusCell = row.querySelector(`#status-${alias}`);
            // Only disable if currently active
            if (statusCell && statusCell.textContent.includes('✓ Active')) {
                const disableButton = row.querySelector('button.btn-secondary');
                if (disableButton && disableButton.textContent.includes('Disable')) {
                    // Programmatically click the disable button (HTMX will handle update)
                    disableButton.click();
                }
            }
        }
    });

    // Uncheck all after HTMX finishes (give it time to process)
    setTimeout(() => {
        document.getElementById('select-all-global').checked = false;
        document.querySelectorAll('.global-checkbox').forEach(cb => cb.checked = false);
        updateGlobalSelection();
    }, 500);
}

function enableSelectedGlobal() {
    const checkboxes = document.querySelectorAll('.global-checkbox:checked');
    const aliases = Array.from(checkboxes).map(cb => cb.value);

    if (aliases.length === 0) return;

    if (!confirm(`Enable ${aliases.length} selected bookmark(s)?`)) return;

    // Trigger individual enable buttons via HTMX (reuse existing functionality)
    aliases.forEach(alias => {
        const row = document.getElementById(`global-${alias}`);
        if (row) {
            const statusCell = row.querySelector(`#status-${alias}`);
            // Only enable if currently disabled
            if (statusCell && statusCell.textContent.includes('✗ Disabled')) {
                const enableButton = row.querySelector('button.btn-primary');
                if (enableButton && enableButton.textContent.includes('Enable')) {
                    // Programmatically click the enable button (HTMX will handle update)
                    enableButton.click();
                }
            }
        }
    });

    // Uncheck all after HTMX finishes
    setTimeout(() => {
        document.getElementById('select-all-global').checked = false;
        document.querySelectorAll('.global-checkbox').forEach(cb => cb.checked = false);
        updateGlobalSelection();
    }, 500);
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

    // Wire up personal bookmark bulk actions
    const deleteSelectedBtn = document.getElementById('delete-selected-btn');
    if (deleteSelectedBtn) {
        deleteSelectedBtn.addEventListener('click', deleteSelectedPersonal);
    }

    const selectAllPersonal = document.getElementById('select-all-personal');
    if (selectAllPersonal) {
        selectAllPersonal.addEventListener('change', function() {
            toggleAllPersonal(this);
        });
    }

    // Wire up personal checkboxes (use event delegation for dynamically created elements)
    document.addEventListener('change', function(e) {
        if (e.target.classList.contains('personal-checkbox')) {
            updatePersonalSelection();
        }
    });

    // Wire up global bookmark bulk actions
    const disableSelectedBtn = document.getElementById('disable-selected-btn');
    if (disableSelectedBtn) {
        disableSelectedBtn.addEventListener('click', disableSelectedGlobal);
    }

    const enableSelectedBtn = document.getElementById('enable-selected-btn');
    if (enableSelectedBtn) {
        enableSelectedBtn.addEventListener('click', enableSelectedGlobal);
    }

    const selectAllGlobal = document.getElementById('select-all-global');
    if (selectAllGlobal) {
        selectAllGlobal.addEventListener('change', function() {
            toggleAllGlobal(this);
        });
    }

    // Wire up global checkboxes (use event delegation for dynamically created elements)
    document.addEventListener('change', function(e) {
        if (e.target.classList.contains('global-checkbox')) {
            updateGlobalSelection();
        }
    });

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

    // Add HTMX event listener to prepare nested commands before submission
    const createForm = document.getElementById('create-bookmark-form');
    console.log('Create form found:', createForm !== null);
    if (createForm) {
        createForm.addEventListener('htmx:configRequest', function(event) {
            console.log('htmx:configRequest event fired');
            prepareNestedCommands();
        });
        console.log('HTMX event listener added');
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

    // Wire up nested manager buttons using data attributes (XSS-safe)
    document.querySelectorAll('.btn-nested-manager').forEach(function(btn) {
        btn.addEventListener('click', function() {
            showNestedManager(this.dataset.id, this.dataset.alias);
        });
    });
});
