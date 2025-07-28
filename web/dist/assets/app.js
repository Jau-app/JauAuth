// JauAuth Dashboard Application
class JauAuthDashboard {
    constructor() {
        this.currentSection = 'overview';
        this.servers = [];
        this.tools = [];
        this.authToken = null;
        this.csrfToken = null;
        
        this.init();
        this.initTheme();
    }
    
    async init() {
        // Setup navigation
        this.setupNavigation();
        
        // Initialize server modal with app instance
        if (window.serverModal) {
            window.serverModal.init(this);
        }
        
        // Setup CSRF protection
        await this.setupSecurity();
        
        // Handle initial route
        this.handleRoute();
        
        // Setup auto-refresh
        this.startAutoRefresh();
        
        // Listen for hash changes
        window.addEventListener('hashchange', () => this.handleRoute());
    }
    
    async setupSecurity() {
        // Get CSRF token from meta tag or cookie
        const meta = document.querySelector('meta[name="csrf-token"]');
        if (meta) {
            this.csrfToken = meta.getAttribute('content');
        }
        
        // Setup default headers for all requests
        this.defaultHeaders = {
            'Content-Type': 'application/json',
            'X-CSRF-Token': this.csrfToken || ''
        };
        
        // Add auth token if available
        const token = localStorage.getItem('jau-auth-token');
        if (token) {
            this.authToken = token;
            this.defaultHeaders['Authorization'] = `Bearer ${token}`;
        }
    }
    
    setupNavigation() {
        // Handle navigation clicks
        document.querySelectorAll('.nav-link').forEach(link => {
            link.addEventListener('click', (e) => {
                e.preventDefault();
                const section = link.getAttribute('href').substring(1);
                window.location.hash = section;
            });
        });
    }
    
    handleRoute() {
        // Get current hash or default to overview
        const hash = window.location.hash.substring(1) || 'overview';
        this.showSection(hash);
    }
    
    showSection(section) {
        // Update active nav
        document.querySelectorAll('.nav-link').forEach(link => {
            link.classList.remove('active');
        });
        const navLink = document.querySelector(`a[href="#${section}"]`);
        if (navLink) {
            navLink.classList.add('active');
        }
        
        // Update active section
        document.querySelectorAll('.section').forEach(sec => {
            sec.classList.remove('active');
        });
        const sectionElement = document.getElementById(section);
        if (sectionElement) {
            sectionElement.classList.add('active');
        }
        
        this.currentSection = section;
        
        // Load section data
        switch(section) {
            case 'overview':
                this.loadOverview();
                break;
            case 'servers':
                this.loadServers();
                break;
            case 'tools':
                this.loadTools();
                break;
            case 'docs':
                this.loadDocs();
                break;
            case 'settings':
                this.loadSettings();
                break;
        }
    }
    
    async apiCall(url, options = {}) {
        try {
            const response = await fetch(url, {
                ...options,
                headers: {
                    ...this.defaultHeaders,
                    ...options.headers
                },
                credentials: 'same-origin' // Include cookies for session
            });
            
            if (response.status === 401) {
                // Redirect to login
                window.location.href = '/login';
                return;
            }
            
            if (!response.ok) {
                let errorMessage;
                const contentType = response.headers.get('content-type');
                
                if (contentType && contentType.includes('application/json')) {
                    try {
                        const error = await response.json();
                        errorMessage = error.error || error.message || `HTTP ${response.status}`;
                    } catch (e) {
                        errorMessage = `HTTP ${response.status}: ${response.statusText}`;
                    }
                } else {
                    // Try to get text error message
                    try {
                        errorMessage = await response.text();
                    } catch (e) {
                        errorMessage = `HTTP ${response.status}: ${response.statusText}`;
                    }
                }
                
                throw new Error(errorMessage);
            }
            
            return await response.json();
        } catch (error) {
            console.error('API Error:', error);
            this.showError(error.message);
            throw error;
        }
    }
    
    async loadOverview() {
        try {
            const data = await this.apiCall('/api/dashboard/overview');
            
            // Update stats
            document.getElementById('totalServers').textContent = data.total_servers;
            document.getElementById('healthyServers').textContent = data.healthy_servers;
            document.getElementById('totalTools').textContent = data.total_tools;
            document.getElementById('routerUptime').textContent = this.formatUptime(data.router_uptime);
            
            // Load server status
            await this.loadServerStatus();
        } catch (error) {
            console.error('Failed to load overview:', error);
        }
    }
    
    async loadServerStatus() {
        try {
            const servers = await this.apiCall('/api/dashboard/servers');
            
            const container = document.getElementById('serverStatusList');
            container.innerHTML = '<h3>Server Status</h3>';
            
            if (servers.length === 0) {
                container.innerHTML += '<p class="empty">No servers configured</p>';
                return;
            }
            
            servers.forEach(server => {
                const card = document.createElement('div');
                card.className = 'server-card';
                card.innerHTML = `
                    <div class="server-info">
                        <h4>${this.escapeHtml(server.name)}</h4>
                        <div class="server-meta">
                            <span>ID: ${this.escapeHtml(server.id)}</span>
                            <span>Tools: ${server.tool_count}</span>
                            <span>Sandbox: ${this.escapeHtml(server.sandbox_type)}</span>
                        </div>
                    </div>
                    <div class="server-status ${server.healthy ? 'healthy' : 'unhealthy'}">
                        ${server.healthy ? '✅ Healthy' : '❌ Unhealthy'}
                    </div>
                `;
                container.appendChild(card);
            });
        } catch (error) {
            console.error('Failed to load server status:', error);
        }
    }
    
    async loadServers() {
        try {
            const servers = await this.apiCall('/api/dashboard/servers');
            this.servers = servers;
            
            const container = document.getElementById('serverList');
            container.innerHTML = '';
            
            if (servers.length === 0) {
                container.innerHTML = '<p class="empty">No servers configured. Click "Add Server" to get started.</p>';
                return;
            }
            
            servers.forEach(server => {
                const card = document.createElement('div');
                card.className = 'server-card';
                card.innerHTML = `
                    <div class="server-info">
                        <h4>${this.escapeHtml(server.name)}</h4>
                        <div class="server-meta">
                            <span>ID: ${this.escapeHtml(server.id)}</span>
                            <span>Tools: ${server.tool_count}</span>
                            <span>Sandbox: ${this.escapeHtml(server.sandbox_type)}</span>
                        </div>
                    </div>
                    <div class="server-actions">
                        <button class="btn btn-sm btn-secondary" onclick="app.viewServerLogs('${server.id}')">
                            📋 Logs
                        </button>
                        <button class="btn btn-sm btn-primary" onclick="app.editServer('${server.id}')">
                            ✏️ Edit
                        </button>
                        <button class="btn btn-sm btn-danger" onclick="app.deleteServer('${server.id}')">
                            🗑️ Delete
                        </button>
                    </div>
                `;
                container.appendChild(card);
            });
        } catch (error) {
            console.error('Failed to load servers:', error);
        }
    }
    
    async loadTools() {
        // Load tools module if not already loaded
        if (!window.toolsModule) {
            window.toolsModule = new window.ToolsModule('/api/dashboard');
        }
        
        // Initialize the tools module in the tools section
        const toolsSection = document.getElementById('tools');
        if (toolsSection) {
            await window.toolsModule.init(toolsSection);
        }
    }
    
    // ORIGINAL loadTools - kept for reference
    /*
    async loadToolsOriginal() {
        try {
            const tools = await this.apiCall('/api/dashboard/tools');
            this.tools = tools;
            
            const container = document.getElementById('toolsList');
            container.innerHTML = '';
            
            if (tools.length === 0) {
                container.innerHTML = '<p class="empty">No tools available. Add and start some MCP servers first.</p>';
                return;
            }
            
            tools.forEach(tool => {
                const item = document.createElement('div');
                item.className = 'tool-item';
                item.innerHTML = `
                    <div class="tool-info">
                        <h4>${this.escapeHtml(tool.name)}</h4>
                        <p class="tool-description">${this.escapeHtml(tool.description)}</p>
                    </div>
                    <button class="btn btn-sm btn-primary" onclick="app.testTool('${tool.name}')">
                        🧪 Test
                    </button>
                `;
                container.appendChild(item);
            });
        } catch (error) {
            console.error('Failed to load tools:', error);
        }
    }
    */
    
    loadDocs() {
        // Setup docs navigation
        const docsLinks = document.querySelectorAll('.docs-link');
        docsLinks.forEach(link => {
            link.addEventListener('click', (e) => {
                e.preventDefault();
                const targetId = link.getAttribute('href').substring(1);
                this.showDocsSection(targetId);
            });
        });
        
        // Setup accordion functionality
        const accordionItems = document.querySelectorAll('.accordion-item h5');
        accordionItems.forEach(item => {
            item.addEventListener('click', () => {
                const parent = item.parentElement;
                parent.classList.toggle('active');
            });
        });
        
        // Show first docs section by default
        this.showDocsSection('docs-getting-started');
    }
    
    showDocsSection(sectionId) {
        // Update active states
        document.querySelectorAll('.docs-link').forEach(link => {
            link.classList.remove('active');
            if (link.getAttribute('href') === `#${sectionId}`) {
                link.classList.add('active');
            }
        });
        
        // Show/hide sections
        document.querySelectorAll('.docs-section').forEach(section => {
            section.classList.remove('active');
        });
        const targetSection = document.getElementById(sectionId);
        if (targetSection) {
            targetSection.classList.add('active');
        }
    }
    
    async loadSettings() {
        // Load all settings sections
        this.loadSessions();
        this.loadAuthSettings();
        this.loadUserProfile();
    }
    
    async loadSessions() {
        try {
            const sessions = await this.apiCall('/api/dashboard/sessions');
            
            const container = document.getElementById('sessionsList');
            if (sessions.length === 0) {
                container.innerHTML = '<p class="empty">No active sessions</p>';
                return;
            }
            
            container.innerHTML = sessions.map(session => `
                <div class="session-item">
                    <div class="session-info">
                        <h4>${this.escapeHtml(session.name || 'Unnamed Session')}</h4>
                        <div class="session-meta">
                            <span class="session-type ${session.type}">
                                ${session.type === 'browser' ? '🌐' : '💻'} ${session.type}
                            </span>
                            <span>Created: ${new Date(session.created_at).toLocaleDateString()}</span>
                            <span>Last used: ${this.formatTimeAgo(session.last_used)}</span>
                            ${session.expires_at ? `<span>Expires: ${new Date(session.expires_at).toLocaleDateString()}</span>` : ''}
                        </div>
                    </div>
                    <button class="btn btn-sm btn-danger" onclick="app.revokeSession('${session.id}')">
                        Revoke
                    </button>
                </div>
            `).join('');
        } catch (error) {
            console.error('Failed to load sessions:', error);
        }
    }
    
    async loadAuthSettings() {
        try {
            const settings = await this.apiCall('/api/dashboard/auth/settings');
            
            const container = document.getElementById('settingsForm');
            container.innerHTML = `
                <form onsubmit="app.saveSettings(event)">
                    <div class="form-group">
                        <label for="appName">Application Name</label>
                        <input type="text" id="appName" value="${this.escapeHtml(settings.app_name)}" readonly>
                    </div>
                    
                    <div class="form-group">
                        <label for="sessionDuration">Session Duration (minutes)</label>
                        <input type="number" id="sessionDuration" value="${settings.session_duration_minutes}" min="5" max="1440">
                    </div>
                    
                    <div class="form-group">
                        <label>
                            <input type="checkbox" id="enableMagicLinks" ${settings.enable_magic_links ? 'checked' : ''}>
                            Enable Magic Link Authentication
                        </label>
                        <small>Allow users to login via email links</small>
                    </div>
                    
                    <div class="form-group">
                        <label>
                            <input type="checkbox" id="requireDeviceTrust" ${settings.require_device_trust ? 'checked' : ''}>
                            Require device trust
                        </label>
                        <small>New devices must be approved before use</small>
                    </div>
                    
                    <div class="form-group">
                        <label for="maxDevices">Max devices per user</label>
                        <input type="number" id="maxDevices" value="${settings.max_devices_per_user}" min="1" max="10">
                    </div>
                    
                    <button type="submit" class="btn btn-primary">Save Settings</button>
                </form>
            `;
        } catch (error) {
            console.error('Failed to load settings:', error);
        }
    }
    
    async loadUserProfile() {
        try {
            const profile = await this.apiCall('/api/dashboard/user/profile');
            
            const container = document.getElementById('userProfile');
            container.innerHTML = `
                <div class="user-profile-card">
                    <div class="user-avatar">
                        ${profile.username.charAt(0).toUpperCase()}
                    </div>
                    <div class="user-details">
                        <h4>${this.escapeHtml(profile.username)}</h4>
                        <p>${this.escapeHtml(profile.email)}</p>
                        <p class="text-muted">Member since ${new Date(profile.created_at).toLocaleDateString()}</p>
                    </div>
                </div>
                
                <div class="user-actions" style="margin-top: var(--spacing-lg);">
                    <button class="btn btn-secondary" onclick="app.logout()">
                        🚪 Logout
                    </button>
                </div>
            `;
        } catch (error) {
            console.error('Failed to load user profile:', error);
        }
    }
    
    showAddServerModal() {
        document.getElementById('modalTitle').textContent = 'Add Server';
        document.getElementById('saveServerBtn').textContent = 'Add Server';
        document.getElementById('serverForm').reset();
        document.getElementById('serverId').disabled = false;
        document.getElementById('serverModal').classList.add('active');
        
        // Initialize JSON editor with default structure
        const defaultConfig = {
            id: "my-server",
            name: "My MCP Server",
            command: "npx",
            args: [],
            env: {},
            requires_auth: false,
            allowed_users: [],
            sandbox: {
                strategy: "none",
                env_passthrough: []
            }
        };
        
        document.getElementById('jsonEditor').value = JSON.stringify(defaultConfig, null, 2);
        this.setEditorView('split'); // Default to split view
    }
    
    async editServer(serverId) {
        // Use the serverModal object to properly handle edit mode
        if (window.serverModal) {
            window.serverModal.open(serverId);
        } else {
            console.error('Server modal not initialized');
        }
    }
    
    // DEPRECATED - Now handled by serverModal.save()
    /* async saveServer(event) {
        event.preventDefault();
        
        const form = event.target;
        const formData = new FormData(form);
        
        const serverId = formData.get('id');
        const isEdit = document.getElementById('serverId').disabled;
        
        // Build sandbox config based on strategy
        let sandboxStrategy;
        const strategyType = formData.get('sandboxStrategy');
        if (strategyType === 'none') {
            sandboxStrategy = 'none';
        } else {
            sandboxStrategy = {};
            sandboxStrategy[strategyType] = strategyType === 'docker' || strategyType === 'podman' 
                ? { image: 'node:18-alpine' } 
                : {};
        }
        
        const serverData = {
            id: serverId,
            name: formData.get('name'),
            command: formData.get('command'),
            args: formData.get('args').split(/\s+/).filter(arg => arg.length > 0),
            env: {},
            requires_auth: formData.get('requiresAuth') === 'on',
            allowed_users: [],
            sandbox: {
                strategy: sandboxStrategy,
                env_passthrough: ['HOME', 'USER', 'PATH']
            }
        };
        
        try {
            if (isEdit) {
                await this.apiCall(`/api/dashboard/servers/${serverId}`, {
                    method: 'PUT',
                    body: JSON.stringify(serverData)
                });
            } else {
                await this.apiCall('/api/dashboard/servers', {
                    method: 'POST',
                    body: JSON.stringify(serverData)
                });
            }
            
            this.closeModal();
            this.loadServers();
            this.showSuccess(isEdit ? 'Server updated successfully' : 'Server added successfully');
        } catch (error) {
            console.error('Failed to save server:', error);
        }
    } */
    
    async deleteServer(serverId) {
        if (!confirm(`Are you sure you want to delete server "${serverId}"?`)) {
            return;
        }
        
        const persistToConfig = confirm('Do you want to remove this server from the config file as well?');
        
        try {
            await this.apiCall(`/api/dashboard/servers/${serverId}`, {
                method: 'DELETE',
                body: JSON.stringify({ persist_to_config: persistToConfig })
            });
            
            this.loadServers();
            this.showSuccess('Server deleted successfully');
        } catch (error) {
            console.error('Failed to delete server:', error);
        }
    }
    
    async viewServerLogs(serverId) {
        try {
            const logs = await this.apiCall(`/api/dashboard/servers/${serverId}/logs`);
            
            alert('Server logs:\n\n' + logs.join('\n'));
        } catch (error) {
            console.error('Failed to load logs:', error);
        }
    }
    
    // MOVED TO tools.js
    /* async testTool(toolName) {
        const tool = this.tools.find(t => t.name === toolName);
        if (!tool) return;
        
        // Build form based on input schema
        let formHtml = `<h4>Tool: ${this.escapeHtml(toolName)}</h4>`;
        formHtml += `<p>${this.escapeHtml(tool.description)}</p>`;
        formHtml += '<form id="toolTestForm" onsubmit="app.runToolTest(event)">';
        formHtml += `<input type="hidden" name="tool_name" value="${toolName}">`;
        
        if (tool.inputSchema && tool.inputSchema.properties) {
            Object.entries(tool.inputSchema.properties).forEach(([key, schema]) => {
                formHtml += '<div class="form-group">';
                formHtml += `<label for="tool_${key}">${this.escapeHtml(key)}</label>`;
                
                if (schema.type === 'string') {
                    formHtml += `<input type="text" id="tool_${key}" name="${key}" ${tool.inputSchema.required?.includes(key) ? 'required' : ''}>`;
                } else if (schema.type === 'number') {
                    formHtml += `<input type="number" id="tool_${key}" name="${key}" ${tool.inputSchema.required?.includes(key) ? 'required' : ''}>`;
                } else if (schema.type === 'boolean') {
                    formHtml += `<input type="checkbox" id="tool_${key}" name="${key}">`;
                }
                
                if (schema.description) {
                    formHtml += `<small>${this.escapeHtml(schema.description)}</small>`;
                }
                formHtml += '</div>';
            });
        }
        
        formHtml += '<div class="modal-footer">';
        formHtml += '<button type="button" class="btn btn-secondary" onclick="app.closeToolModal()">Cancel</button>';
        formHtml += '<button type="submit" class="btn btn-primary">Run Test</button>';
        formHtml += '</div>';
        formHtml += '</form>';
        formHtml += '<div id="toolTestResult"></div>';
        
        document.getElementById('toolTestContent').innerHTML = formHtml;
        document.getElementById('toolModal').classList.add('active');
    } */
    
    // MOVED TO tools.js  
    /* async runToolTest(event) {
        event.preventDefault();
        
        const form = event.target;
        const formData = new FormData(form);
        
        const toolName = formData.get('tool_name');
        formData.delete('tool_name');
        
        // Build arguments object
        const args = {};
        for (const [key, value] of formData.entries()) {
            if (value === 'on') {
                args[key] = true;
            } else if (!isNaN(value) && value !== '') {
                args[key] = parseFloat(value);
            } else {
                args[key] = value;
            }
        }
        
        try {
            const result = await this.apiCall('/api/dashboard/tools/test', {
                method: 'POST',
                body: JSON.stringify({
                    tool_name: toolName,
                    arguments: args
                })
            });
            
            document.getElementById('toolTestResult').innerHTML = `
                <h4>Result:</h4>
                <pre>${JSON.stringify(result, null, 2)}</pre>
            `;
        } catch (error) {
            console.error('Tool test failed:', error);
        }
    } */
    
    async saveSettings(event) {
        event.preventDefault();
        
        const settings = {
            session_duration_minutes: parseInt(document.getElementById('sessionDuration').value),
            require_pin: document.getElementById('requirePin').checked,
            require_device_trust: document.getElementById('requireDeviceTrust').checked,
            max_devices_per_user: parseInt(document.getElementById('maxDevices').value)
        };
        
        try {
            await this.apiCall('/api/dashboard/auth/settings', {
                method: 'PUT',
                body: JSON.stringify(settings)
            });
            
            this.showSuccess('Settings saved successfully');
        } catch (error) {
            console.error('Failed to save settings:', error);
        }
    }
    
    closeModal() {
        document.getElementById('serverModal').classList.remove('active');
    }
    
    updateSandboxFields() {
        // This function updates sandbox-specific fields based on selected strategy
        const strategy = document.getElementById('sandboxStrategy').value;
        const options = document.querySelectorAll('.strategy-options');
        
        // Hide all options first
        options.forEach(opt => opt.style.display = 'none');
        
        // Show relevant options
        switch(strategy) {
            case 'docker':
            case 'podman':
                const dockerOpts = document.getElementById('dockerOptions');
                if (dockerOpts) dockerOpts.style.display = 'block';
                break;
            case 'firejail':
                const firejailOpts = document.getElementById('firejailOptions');
                if (firejailOpts) firejailOpts.style.display = 'block';
                break;
            case 'bubblewrap':
                const bubblewrapOpts = document.getElementById('bubblewrapOptions');
                if (bubblewrapOpts) bubblewrapOpts.style.display = 'block';
                break;
        }
    }
    
    // MOVED TO tools.js
    /* closeToolModal() {
        document.getElementById('toolModal').classList.remove('active');
    } */
    
    formatUptime(seconds) {
        if (seconds === 0) return 'Just started';
        
        const days = Math.floor(seconds / 86400);
        const hours = Math.floor((seconds % 86400) / 3600);
        const mins = Math.floor((seconds % 3600) / 60);
        
        const parts = [];
        if (days > 0) parts.push(`${days}d`);
        if (hours > 0) parts.push(`${hours}h`);
        if (mins > 0) parts.push(`${mins}m`);
        
        return parts.join(' ') || '< 1m';
    }
    
    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }
    
    showToast(message, type = 'info') {
        // Create toast container if it doesn't exist
        let toastContainer = document.getElementById('toastContainer');
        if (!toastContainer) {
            toastContainer = document.createElement('div');
            toastContainer.id = 'toastContainer';
            toastContainer.className = 'toast-container';
            document.body.appendChild(toastContainer);
        }
        
        // Create toast element
        const toast = document.createElement('div');
        toast.className = `toast toast-${type}`;
        
        // Add icon based on type
        const icons = {
            success: '✅',
            error: '❌',
            info: 'ℹ️',
            warning: '⚠️'
        };
        
        toast.innerHTML = `
            <span class="toast-icon">${icons[type] || icons.info}</span>
            <span class="toast-message">${this.escapeHtml(message)}</span>
            <button class="toast-close" onclick="this.parentElement.remove()">×</button>
        `;
        
        toastContainer.appendChild(toast);
        
        // Auto-remove after 5 seconds (10 seconds for errors)
        setTimeout(() => {
            toast.remove();
        }, type === 'error' ? 10000 : 5000);
    }
    
    showSuccess(message) {
        console.log('Success:', message);
        this.showToast(message, 'success');
    }
    
    showInfo(message) {
        console.log('Info:', message);
        this.showToast(message, 'info');
    }
    
    showError(message) {
        console.error('Error:', message);
        this.showToast(message, 'error');
    }
    
    startAutoRefresh() {
        // Refresh overview every 10 seconds when on that page
        setInterval(() => {
            if (this.currentSection === 'overview') {
                this.loadOverview();
            }
        }, 10000);
    }
    
    // JSON Editor Functions
    setEditorView(view, event) {
        const container = document.getElementById('editorContainer');
        const toggleButtons = document.querySelectorAll('.toggle-btn');
        
        // Update button states
        toggleButtons.forEach(btn => {
            btn.classList.remove('active');
        });
        
        // Add active class to the clicked button if event is provided
        if (event && event.target) {
            event.target.classList.add('active');
        } else {
            // If no event, find the button that matches the view
            const targetBtn = document.querySelector(`.toggle-btn[onclick*="${view}"]`);
            if (targetBtn) targetBtn.classList.add('active');
        }
        
        // Update container class
        container.className = 'editor-container';
        if (view === 'form') {
            container.classList.add('form-only');
        } else if (view === 'json') {
            container.classList.add('json-only');
        }
    }
    
    syncFormToJson() {
        try {
            const form = document.getElementById('serverForm');
            const formData = new FormData(form);
            
            // Parse arguments
            const argsStr = formData.get('args');
            const args = argsStr ? argsStr.split(/\s+/).filter(arg => arg.length > 0) : [];
            
            // Parse allowed users
            const allowedUsersStr = formData.get('allowedUsers');
            const allowedUsers = allowedUsersStr ? 
                allowedUsersStr.split(',').map(u => u.trim()).filter(u => u.length > 0) : [];
            
            // Build sandbox config
            let sandboxStrategy;
            let envPassthrough = ['HOME', 'USER', 'PATH'];
            
            // Try to preserve sandbox config from existing JSON
            try {
                const currentJson = document.getElementById('jsonEditor').value;
                if (currentJson) {
                    const currentConfig = JSON.parse(currentJson);
                    if (currentConfig.sandbox && currentConfig.sandbox.env_passthrough) {
                        envPassthrough = currentConfig.sandbox.env_passthrough;
                    }
                }
            } catch (e) {
                // Ignore parse errors
            }
            
            const strategyType = formData.get('sandboxStrategy');
            if (strategyType === 'none') {
                sandboxStrategy = 'none';
            } else {
                sandboxStrategy = {};
                sandboxStrategy[strategyType] = strategyType === 'docker' || strategyType === 'podman' 
                    ? { image: 'node:18-alpine' } 
                    : {};
            }
            
            // Try to preserve env from existing JSON if available
            let env = {};
            try {
                const currentJson = document.getElementById('jsonEditor').value;
                if (currentJson) {
                    const currentConfig = JSON.parse(currentJson);
                    env = currentConfig.env || {};
                }
            } catch (e) {
                // Ignore parse errors
            }
            
            const config = {
                id: formData.get('id') || '',
                name: formData.get('name') || '',
                command: formData.get('command') || '',
                args: args,
                env: env,
                requires_auth: formData.get('requiresAuth') === 'on',
                allowed_users: allowedUsers,
                sandbox: {
                    strategy: sandboxStrategy,
                    env_passthrough: envPassthrough
                }
            };
            
            // Update JSON editor
            document.getElementById('jsonEditor').value = JSON.stringify(config, null, 2);
            document.getElementById('jsonError').classList.remove('active');
            document.getElementById('validationStatus').textContent = '✓ Valid configuration';
            document.getElementById('validationStatus').className = 'validation-status valid';
            
            // Show/hide allowed users field
            document.getElementById('allowedUsersGroup').style.display = 
                config.requires_auth ? 'block' : 'none';
        } catch (error) {
            console.error('Error syncing form to JSON:', error);
        }
    }
    
    syncJsonToForm() {
        try {
            const jsonStr = document.getElementById('jsonEditor').value;
            const config = JSON.parse(jsonStr);
            
            // Update form fields
            document.getElementById('serverId').value = config.id || '';
            document.getElementById('serverName').value = config.name || '';
            document.getElementById('serverCommand').value = config.command || '';
            document.getElementById('serverArgs').value = (config.args || []).join(' ');
            document.getElementById('requiresAuth').checked = config.requires_auth || false;
            document.getElementById('allowedUsers').value = (config.allowed_users || []).join(', ');
            
            // Set sandbox strategy
            if (config.sandbox && config.sandbox.strategy) {
                const strategy = config.sandbox.strategy;
                if (typeof strategy === 'string') {
                    document.getElementById('sandboxStrategy').value = strategy;
                } else if (strategy.docker) {
                    document.getElementById('sandboxStrategy').value = 'docker';
                } else if (strategy.podman) {
                    document.getElementById('sandboxStrategy').value = 'podman';
                } else if (strategy.firejail) {
                    document.getElementById('sandboxStrategy').value = 'firejail';
                } else if (strategy.bubblewrap) {
                    document.getElementById('sandboxStrategy').value = 'bubblewrap';
                }
            }
            
            // Clear error and update status
            document.getElementById('jsonError').classList.remove('active');
            document.getElementById('validationStatus').textContent = '✓ Valid JSON';
            document.getElementById('validationStatus').className = 'validation-status valid';
            
            // Show/hide allowed users field
            document.getElementById('allowedUsersGroup').style.display = 
                config.requires_auth ? 'block' : 'none';
            
        } catch (error) {
            // Show error
            document.getElementById('jsonError').textContent = 'Invalid JSON: ' + error.message;
            document.getElementById('jsonError').classList.add('active');
            document.getElementById('validationStatus').textContent = '✗ Invalid JSON';
            document.getElementById('validationStatus').className = 'validation-status invalid';
        }
    }
    
    formatJson() {
        try {
            const jsonStr = document.getElementById('jsonEditor').value;
            const config = JSON.parse(jsonStr);
            document.getElementById('jsonEditor').value = JSON.stringify(config, null, 2);
            document.getElementById('jsonError').classList.remove('active');
        } catch (error) {
            document.getElementById('jsonError').textContent = 'Cannot format: ' + error.message;
            document.getElementById('jsonError').classList.add('active');
        }
    }
    
    // DEPRECATED - Now handled by serverModal.save()
    /* // Override saveServer to support JSON input
    async saveServer(event) {
        event.preventDefault();
        
        try {
            // Get config from JSON editor if it's visible
            const jsonPane = document.getElementById('jsonPane');
            const isJsonVisible = !jsonPane.closest('.editor-container').classList.contains('form-only');
            
            let serverData;
            
            if (isJsonVisible) {
                // Parse from JSON editor
                const jsonStr = document.getElementById('jsonEditor').value;
                serverData = JSON.parse(jsonStr);
            } else {
                // Build from form
                const form = document.getElementById('serverForm');
                const formData = new FormData(form);
                
                const argsStr = formData.get('args');
                const args = argsStr ? argsStr.split(/\s+/).filter(arg => arg.length > 0) : [];
                
                const allowedUsersStr = formData.get('allowedUsers');
                const allowedUsers = allowedUsersStr ? 
                    allowedUsersStr.split(',').map(u => u.trim()).filter(u => u.length > 0) : [];
                
                let sandboxStrategy;
                const strategyType = formData.get('sandboxStrategy');
                if (strategyType === 'none') {
                    sandboxStrategy = 'none';
                } else {
                    sandboxStrategy = {};
                    sandboxStrategy[strategyType] = strategyType === 'docker' || strategyType === 'podman' 
                        ? { image: 'node:18-alpine' } 
                        : {};
                }
                
                serverData = {
                    id: formData.get('id'),
                    name: formData.get('name'),
                    command: formData.get('command'),
                    args: args,
                    env: {},
                    requires_auth: formData.get('requiresAuth') === 'on',
                    allowed_users: allowedUsers,
                    sandbox: {
                        strategy: sandboxStrategy,
                        env_passthrough: ['HOME', 'USER', 'PATH']
                    }
                };
            }
            
            const isEdit = document.getElementById('serverId').disabled;
            const serverId = serverData.id;
            
            if (isEdit) {
                await this.apiCall(`/api/dashboard/servers/${serverId}`, {
                    method: 'PUT',
                    body: JSON.stringify(serverData)
                });
            } else {
                await this.apiCall('/api/dashboard/servers', {
                    method: 'POST',
                    body: JSON.stringify(serverData)
                });
            }
            
            this.closeModal();
            this.loadServers();
            this.showSuccess(isEdit ? 'Server updated successfully' : 'Server added successfully');
        } catch (error) {
            console.error('Failed to save server:', error);
            this.showError('Failed to save server: ' + error.message);
        }
    } */
    
    // Theme Management
    initTheme() {
        // Check for saved theme preference or default to light
        const savedTheme = localStorage.getItem('theme') || 'light';
        document.documentElement.setAttribute('data-theme', savedTheme);
    }
    
    toggleTheme() {
        const currentTheme = document.documentElement.getAttribute('data-theme');
        const newTheme = currentTheme === 'dark' ? 'light' : 'dark';
        
        document.documentElement.setAttribute('data-theme', newTheme);
        localStorage.setItem('theme', newTheme);
    }
    
    // Session Management
    async generateAPIToken() {
        document.getElementById('apiTokenModal').classList.add('active');
        document.getElementById('tokenForm').reset();
        document.getElementById('tokenDisplay').style.display = 'none';
    }
    
    async createAPIToken(event) {
        event.preventDefault();
        
        const form = event.target;
        const formData = new FormData(form);
        
        // Get selected scopes
        const scopes = [];
        document.querySelectorAll('input[name="scopes"]:checked').forEach(checkbox => {
            scopes.push(checkbox.value);
        });
        
        try {
            const response = await this.apiCall('/api/dashboard/tokens', {
                method: 'POST',
                body: JSON.stringify({
                    name: formData.get('name'),
                    expiry_days: parseInt(formData.get('expiry')),
                    scopes: scopes
                })
            });
            
            // Show the generated token
            document.getElementById('generatedToken').textContent = response.token;
            document.getElementById('tokenForm').style.display = 'none';
            document.getElementById('tokenDisplay').style.display = 'block';
            
            // Refresh sessions list
            this.loadSessions();
        } catch (error) {
            console.error('Failed to generate token:', error);
        }
    }
    
    copyToken() {
        const token = document.getElementById('generatedToken').textContent;
        navigator.clipboard.writeText(token).then(() => {
            this.showSuccess('Token copied to clipboard!');
        }).catch(() => {
            // Fallback for older browsers
            const textarea = document.createElement('textarea');
            textarea.value = token;
            document.body.appendChild(textarea);
            textarea.select();
            document.execCommand('copy');
            document.body.removeChild(textarea);
            this.showSuccess('Token copied to clipboard!');
        });
    }
    
    closeTokenModal() {
        document.getElementById('apiTokenModal').classList.remove('active');
        document.getElementById('tokenForm').style.display = 'block';
        document.getElementById('tokenDisplay').style.display = 'none';
    }
    
    async revokeSession(sessionId) {
        if (!confirm('Are you sure you want to revoke this session?')) {
            return;
        }
        
        try {
            await this.apiCall(`/api/dashboard/sessions/${sessionId}`, {
                method: 'DELETE'
            });
            
            this.loadSessions();
            this.showSuccess('Session revoked successfully');
        } catch (error) {
            console.error('Failed to revoke session:', error);
        }
    }
    
    async revokeAllSessions() {
        if (!confirm('Are you sure you want to revoke ALL sessions? You will need to login again.')) {
            return;
        }
        
        try {
            await this.apiCall('/api/dashboard/sessions', {
                method: 'DELETE'
            });
            
            // Clear local session and redirect to login
            localStorage.removeItem('jau-auth-token');
            window.location.href = '/login.html';
        } catch (error) {
            console.error('Failed to revoke sessions:', error);
        }
    }
    
    async logout() {
        try {
            await this.apiCall('/api/auth/logout', {
                method: 'POST'
            });
        } catch (error) {
            // Continue with logout even if API call fails
        }
        
        localStorage.removeItem('jau-auth-token');
        window.location.href = '/login.html';
    }
    
    formatTimeAgo(date) {
        const seconds = Math.floor((new Date() - new Date(date)) / 1000);
        
        const intervals = {
            year: 31536000,
            month: 2592000,
            week: 604800,
            day: 86400,
            hour: 3600,
            minute: 60
        };
        
        for (const [unit, secondsInUnit] of Object.entries(intervals)) {
            const interval = Math.floor(seconds / secondsInUnit);
            if (interval >= 1) {
                return `${interval} ${unit}${interval === 1 ? '' : 's'} ago`;
            }
        }
        
        return 'Just now';
    }
}

// Initialize app when DOM is ready
const app = new JauAuthDashboard();
window.app = app; // Make app globally accessible for debugging