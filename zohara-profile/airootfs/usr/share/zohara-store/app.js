// Zohara Store Frontend Logic

let catalogData = {};
let currentApp = null;

let isPyWebViewReady = false;
let _originalCatalog = {};

// Initialize when DOM is ready and pywebview is available
window.addEventListener('pywebviewready', function() {
    isPyWebViewReady = true;
    console.log("PyWebView is ready. Fetching catalog...");
    // Retry a few times in case the API isn't fully ready yet
    let attempts = 0;
    const tryFetch = () => {
        attempts++;
        fetchCatalog().catch(err => {
            console.warn(`Catalog fetch attempt ${attempts} failed: ${err}`);
            if (attempts < 5) setTimeout(tryFetch, 1000);
            else {
                console.error('All catalog fetch attempts failed.');
                const el = document.getElementById('loading-spinner');
                if (el) el.innerHTML = `<p style="color:var(--accent-red)">Could not load catalog.<br>Check /tmp/zohara-store.log</p>`;
            }
        });
    };
    tryFetch();
});

// Fallback for browser testing without PyWebView
document.addEventListener('DOMContentLoaded', () => {
    setupNavigation();
    setupModal();
    setupSearch();
    
    // Wait briefly for pywebview to inject before falling back
    setTimeout(() => {
        if (!window.pywebview && !isPyWebViewReady) {
            console.warn("PyWebView not detected. Using dummy data for testing.");
            document.getElementById('loading-spinner').classList.add('hidden');
            renderDummyData();
        }
    }, 1500);
});

let searchDebounce = null;
function setupSearch() {
    const input = document.getElementById('search-input');
    if (!input) return;
    input.addEventListener('input', () => {
        clearTimeout(searchDebounce);
        const q = input.value.trim();
        if (q.length === 0) {
            // Restore full catalog when search is cleared
            catalogData = _originalCatalog;
            renderCatalog();
            return;
        }
        if (q.length < 2) return; // Wait for at least 2 chars
        searchDebounce = setTimeout(async () => {
            showLoadingSpinner(true);
            if (window.pywebview) {
                try {
                    const raw = await window.pywebview.api.search_packages(q);
                    const results = JSON.parse(raw);
                    // If search returns nothing, filter from loaded catalog instead
                    if (results.apps && results.apps.length > 0) {
                        catalogData = results;
                    } else {
                        const lower = q.toLowerCase();
                        catalogData = {
                            apps: (_originalCatalog.apps || []).filter(a =>
                                a.name.toLowerCase().includes(lower) ||
                                (a.description || '').toLowerCase().includes(lower)
                            )
                        };
                    }
                    renderCatalog();
                } catch(err) {
                    console.error('Search error:', err);
                    // Fallback: filter in-memory catalog
                    const lower = q.toLowerCase();
                    catalogData = {
                        apps: (_originalCatalog.apps || []).filter(a =>
                            a.name.toLowerCase().includes(lower) ||
                            (a.description || '').toLowerCase().includes(lower)
                        )
                    };
                    renderCatalog();
                } finally {
                    showLoadingSpinner(false);
                }
            } else {
                // Dev fallback: filter dummy data
                const lower = q.toLowerCase();
                catalogData = {
                    apps: (_originalCatalog.apps || []).filter(a =>
                        a.name.toLowerCase().includes(lower) ||
                        (a.description || '').toLowerCase().includes(lower)
                    )
                };
                renderCatalog();
                showLoadingSpinner(false);
            }
        }, 400);
    });
}

function showLoadingSpinner(show) {
    const el = document.getElementById('loading-spinner');
    if (el) el.classList.toggle('hidden', !show);
}

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
            const target = document.getElementById('page-' + targetPage);
            if (target) target.classList.add('active');
            
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
    const data = await window.pywebview.api.get_catalog();
    if (!data) throw new Error('Empty response from get_catalog');
    catalogData = JSON.parse(data);
    _originalCatalog = catalogData;
    if (!catalogData.apps || catalogData.apps.length === 0) throw new Error('Catalog returned 0 apps');
    console.log(`Catalog loaded: ${catalogData.apps.length} apps`);
    renderCatalog();
    document.getElementById('loading-spinner').classList.add('hidden');
}

function renderCatalog() {
    const appsGrid = document.getElementById('apps-grid');
    const gamesGrid = document.getElementById('games-grid');
    const appsOnlyGrid = document.getElementById('apps-only-grid');
    const gamesOnlyGrid = document.getElementById('games-only-grid');
    
    if (appsGrid) appsGrid.innerHTML = '';
    if (gamesGrid) gamesGrid.innerHTML = '';
    if (appsOnlyGrid) appsOnlyGrid.innerHTML = '';
    if (gamesOnlyGrid) gamesOnlyGrid.innerHTML = '';
    
    if (!catalogData || !catalogData.apps) return;
    
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
        
        // Clone for the dedicated tabs
        const cardClone = card.cloneNode(true);
        cardClone.addEventListener('click', () => openAppDetails(app));
        
        if (app.category && app.category.toLowerCase() === 'game') {
            if (gamesGrid) gamesGrid.appendChild(card);
            if (gamesOnlyGrid) gamesOnlyGrid.appendChild(cardClone);
        } else {
            if (appsGrid) appsGrid.appendChild(card);
            if (appsOnlyGrid) appsOnlyGrid.appendChild(cardClone);
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


// Dummy data for testing UI without python (no Windows apps)
function renderDummyData() {
    catalogData = {
        "apps": [
            {
                "id": "firefox",
                "name": "Mozilla Firefox",
                "publisher": "Mozilla",
                "description": "A fast, privacy-focused web browser built for speed, privacy, and security.",
                "category": "App",
                "type": "pacman",
                "package": "firefox",
                "icon_url": "https://dl.flathub.org/repo/appstream/x86_64/icons/128x128/org.mozilla.firefox.png"
            },
            {
                "id": "vscodium",
                "name": "VSCodium",
                "publisher": "VSCodium Contributors",
                "description": "Free/Libre open source software binaries of VS Code.",
                "category": "App",
                "type": "flatpak",
                "package": "com.vscodium.codium",
                "icon_url": "https://dl.flathub.org/repo/appstream/x86_64/icons/128x128/com.vscodium.codium.png"
            },
            {
                "id": "spotify",
                "name": "Spotify",
                "publisher": "Spotify Ltd.",
                "description": "Music and podcast streaming platform.",
                "category": "App",
                "type": "flatpak",
                "package": "com.spotify.Client",
                "icon_url": "https://dl.flathub.org/repo/appstream/x86_64/icons/128x128/com.spotify.Client.png"
            },
            {
                "id": "discord",
                "name": "Discord",
                "publisher": "Discord Inc.",
                "description": "Chat for communities and friends.",
                "category": "App",
                "type": "flatpak",
                "package": "com.discordapp.Discord",
                "icon_url": "https://dl.flathub.org/repo/appstream/x86_64/icons/128x128/com.discordapp.Discord.png"
            },
            {
                "id": "vlc",
                "name": "VLC Media Player",
                "publisher": "VideoLAN",
                "description": "A free and open source cross-platform multimedia player.",
                "category": "App",
                "type": "pacman",
                "package": "vlc",
                "icon_url": "https://dl.flathub.org/repo/appstream/x86_64/icons/128x128/org.videolan.VLC.png"
            },
            {
                "id": "lutris",
                "name": "Lutris",
                "publisher": "Lutris Team",
                "description": "Open Source gaming platform for Linux — run any game from any era.",
                "category": "Game",
                "type": "pacman",
                "package": "lutris",
                "icon_url": "https://dl.flathub.org/repo/appstream/x86_64/icons/128x128/net.lutris.Lutris.png"
            },
            {
                "id": "supertuxkart",
                "name": "SuperTuxKart",
                "publisher": "SuperTuxKart Team",
                "description": "A fun 3D open-source arcade racing game.",
                "category": "Game",
                "type": "pacman",
                "package": "supertuxkart",
                "icon_url": "https://dl.flathub.org/repo/appstream/x86_64/icons/128x128/net.supertuxkart.SuperTuxKart.png"
            },
            {
                "id": "steam",
                "name": "Steam",
                "publisher": "Valve Corporation",
                "description": "The ultimate gaming platform. Access thousands of games.",
                "category": "Game",
                "type": "flatpak",
                "package": "com.valvesoftware.Steam",
                "icon_url": "https://dl.flathub.org/repo/appstream/x86_64/icons/128x128/com.valvesoftware.Steam.png"
            }
        ]
    };
    renderCatalog();
}
