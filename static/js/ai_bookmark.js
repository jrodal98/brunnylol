// "Create with AI" flow for the Create Bookmark tab.
//
// State machine:
//   idle -> (Generate) -> proposal shown
//   proposal -> Accept    -> POSTs the proposal to the existing
//                            /manage/bookmark endpoint, then reloads
//   proposal -> Reject    -> discards proposal + conversation, back to idle
//   proposal -> Edit      -> prefills the manual create form with the
//                            proposal values and hides the proposal
//   proposal -> Follow-up -> sends an extra user message together with the
//                            conversation history + previous proposal to
//                            POST /api/ai/bookmark/propose and replaces
//                            the proposal with the revised one
//
// The server is stateless: this script owns the conversation history.

(function () {
    let history = []; // [{role: "user"|"assistant", content: string}]
    let currentProposal = null;

    function el(id) { return document.getElementById(id); }

    function escapeHtml(s) {
        return String(s == null ? '' : s)
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;')
            .replace(/'/g, '&#x27;');
    }

    function setStatus(text) {
        const status = el('ai-status');
        if (status) status.textContent = text || '';
    }

    function showError(text) {
        const box = el('ai-error');
        if (!box) return;
        if (text) {
            box.textContent = text;
            box.style.display = 'block';
        } else {
            box.style.display = 'none';
            box.textContent = '';
        }
    }

    function resetAi() {
        history = [];
        currentProposal = null;
        if (el('ai-prompt')) el('ai-prompt').value = '';
        if (el('ai-proposal')) el('ai-proposal').style.display = 'none';
        if (el('ai-followup-box')) el('ai-followup-box').style.display = 'none';
        if (el('ai-followup-text')) el('ai-followup-text').value = '';
        if (el('ai-accept-result')) el('ai-accept-result').innerHTML = '';
        showError('');
        setStatus('');
    }

    function renderProposal(proposal) {
        currentProposal = proposal;
        const fields = el('ai-proposal-fields');
        let html = '<table class="ai-proposal-table"><tbody>';
        html += '<tr><th>Alias</th><td><strong>' + escapeHtml(proposal.alias) + '</strong></td></tr>';
        html += '<tr><th>Type</th><td>' + escapeHtml(proposal.bookmark_type === 'nested' ? 'Nested Commands' : 'Standard Bookmark') + '</td></tr>';
        if (proposal.url) {
            html += '<tr><th>Base URL</th><td>' + escapeHtml(proposal.url) + '</td></tr>';
        }
        html += '<tr><th>Description</th><td>' + escapeHtml(proposal.description) + '</td></tr>';
        if (proposal.command_template) {
            html += '<tr><th>Template</th><td><code>' + escapeHtml(proposal.command_template) + '</code></td></tr>';
        }
        html += '</tbody></table>';

        if (proposal.bookmark_type === 'nested' && proposal.nested_commands && proposal.nested_commands.length) {
            html += '<h5>Sub-commands</h5><table class="ai-proposal-table"><thead><tr><th>Alias</th><th>URL</th><th>Description</th><th>Template</th></tr></thead><tbody>';
            proposal.nested_commands.forEach(function (n) {
                html += '<tr><td><strong>' + escapeHtml(n.alias) + '</strong></td><td>' + escapeHtml(n.url) +
                    '</td><td>' + escapeHtml(n.description) + '</td><td><code>' + escapeHtml(n.command_template || '') + '</code></td></tr>';
            });
            html += '</tbody></table>';
        }

        fields.innerHTML = html;
        el('ai-proposal').style.display = 'block';
        el('ai-followup-box').style.display = 'none';
        el('ai-accept-result').innerHTML = '';
    }

    async function requestProposal(message) {
        showError('');
        setStatus('Asking AI…');
        const generateBtn = el('ai-generate-btn');
        const followupBtn = el('ai-followup-submit-btn');
        if (generateBtn) generateBtn.disabled = true;
        if (followupBtn) followupBtn.disabled = true;
        try {
            const response = await fetch('/api/ai/bookmark/propose', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    message: message,
                    history: history,
                    previous_proposal: currentProposal,
                }),
            });
            if (!response.ok) {
                let messageText = 'AI request failed (' + response.status + ')';
                try {
                    const text = await response.text();
                    // AppError pages are HTML; pull the readable message out.
                    const match = text.replace(/<[^>]*>/g, ' ').match(/(Bad request|Forbidden|Internal error):[^\n<]*/);
                    if (match) messageText = match[0].trim();
                } catch (e) { /* keep default */ }
                throw new Error(messageText);
            }
            const data = await response.json();
            history.push({ role: 'user', content: message });
            history.push({ role: 'assistant', content: JSON.stringify(data.proposal) });
            renderProposal(data.proposal);
            setStatus('Proposed with ' + (data.model || 'AI') + (data.source ? ' (' + data.source + ' config)' : ''));
        } catch (err) {
            showError(err.message || 'Something went wrong asking the AI.');
            setStatus('');
        } finally {
            if (generateBtn) generateBtn.disabled = false;
            if (followupBtn) followupBtn.disabled = false;
        }
    }

    async function acceptProposal() {
        if (!currentProposal) return;
        const result = el('ai-accept-result');
        result.innerHTML = 'Creating bookmark…';
        const params = new URLSearchParams();
        params.set('alias', currentProposal.alias);
        params.set('bookmark_type', currentProposal.bookmark_type || 'templated');
        params.set('url', currentProposal.url || '');
        params.set('description', currentProposal.description);
        params.set('command_template', currentProposal.command_template || '');
        if (currentProposal.bookmark_type === 'nested') {
            params.set('nested_commands_json', JSON.stringify(currentProposal.nested_commands || []));
        }
        try {
            const response = await fetch('/manage/bookmark', {
                method: 'POST',
                headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
                body: params.toString(),
            });
            const text = await response.text();
            if (!response.ok) {
                result.innerHTML = '<span style="color:#d32f2f;">Could not create the bookmark — try Edit to fix it manually.</span>';
                return;
            }
            result.innerHTML = text;
            setTimeout(function () { window.location.reload(); }, 1000);
        } catch (err) {
            result.innerHTML = '<span style="color:#d32f2f;">Network error creating the bookmark.</span>';
        }
    }

    function editProposal() {
        if (!currentProposal) return;
        const p = currentProposal;
        el('alias').value = p.alias || '';
        const typeSelect = el('bookmark_type');
        typeSelect.value = p.bookmark_type === 'nested' ? 'nested' : 'templated';
        if (typeof toggleFields === 'function') toggleFields();
        el('url').value = p.url || '';
        el('description').value = p.description || '';
        if (el('command_template')) el('command_template').value = p.command_template || '';

        if (p.bookmark_type === 'nested' && p.nested_commands && p.nested_commands.length) {
            const list = el('nested-commands-list');
            if (list) list.innerHTML = '';
            p.nested_commands.forEach(function (n) {
                if (typeof addNestedRow === 'function') addNestedRow();
            });
            // Fill the rows just added, in order.
            const rows = list ? list.querySelectorAll('div[id^="nested-row-"]') : [];
            p.nested_commands.forEach(function (n, i) {
                const row = rows[i];
                if (!row) return;
                const set = function (name, value) {
                    const input = row.querySelector('input[name="' + name + '"]');
                    if (input) input.value = value || '';
                };
                set('nested_alias[]', n.alias);
                set('nested_url[]', n.url);
                set('nested_description[]', n.description);
                set('nested_command_template[]', n.command_template);
            });
        }

        el('ai-proposal').style.display = 'none';
        setStatus('Proposal loaded into the manual form below — adjust anything, then Create Bookmark.');
        el('alias').scrollIntoView({ behavior: 'smooth', block: 'center' });
        el('alias').focus();
    }

    document.addEventListener('DOMContentLoaded', function () {
        const generateBtn = el('ai-generate-btn');
        if (!generateBtn) return; // AI not available for this user; panel is inert.

        generateBtn.addEventListener('click', function () {
            const prompt = el('ai-prompt').value.trim();
            if (!prompt) {
                showError('Describe the bookmark you want first.');
                return;
            }
            // A fresh description starts a fresh conversation.
            history = [];
            currentProposal = null;
            requestProposal(prompt);
        });

        el('ai-accept-btn').addEventListener('click', acceptProposal);
        el('ai-reject-btn').addEventListener('click', resetAi);
        el('ai-edit-btn').addEventListener('click', editProposal);
        el('ai-followup-btn').addEventListener('click', function () {
            const box = el('ai-followup-box');
            box.style.display = box.style.display === 'none' ? 'block' : 'none';
            if (box.style.display === 'block') el('ai-followup-text').focus();
        });
        el('ai-followup-submit-btn').addEventListener('click', function () {
            const text = el('ai-followup-text').value.trim();
            if (!text) {
                showError('Write what should change first.');
                return;
            }
            el('ai-followup-text').value = '';
            requestProposal(text);
        });
    });
})();
