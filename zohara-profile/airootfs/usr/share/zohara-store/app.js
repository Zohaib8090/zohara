// Zohara Store Frontend Logic

let catalogData = {};
let currentApp = null;

// Initialize when DOM is ready and pywebview is available
window.addEventListener('pywebviewready', function() {
    console.log("PyWebView is ready. Fetching catalog...");
    fetchCatalog();
});

// Fallback for browser testing without PyWebView
document.addEventListener('DOMContentLoaded', () => {
    if (!window.pywebview) {
        console.warn("PyWebView not detected. Using dummy data for testing.");
        document.getElementById('loading-spinner').classList.add('hidden');
        renderDummyData();
    }
    
    setupNavigation();
    setupModal();
});

function setupNavigation() {
    const links = document.querySelectorAll('.nav-links li');
    links.forEach(link => {
        link.addEventListener('click', (e) => {
            // Update active state
            links.forEach(l => l.classList.remove('active'));
            link.classList.add('active');
            
            // Switch page
            const targetPage = link.getAttribute('data-page');
            document.querySelectorAll('.page-section').forEach(p => p.classList.remove('active'));
            document.getElementById('page-' + targetPage).classList.add('active');
            
            if (targetPage === 'library') {
                refreshLibrary();
            }
        });
    });
}

function setupModal() {
    const modal = document.getElementById('app-details-modal');
    document.getElementById('btn-close-modal').addEventListener('click', () => {
        modal.classList.add('hidden');
    });
    
    document.getElementById('detail-install-btn').addEventListener('click', async () => {
        if (!currentApp) return;
        
        const sourceSelect = document.getElementById('detail-source-select');
        const selectedSource = sourceSelect.value;
        const btn = document.getElementById('detail-install-btn');
        const consoleBox = document.getElementById('detail-console');
        const consoleOut = document.getElementById('detail-console-output');
        
        btn.disabled = true;
        btn.textContent = "Installing...";
        consoleBox.classList.remove('hidden');
        consoleOut.textContent = "Requesting installation via " + selectedSource + "...\n";
        
        if (window.pywebview) {
            try {
                // Call Python backend
                await window.pywebview.api.install_app(currentApp.id, selectedSource);
            } catch (err) {
                consoleOut.textContent += "\nError: " + err;
                btn.disabled = false;
                btn.textContent = "Install";
            }
        }
    });
}

// Function called by Python backend to append console logs
function appendLog(msg) {
    const consoleOut = document.getElementById('detail-console-output');
    if (consoleOut) {
        consoleOut.textContent += msg + "\n";
        consoleOut.scrollTop = consoleOut.scrollHeight;
    }
}

// Function called by Python backend when installation completes
function installationComplete(success) {
    const btn = document.getElementById('detail-install-btn');
    if (success) {
        btn.textContent = "Installed";
        btn.disabled = true;
    } else {
        btn.textContent = "Retry Install";
        btn.disabled = false;
    }
}

async function fetchCatalog() {
    try {
        const data = await window.pywebview.api.get_catalog();
        catalogData = JSON.parse(data);
        renderCatalog();
        document.getElementById('loading-spinner').classList.add('hidden');
    } catch (err) {
        console.error("Failed to fetch catalog:", err);
        document.getElementById('loading-spinner').innerHTML = `<p style="color:var(--accent-red)">Error loading catalog:<br>${err}</p>`;
    }
}

function renderCatalog() {
    const appsGrid = document.getElementById('apps-grid');
    const gamesGrid = document.getElementById('games-grid');
    
    if (appsGrid) appsGrid.innerHTML = '';
    if (gamesGrid) gamesGrid.innerHTML = '';
    
    catalogData.apps.forEach(app => {
        const card = document.createElement('div');
        card.className = 'app-card';
        
        // Use default icon if none provided
        const iconSrc = app.icon_url || 'https://upload.wikimedia.org/wikipedia/commons/3/35/Tux.svg';
        
        card.innerHTML = `
            <img src="${iconSrc}" alt="${app.name} icon">
            <h3>${app.name}</h3>
            <p>${app.publisher}</p>
        `;
        
        card.addEventListener('click', () => openAppDetails(app));
        
        if (app.category && app.category.toLowerCase() === 'game') {
            if (gamesGrid) gamesGrid.appendChild(card);
        } else {
            if (appsGrid) appsGrid.appendChild(card);
        }
    });
}

function openAppDetails(app) {
    currentApp = app;
    document.getElementById('detail-icon').src = app.icon_url || 'https://upload.wikimedia.org/wikipedia/commons/3/35/Tux.svg';
    document.getElementById('detail-title').textContent = app.name;
    document.getElementById('detail-publisher').textContent = app.publisher;
    document.getElementById('detail-description').textContent = app.description || 'No description available.';
    
    // Setup sources dropdown (deduping)
    const sourceSelect = document.getElementById('detail-source-select');
    sourceSelect.innerHTML = '';
    
    // If the catalog groups sources for an app:
    const sources = app.sources || [{type: app.type, package: app.package}];
    
    sources.forEach(src => {
        const opt = document.createElement('option');
        opt.value = src.type; // e.g., 'flatpak', 'pacman', 'windows'
        opt.textContent = src.type.charAt(0).toUpperCase() + src.type.slice(1); // Capitalize
        sourceSelect.appendChild(opt);
    });
    
    // Reset state
    document.getElementById('detail-console').classList.add('hidden');
    document.getElementById('detail-console-output').textContent = '';
    const btn = document.getElementById('detail-install-btn');
    btn.textContent = "Install";
    btn.disabled = false;
    
    // Check if installed
    if (window.pywebview) {
        window.pywebview.api.is_installed(app.id).then(installed => {
            if (installed) {
                btn.textContent = "Installed";
                btn.disabled = true;
            }
        });
    }
    
    document.getElementById('app-details-modal').classList.remove('hidden');
}

async function refreshLibrary() {
    const tbody = document.getElementById('library-tbody');
    tbody.innerHTML = '<tr><td colspan="4" style="text-align:center">Loading installed apps...</td></tr>';
    
    if (window.pywebview) {
        try {
            const installedStr = await window.pywebview.api.get_library();
            const installed = JSON.parse(installedStr);
            
            tbody.innerHTML = '';
            if (installed.length === 0) {
                tbody.innerHTML = '<tr><td colspan="4" style="text-align:center">No apps installed via Zohara Store.</td></tr>';
                return;
            }
            
            installed.forEach(app => {
                const tr = document.createElement('tr');
                tr.innerHTML = `
                    <td><strong>${app.name}</strong></td>
                    <td><span style="background: rgba(255,255,255,0.1); padding: 4px 8px; border-radius: 4px; font-size: 0.8rem;">${app.source}</span></td>
                    <td>${app.version || 'Latest'}</td>
                    <td><button class="secondary-btn" style="padding: 6px 12px; font-size: 0.85rem;" onclick="removeApp('${app.id}')">Remove</button></td>
                `;
                tbody.appendChild(tr);
            });
            
        } catch (err) {
            tbody.innerHTML = `<tr><td colspan="4" style="color:var(--accent-red)">Error: ${err}</td></tr>`;
        }
    }
}

// Global function for the remove button in library
async function removeApp(app_id) {
    if (window.pywebview) {
        // Find app in catalog to get its type
        const app = catalogData.apps.find(a => a.id === app_id);
        if (app) {
            if(confirm(`Are you sure you want to remove ${app.name}?`)) {
                await window.pywebview.api.remove_app(app_id, app.type);
                refreshLibrary();
            }
        }
    }
}


// Dummy data for testing UI without python
function renderDummyData() {
    catalogData = {
        "featured": ["firefox", "vscodium"],
        "apps": [
            {
                "id": "firefox",
                "name": "Mozilla Firefox",
                "publisher": "Mozilla",
                "description": "Fast and private browser.",
                "type": "pacman",
                "icon_url": "https://upload.wikimedia.org/wikipedia/commons/a/a0/Firefox_logo%2C_2019.svg"
            },
            {
                "id": "vscodium",
                "name": "VSCodium",
                "publisher": "VSCodium Contributors",
                "description": "Free open source VS Code.",
                "type": "flatpak",
                "icon_url": "https://upload.wikimedia.org/wikipedia/commons/6/60/VSCodium_logo.svg"
            },
            {
                "id": "photoshop-wine",
                "name": "Adobe Photoshop CS6",
                "publisher": "Adobe Systems",
                "description": "Windows binary running seamlessly via Zohara Proton Layer.",
                "type": "windows",
                "icon_url": "https://upload.wikimedia.org/wikipedia/commons/2/20/Photoshop_CC_icon.png"
            }
        ]
    };
    renderCatalog();
}
