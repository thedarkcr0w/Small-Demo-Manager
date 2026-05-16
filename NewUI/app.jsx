// Main app: window chrome + sidebar + library + details

const TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
  "accent": "#f59e0b",
  "sidebarStyle": "full",
  "layout": "hybrid"
}/*EDITMODE-END*/;

// ─────────────────────────────────────────────────────────────────────────────
// Window chrome (Windows-style)
// ─────────────────────────────────────────────────────────────────────────────
function WindowChrome({ children }) {
  return (
    <div className="win-chrome">
      <div className="win-title">
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, paddingLeft: 12 }}>
          <img src="icon.ico" alt="" width={16} height={16}
               style={{ borderRadius: 3, imageRendering: 'auto' }}/>
          <span style={{ fontSize: 12, color: 'var(--sec)', fontWeight: 500 }}>SmallDemoManager</span>
          <span style={{ fontSize: 11, color: 'var(--mut)', marginLeft: 4 }}>
            {window.SDM_VERSION ? `· v${window.SDM_VERSION}` : ''}
          </span>
        </div>
        <div className="win-btns">
          <button className="win-btn" aria-label="Minimize"
                  onClick={() => window.SDM?.call('windowMinimize').catch(() => {})}>
            <IconMinus size={12}/>
          </button>
          <button className="win-btn win-btn-close" aria-label="Close"
                  onClick={() => window.SDM?.call('windowClose').catch(() => {})}>
            <IconClose size={13}/>
          </button>
        </div>
      </div>
      <div className="win-body">{children}</div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Sidebar About card + About modal
// ─────────────────────────────────────────────────────────────────────────────
function SidebarAbout({ onOpen }) {
  return (
    <button className="about-card" onClick={onOpen}>
      <div className="about-card-mark" style={{ background: 'transparent', padding: 0 }}>
        <img src="icon.ico" alt="" width={22} height={22} style={{ display: 'block', borderRadius: 5 }}/>
      </div>
      <div className="about-card-text">
        <div className="about-card-title">About the project</div>
      </div>
      <IconChevron size={12} style={{ color: 'var(--mut)', flexShrink: 0 }}/>
    </button>
  );
}

// Open external links via the C# bridge so they launch in the OS default browser
// rather than spawning a new WebView2 window.
function openExternal(e) {
  const href = e.currentTarget.getAttribute('href');
  e.preventDefault();
  if (href) window.SDM?.call('openExternal', { url: href }).catch(() => {});
}

function AboutModal({ onClose }) {
  React.useEffect(() => {
    const onKey = (e) => { if (e.key === 'Escape') onClose(); };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);
  return (
    <div className="modal-scrim" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <div className="about-card-mark" style={{ width: 28, height: 28, background: 'transparent', padding: 0 }}>
              <img src="icon.ico" alt="" width={28} height={28} style={{ display: 'block', borderRadius: 6 }}/>
            </div>
            <div>
              <div style={{ fontSize: 13, fontWeight: 700, color: 'var(--fg)' }}>About SmallDemoManager</div>
              <div style={{ fontSize: 10.5, color: 'var(--mut)', fontFamily: 'JetBrains Mono, monospace', marginTop: 1 }}>
                {window.SDM_VERSION ? `v${window.SDM_VERSION} · open source` : 'open source'}
              </div>
            </div>
          </div>
          <button className="ic-btn" onClick={onClose} aria-label="Close"><IconClose size={13}/></button>
        </div>
        <div className="modal-body">
          <div className="about-block">
            <p>
              Small Demo Manager was created by <span className="about-mention">Pythaeus</span> as an open-source tool for the CS2 community.
              Pythaeus is currently away, so <span className="about-mention">darkcr0w</span> will continue maintaining and updating the project in his absence.
            </p>
            <p>
              The goal is to carry his work forward, respect what he built, and keep his legacy alive.
              We wish Pythaeus all the best &mdash; he is a true legend.
            </p>
          </div>
          <div className="about-block">
            <p>
              Special thanks go, as always, to <span className="about-mention">KEROVSKI</span>, who inspired me to create and improve this software.
            </p>
            <p>
              I would also like to thank everyone who puts effort into raising awareness about the cheating problem in CS2, including
              {' '}<span className="about-mention">@KEROVSKI_</span>, <span className="about-mention">@HaiX</span>, <span className="about-mention">@LobaCS2</span>, <span className="about-mention">@neokCS</span>, and many others.
            </p>
          </div>
          <div className="about-links">
            <div className="about-links-head">Links</div>
            <div className="about-links-grid">
              <a className="about-link" href="https://github.com/thedarkcr0w/Small-Demo-Manager" onClick={openExternal}>
                <div className="about-link-icon"><IconGithub size={16}/></div>
                <div className="about-link-text">
                  <div className="about-link-title">GitHub repository</div>
                  <div className="about-link-sub">thedarkcr0w/Small-Demo-Manager</div>
                </div>
                <IconExternal size={12} style={{ color: 'var(--mut)', flexShrink: 0 }}/>
              </a>
              <div className="about-links-row">
                <a className="about-link about-link-kofi" href="https://ko-fi.com/darkcr0w" onClick={openExternal}>
                  <div className="about-link-icon about-link-icon-kofi"><IconCoffee size={16}/></div>
                  <div className="about-link-text">
                    <div className="about-link-title">Support darkcr0w</div>
                    <div className="about-link-sub">ko-fi.com/darkcr0w</div>
                  </div>
                </a>
                <a className="about-link about-link-kofi" href="https://ko-fi.com/pythaeus" onClick={openExternal}>
                  <div className="about-link-icon about-link-icon-kofi"><IconCoffee size={16}/></div>
                  <div className="about-link-text">
                    <div className="about-link-title">Support Pythaeus</div>
                    <div className="about-link-sub">ko-fi.com/pythaeus</div>
                  </div>
                </a>
              </div>
            </div>
          </div>
        </div>
        <div className="modal-foot">
          <button className="btn" onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Sidebar Settings card + Settings modal
// ─────────────────────────────────────────────────────────────────────────────
function SidebarSettings({ onOpen, cs2Path }) {
  return (
    <button className="about-card settings-card" onClick={onOpen}>
      <div className="about-card-mark settings-card-mark"><IconSettings size={13}/></div>
      <div className="about-card-text">
        <div className="about-card-title">Settings</div>
        <div className="about-card-sub" title={cs2Path || 'CS2 install location not set'}>
          {cs2Path || 'CS2 path not set'}
        </div>
      </div>
      <IconChevron size={12} style={{ color: 'var(--mut)', flexShrink: 0 }}/>
    </button>
  );
}

function SettingsModal({ onClose, initial, onSave }) {
  const [path, setPath] = React.useState(initial.cs2Path || '');
  const [moveOnImport, setMoveOnImport] = React.useState(initial.moveOnImport !== false);
  const [autoBackup, setAutoBackup] = React.useState(!!initial.autoBackup);
  const [detecting, setDetecting] = React.useState(false);

  // Update flow state
  const [updateState, setUpdateState] = React.useState({ status: 'idle' }); // idle | checking | up_to_date | available | installing | error
  const [updateInfo, setUpdateInfo] = React.useState(null);
  const [installProgress, setInstallProgress] = React.useState(0);

  React.useEffect(() => {
    const onKey = (e) => { if (e.key === 'Escape') onClose(); };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);

  React.useEffect(() => {
    if (!window.SDM) return;
    return window.SDM.on('update-progress', ({ p }) => {
      setInstallProgress(Math.max(0, Math.min(100, Math.round(p * 100))));
    });
  }, []);

  const handleCheckUpdate = async () => {
    if (!window.SDM?.hasHost) { window.__toast?.('Bridge unavailable'); return; }
    setUpdateState({ status: 'checking' });
    setUpdateInfo(null);
    try {
      const info = await window.SDM.call('checkForUpdate');
      setUpdateInfo(info);
      setUpdateState({ status: info?.available ? 'available' : 'up_to_date' });
    } catch (e) {
      setUpdateState({ status: 'error', error: e.message });
    }
  };
  const handleInstallUpdate = async () => {
    if (!updateInfo?.downloadUrl) return;
    if (!window.confirm(`Install ${updateInfo.latest}? The app will close, update, and reopen.`)) return;
    setUpdateState({ status: 'installing' });
    setInstallProgress(0);
    try {
      const res = await window.SDM.call('applyUpdate', { downloadUrl: updateInfo.downloadUrl });
      if (!res?.ok) {
        setUpdateState({ status: 'error', error: res?.error || 'Update failed' });
      }
      // On success the app exits shortly; we may never reach here.
    } catch (e) {
      setUpdateState({ status: 'error', error: e.message });
    }
  };

  const handleBrowse = async () => {
    try {
      const res = await window.SDM?.call('pickCs2Folder');
      if (res && res.path) setPath(res.path);
    } catch (e) { window.__toast?.('Browse failed: ' + e.message); }
  };
  const handleDetect = async () => {
    if (detecting) return;
    setDetecting(true);
    try {
      const res = await window.SDM?.call('detectCs2Path');
      if (res && res.path) {
        setPath(res.path);
        window.__toast?.('Detected: ' + res.path);
      } else {
        window.__toast?.('Could not locate CS2 install');
      }
    } catch (e) {
      window.__toast?.('Detect failed: ' + e.message);
    } finally {
      setDetecting(false);
    }
  };
  const handleSave = async () => {
    const payload = { cs2Path: path.trim(), moveOnImport, autoBackup };
    try {
      await window.SDM?.call('saveSettings', payload);
      onSave && onSave(payload);
      window.__toast?.('Settings saved');
      onClose();
    } catch (e) {
      window.__toast?.('Save failed: ' + e.message);
    }
  };

  const valid = path.trim().length > 0;

  return (
    <div className="modal-scrim" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <div className="about-card-mark settings-card-mark" style={{ width: 28, height: 28 }}>
              <IconSettings size={15}/>
            </div>
            <div>
              <div style={{ fontSize: 13, fontWeight: 700, color: 'var(--fg)' }}>Settings</div>
              <div style={{ fontSize: 10.5, color: 'var(--mut)', fontFamily: 'JetBrains Mono, monospace', marginTop: 1 }}>paths · import · backup</div>
            </div>
          </div>
          <button className="ic-btn" onClick={onClose} aria-label="Close"><IconClose size={13}/></button>
        </div>
        <div className="modal-body">
          <div className="settings-group">
            <div className="settings-group-head">
              <div className="settings-group-title">CS2 install location</div>
              <div className="settings-group-sub">Destination when moving demo files into the game's replay folder.</div>
            </div>

            <div className="settings-field">
              <label className="settings-label">Replays directory</label>
              <div className="settings-path-row">
                <div className="settings-input-wrap">
                  <IconFolder size={13} style={{ color: 'var(--mut)', flexShrink: 0 }}/>
                  <input
                    className="settings-input"
                    type="text"
                    value={path}
                    onChange={(e) => setPath(e.target.value)}
                    placeholder="C:\Program Files (x86)\Steam\steamapps\common\..."
                    spellCheck={false}
                  />
                  {path && (
                    <button className="ic-btn" onClick={() => setPath('')} aria-label="Clear">
                      <IconClose size={11}/>
                    </button>
                  )}
                </div>
                <button className="btn btn-sm" onClick={handleBrowse}>Browse…</button>
              </div>
              <div className="settings-hints">
                <button className="settings-hint-btn" onClick={handleDetect} disabled={detecting}>
                  <IconCheck size={11}/> <span>{detecting ? 'Detecting…' : 'Auto-detect Steam install'}</span>
                </button>
                {path && (
                  <span className={'settings-status ' + (valid ? 'settings-status-ok' : 'settings-status-warn')}>
                    {valid ? 'Path looks valid' : 'Path is empty'}
                  </span>
                )}
              </div>
            </div>

            <label className="settings-toggle">
              <input type="checkbox" checked={moveOnImport} onChange={(e) => setMoveOnImport(e.target.checked)}/>
              <span className="settings-toggle-text">
                <span className="settings-toggle-title">Move files instead of copying</span>
                <span className="settings-toggle-sub">When importing into the CS2 folder, remove the source file.</span>
              </span>
            </label>
            <label className="settings-toggle">
              <input type="checkbox" checked={autoBackup} onChange={(e) => setAutoBackup(e.target.checked)}/>
              <span className="settings-toggle-text">
                <span className="settings-toggle-title">Keep a backup before moving</span>
                <span className="settings-toggle-sub">Copies the original to ~/SDM/backups before any destructive move.</span>
              </span>
            </label>
          </div>

          {/* ── Updates ───────────────────────────────────────────────── */}
          <div className="settings-group" style={{ marginTop: 14 }}>
            <div className="settings-group-head">
              <div className="settings-group-title">App update</div>
              <div className="settings-group-sub">
                Current version <span className="vx-mono" style={{ color: 'var(--fg)' }}>{window.SDM_VERSION || '—'}</span>.
                Checks <span style={{ color: 'var(--acc)' }}>github.com/{updateInfo ? '' : 'thedarkcr0w/Small-Demo-Manager'}</span> for newer releases.
              </div>
            </div>

            <div className="settings-path-row">
              <button
                className="btn"
                onClick={handleCheckUpdate}
                disabled={updateState.status === 'checking' || updateState.status === 'installing'}
              >
                <IconRefresh size={13}/>
                <span>{updateState.status === 'checking' ? 'Checking…' : 'Check for updates'}</span>
              </button>
              {updateState.status === 'available' && updateInfo?.downloadUrl && (
                <button className="btn btn-primary" onClick={handleInstallUpdate}>
                  <IconDownload size={13}/>
                  <span>Install {updateInfo.latest}</span>
                </button>
              )}
              {updateInfo?.releaseUrl && (
                <button className="btn btn-sm"
                        onClick={() => window.SDM?.call('openExternal', { url: updateInfo.releaseUrl }).catch(()=>{})}>
                  Release notes
                </button>
              )}
            </div>

            {updateState.status === 'up_to_date' && (
              <span className="settings-status settings-status-ok" style={{ alignSelf: 'flex-start' }}>
                You're on the latest version
              </span>
            )}
            {updateState.status === 'available' && updateInfo && (
              <div style={{ fontSize: 11.5, color: 'var(--sec)' }}>
                Update available: <strong style={{ color: 'var(--acc)' }}>{updateInfo.latest}</strong>
                {updateInfo.assetSize ? ` · ${(updateInfo.assetSize / 1048576).toFixed(1)} MB` : ''}
              </div>
            )}
            {updateState.status === 'installing' && (
              <div className="settings-field">
                <div style={{ fontSize: 11.5, color: 'var(--sec)' }}>
                  Downloading update… {installProgress}%
                </div>
                <div className="scan-bar"><div className="scan-bar-fill" style={{ width: installProgress + '%' }}/></div>
                <div style={{ fontSize: 11, color: 'var(--mut)' }}>
                  The app will close and reopen automatically when the update is applied.
                </div>
              </div>
            )}
            {updateState.status === 'error' && (
              <span className="settings-status settings-status-warn" style={{ alignSelf: 'flex-start', color: '#ff8a8a' }}>
                {updateState.error || 'Update failed'}
              </span>
            )}
          </div>
        </div>
        <div className="modal-foot" style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
          <button className="btn" onClick={onClose}>Cancel</button>
          <button className="btn btn-primary" onClick={handleSave} disabled={!valid}>Save settings</button>
        </div>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Sidebar — folder browser
// ─────────────────────────────────────────────────────────────────────────────
function Sidebar({ style, activeFolder, onFolder, counts, scan, activeTags, onTagFilter, onScan, onAddFolder, favCount, onOpenAbout, onOpenSettings, cs2Path, availableTags }) {
  const collapsed = style === 'rail';
  return (
    <aside className={'sb ' + (collapsed ? 'sb-rail' : '')}>
      {/* Quick views */}
      <nav className="sb-nav">
        <button className={'sb-item ' + (activeFolder === '*' ? 'sb-item-active' : '')} onClick={() => onFolder('*')}>
          <IconLibrary size={16}/>
          {!collapsed && <span className="sb-label">All demos</span>}
          {!collapsed && <span className="sb-count">{counts.all}</span>}
        </button>
        <button className={'sb-item ' + (activeFolder === 'recent' ? 'sb-item-active' : '')} onClick={() => onFolder('recent')}>
          <IconRecent size={16}/>
          {!collapsed && <span className="sb-label">Recent</span>}
          {!collapsed && <span className="sb-count">{counts.recent}</span>}
        </button>
        <button className={'sb-item ' + (activeFolder === 'favorites' ? 'sb-item-active' : '')} onClick={() => onFolder('favorites')}>
          <IconFavorite size={16}/>
          {!collapsed && <span className="sb-label">Favorites</span>}
          {!collapsed && <span className="sb-count">{favCount}</span>}
        </button>
      </nav>

      {!collapsed && (
        <>
          <div className="sb-sect-row">
            <span>Watched folders</span>
            <button className="ic-btn" onClick={onAddFolder} title="Add folder"><IconPlus size={12}/></button>
          </div>
          <nav className="sb-nav">
            {window.FOLDERS.map(f => (
              <button key={f.id} className={'sb-item sb-folder ' + (activeFolder === f.id ? 'sb-item-active' : '')}
                      onClick={() => onFolder(f.id)}>
                <IconFolder size={15}/>
                <span className="sb-folder-text">
                  <span className="sb-folder-label">{f.label}</span>
                  <span className="sb-folder-path">{f.path}</span>
                </span>
                <span className="sb-count">{f.count}</span>
              </button>
            ))}
          </nav>

          <div className="sb-sect">Filter by tag</div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, padding: '0 10px' }}>
            {availableTags && availableTags.length > 0
              ? availableTags.map(t => (
                  <button key={t} className={'chip chip-filter ' + (activeTags.includes(t) ? 'chip-filter-on' : '')}
                          onClick={() => onTagFilter(t)}>
                    {t}
                  </button>
                ))
              : (
                <span style={{ fontSize: 11, color: 'var(--mut)', padding: '4px 2px' }}>
                  No tags yet — add tags to a demo from its details panel.
                </span>
              )}
          </div>
        </>
      )}

      {collapsed && (
        <nav className="sb-nav" style={{ marginTop: 8 }}>
          {window.FOLDERS.map(f => (
            <button key={f.id} className={'sb-item ' + (activeFolder === f.id ? 'sb-item-active' : '')}
                    onClick={() => onFolder(f.id)} title={f.label}>
              <IconFolder size={15}/>
            </button>
          ))}
        </nav>
      )}

      {!collapsed && <div style={{ flex: 1 }}/>}

      {!collapsed && (
        <div style={{ padding: '0 10px 10px', display: 'flex', flexDirection: 'column', gap: 6 }}>
          <SidebarSettings onOpen={onOpenSettings} cs2Path={cs2Path}/>
          <SidebarAbout onOpen={onOpenAbout}/>
        </div>
      )}

      <div className="sb-foot">
        {!collapsed && scan.scanning ? (
          <div className="scan-card">
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 11, color: 'var(--sec)' }}>
              <span>Scanning…</span>
              <span style={{ fontFamily: 'JetBrains Mono, monospace' }}>{scan.progress}%</span>
            </div>
            <div className="scan-bar"><div className="scan-bar-fill" style={{ width: scan.progress + '%' }}/></div>
            <div style={{ fontSize: 10, color: 'var(--mut)', marginTop: 4, fontFamily: 'JetBrains Mono, monospace', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{scan.path}</div>
            <div style={{ fontSize: 10, color: 'var(--mut)' }}>{scan.found} demos indexed</div>
          </div>
        ) : !collapsed ? (
          <button className="scan-btn" onClick={onScan}>
            <IconRefresh size={13}/> <span>Rescan all folders</span>
          </button>
        ) : (
          <button className="ic-btn" onClick={onScan} title="Rescan"><IconRefresh size={14}/></button>
        )}
      </div>
    </aside>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Top bar — search, filters, layout switch, sort
// ─────────────────────────────────────────────────────────────────────────────
function TopBar({ query, onQuery, layout, onLayout, sort, onSort, mapFilter, onMapFilter, totalShown, totalAll, onAddFolder }) {
  const sortOptions = [
    ['date-desc', 'Newest first'],
    ['date-asc', 'Oldest first'],
    ['size-desc', 'Largest'],
    ['dur-desc', 'Longest'],
    ['map-asc', 'Map A–Z'],
  ];
  const [sortOpen, setSortOpen] = React.useState(false);
  const [mapOpen, setMapOpen] = React.useState(false);
  return (
    <div className="topbar">
      <div className="search-wrap">
        <IconSearch size={15} style={{ color: 'var(--mut)' }}/>
        <input
          className="search"
          placeholder="Search by filename, player, server, tag…"
          value={query}
          onChange={(e) => onQuery(e.target.value)}
        />
        {query && (
          <button className="ic-btn" onClick={() => onQuery('')} aria-label="Clear">
            <IconClose size={13}/>
          </button>
        )}
        <span className="kbd">Ctrl K</span>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <div style={{ position: 'relative' }}>
          <button className={'btn-quiet ' + (mapFilter ? 'btn-quiet-active' : '')} onClick={() => setMapOpen(!mapOpen)}>
            <IconFilter size={13}/>
            <span>{mapFilter ? window.MAPS[mapFilter].name : 'Map: All'}</span>
            <IconChevronD size={12}/>
          </button>
          {mapOpen && (
            <div className="dropdown" onMouseLeave={() => setMapOpen(false)}>
              <button className="menu-item" onClick={() => { onMapFilter(null); setMapOpen(false); }}>All maps</button>
              {Object.entries(window.MAPS).map(([k, m]) => (
                <button key={k} className="menu-item" onClick={() => { onMapFilter(k); setMapOpen(false); }}>
                  <span style={{ width: 8, height: 8, borderRadius: 2, background: m.tint, display: 'inline-block' }}/>
                  {m.name}
                </button>
              ))}
            </div>
          )}
        </div>

        <div style={{ position: 'relative' }}>
          <button className="btn-quiet" onClick={() => setSortOpen(!sortOpen)}>
            <IconSort size={13}/>
            <span>{sortOptions.find(s => s[0] === sort)?.[1]}</span>
            <IconChevronD size={12}/>
          </button>
          {sortOpen && (
            <div className="dropdown" onMouseLeave={() => setSortOpen(false)}>
              {sortOptions.map(([k, l]) => (
                <button key={k} className="menu-item" onClick={() => { onSort(k); setSortOpen(false); }}>
                  {sort === k && <IconCheck size={12} style={{ color: 'var(--acc)' }}/>}
                  <span style={{ marginLeft: sort === k ? 0 : 18 }}>{l}</span>
                </button>
              ))}
            </div>
          )}
        </div>

        <div className="layout-switch">
          <button className={layout === 'table' ? 'on' : ''} onClick={() => onLayout('table')} aria-label="Table"><IconRows size={13}/></button>
          <button className={layout === 'cards' ? 'on' : ''} onClick={() => onLayout('cards')} aria-label="Cards"><IconGrid size={13}/></button>
          <button className={layout === 'hybrid' ? 'on' : ''} onClick={() => onLayout('hybrid')} aria-label="Hybrid"><IconSplit size={13}/></button>
        </div>

        <div style={{ width: 1, height: 22, background: 'var(--border)' }}/>

        <div style={{ fontSize: 11, color: 'var(--mut)', fontFamily: 'JetBrains Mono, monospace' }}>
          {totalShown}/{totalAll}
        </div>

        <button className="btn btn-primary btn-sm" onClick={onAddFolder}>
          <IconPlus size={13}/> <span>Add folder</span>
        </button>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Library — table row
// ─────────────────────────────────────────────────────────────────────────────
function TableRow({ demo, selected, onSelect, onFavorite, dense }) {
  const map = window.MAPS[demo.map];
  return (
    <div className={'row ' + (selected ? 'row-active' : '')} onClick={() => onSelect(demo.id)}>
      <button className="row-fav" onClick={(e) => { e.stopPropagation(); onFavorite(demo.id); }}
              style={{ color: demo.fav ? 'var(--acc)' : 'var(--mut)' }} aria-label="Favorite">
        <IconFavorite size={13} fill={demo.fav}/>
      </button>
      <div className="row-map">
        <span className="map-pill" style={{ background: map.tint + '22', color: map.tint, borderColor: map.tint + '55' }}>{map.name}</span>
      </div>
      <div className="row-file">
        <div className="row-file-name" title={demo.file}>{demo.file}</div>
        <div className="row-meta">
          <span>{demo.server}</span>
          <span className="dot">·</span>
          <span>{demo.t1} <span style={{ color: 'var(--mut)' }}>vs</span> {demo.t2}</span>
        </div>
      </div>
      <div className="row-score">
        <span style={{ color: demo.s1 > demo.s2 ? 'var(--acc)' : 'var(--sec)', fontWeight: 600 }}>{demo.s1}</span>
        <span style={{ color: 'var(--mut)', margin: '0 4px' }}>:</span>
        <span style={{ color: demo.s2 > demo.s1 ? 'var(--acc)' : 'var(--sec)', fontWeight: 600 }}>{demo.s2}</span>
      </div>
      <div className="row-source">
        <span className={'src-tag src-' + demo.source}>{demo.source}</span>
      </div>
      <div className="row-tags">
        {demo.tags.slice(0, 2).map((t, i) => <span key={i} className="chip chip-mini">{t}</span>)}
        {demo.tags.length > 2 && <span className="chip chip-mini chip-more">+{demo.tags.length - 2}</span>}
      </div>
      <div className="row-num">{window.fmtDur(demo.dur)}</div>
      <div className="row-num">{window.fmtSize(demo.size)}</div>
      <div className="row-num row-date">{window.fmtDate(demo.date)}</div>
    </div>
  );
}

function TableHeader() {
  return (
    <div className="row row-head">
      <div></div>
      <div>Map</div>
      <div>File · Match</div>
      <div style={{ textAlign: 'center' }}>Score</div>
      <div>Source</div>
      <div>Tags</div>
      <div style={{ textAlign: 'right' }}>Dur</div>
      <div style={{ textAlign: 'right' }}>Size</div>
      <div style={{ textAlign: 'right' }}>Date</div>
    </div>
  );
}

function LibraryTable({ demos, selectedId, onSelect, onFavorite }) {
  return (
    <div className="lib-table">
      <TableHeader/>
      <div className="lib-rows">
        {demos.map((d) => (
          <TableRow key={d.id} demo={d} selected={d.id === selectedId} onSelect={onSelect} onFavorite={onFavorite}/>
        ))}
        {demos.length === 0 && (
          <div style={{ padding: 60, textAlign: 'center', color: 'var(--mut)', fontSize: 13 }}>
            No demos match your filters.
          </div>
        )}
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Cards layout
// ─────────────────────────────────────────────────────────────────────────────
function LibraryCards({ demos, selectedId, onSelect, onFavorite }) {
  return (
    <div className="lib-cards">
      {demos.map(d => {
        const map = window.MAPS[d.map];
        return (
          <div key={d.id} className={'card ' + (d.id === selectedId ? 'card-active' : '')} onClick={() => onSelect(d.id)}>
            <div className="card-banner" style={{
              backgroundImage: `repeating-linear-gradient(45deg, ${map.tint}22 0 8px, transparent 8px 16px), linear-gradient(135deg, ${map.tint}55, ${map.tint}10)`,
            }}>
              <div className="card-banner-overlay"/>
              <button className="row-fav card-fav" onClick={(e) => { e.stopPropagation(); onFavorite(d.id); }}
                      style={{ color: d.fav ? 'var(--acc)' : 'rgba(255,255,255,.6)' }}>
                <IconFavorite size={14} fill={d.fav}/>
              </button>
              <div className="card-banner-info">
                <div style={{ fontSize: 10, color: 'rgba(255,255,255,.65)', textTransform: 'uppercase', letterSpacing: '.08em' }}>{d.map}</div>
                <div style={{ fontSize: 16, color: '#fff', fontWeight: 700, marginTop: 2 }}>{map.name}</div>
              </div>
              <div className="card-score">
                <span style={{ color: d.s1 > d.s2 ? 'var(--acc)' : '#cbd5e1' }}>{d.s1}</span>
                <span style={{ color: 'rgba(255,255,255,.5)', margin: '0 2px' }}>:</span>
                <span style={{ color: d.s2 > d.s1 ? 'var(--acc)' : '#cbd5e1' }}>{d.s2}</span>
              </div>
            </div>
            <div className="card-body">
              <div className="row-file-name" style={{ fontFamily: 'JetBrains Mono, monospace', fontSize: 11 }}>{d.file}</div>
              <div className="row-meta" style={{ marginTop: 4 }}>
                <span>{d.t1} <span style={{ color: 'var(--mut)' }}>vs</span> {d.t2}</span>
              </div>
              <div style={{ display: 'flex', gap: 6, marginTop: 8, flexWrap: 'wrap' }}>
                <span className={'src-tag src-' + d.source}>{d.source}</span>
                {d.tags.slice(0, 2).map((t, i) => <span key={i} className="chip chip-mini">{t}</span>)}
              </div>
              <div className="card-foot">
                <span>{window.fmtDate(d.date)}</span>
                <span>·</span>
                <span>{window.fmtDur(d.dur)}</span>
                <span>·</span>
                <span>{window.fmtSize(d.size)}</span>
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}

function FolderHeader({ activeFolder, filtered, totalAll, onCollapse }) {
  let label, path, icon;
  if (activeFolder === '*') {
    label = 'All demos'; path = `${window.FOLDERS.length} watched folders`; icon = <IconLibrary size={15}/>;
  } else if (activeFolder === 'favorites') {
    label = 'Favorites'; path = `${filtered.length} favorited demo${filtered.length === 1 ? '' : 's'}`; icon = <IconFavorite size={15}/>;
  } else if (activeFolder === 'recent') {
    label = 'Recent'; path = 'Last 7 days'; icon = <IconRecent size={15}/>;
  } else {
    const f = window.FOLDERS.find(x => x.id === activeFolder);
    label = f.label; path = f.path; icon = <IconFolder size={15}/>;
  }
  return (
    <div className="folder-header">
      <div className="folder-header-icon">{icon}</div>
      <div className="folder-header-text">
        <div className="folder-header-name">{label}</div>
        <div className="folder-header-meta">{path}</div>
      </div>
      <div style={{ fontSize: 11, color: 'var(--mut)', fontFamily: 'JetBrains Mono, monospace', flexShrink: 0 }}>
        {filtered.length}/{totalAll}
      </div>
      <button className="ic-btn" onClick={onCollapse} title="Collapse list (⌘\)" aria-label="Collapse demo list">
        <IconCollapse size={14}/>
      </button>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// App root
// ─────────────────────────────────────────────────────────────────────────────
function App() {
  const [t, setTweak] = useTweaks(TWEAK_DEFAULTS);
  const [demos, setDemos] = React.useState(() => window.DEMOS.slice());
  const [folders, setFolders] = React.useState(() => (window.FOLDERS || []).slice());
  const [selectedId, setSelectedId] = React.useState(() => {
    if (window.STARTUP_DEMO) {
      const m = window.DEMOS.find(d => d.fullPath === window.STARTUP_DEMO);
      if (m) return m.id;
    }
    return window.DEMOS[0]?.id || null;
  });
  const [query, setQuery] = React.useState('');
  const [sort, setSort] = React.useState('date-desc');
  const [mapFilter, setMapFilter] = React.useState(null);
  const [activeTags, setActiveTags] = React.useState([]);
  const [activeFolder, setActiveFolder] = React.useState('*');
  const [scan, setScan] = React.useState({ scanning: false, progress: 0, path: '', found: 0 });
  const [listCollapsed, setListCollapsed] = React.useState(false);
  const [voiceOpen, setVoiceOpen] = React.useState(false);
  const [aboutOpen, setAboutOpen] = React.useState(false);
  const [settingsOpen, setSettingsOpen] = React.useState(false);
  const [settings, setSettings] = React.useState(() => window.SDM_SETTINGS || { cs2Path: '', moveOnImport: true, autoBackup: false });
  const prevListCollapsedRef = React.useRef(false);
  const parseInFlight = React.useRef(new Set());

  // Keep window.FOLDERS in sync so the Sidebar component reads fresh data.
  React.useEffect(() => { window.FOLDERS = folders; }, [folders]);

  // Auto-collapse the list pane while in Voice mode so the 3-column workspace has room.
  // Restore the user's prior state on exit.
  const onVoiceOpen = React.useCallback((open) => {
    setVoiceOpen(open);
    if (open) {
      prevListCollapsedRef.current = listCollapsed;
      setListCollapsed(true);
    } else {
      setListCollapsed(prevListCollapsedRef.current);
    }
  }, [listCollapsed]);

  // Apply accent live
  React.useEffect(() => {
    const root = document.documentElement;
    root.style.setProperty('--acc', t.accent);
    // soft accent — same hue, low alpha
    root.style.setProperty('--acc-soft', t.accent + '33');
    root.style.setProperty('--acc-strong', t.accent);
  }, [t.accent]);

  // Filter + sort
  const filtered = React.useMemo(() => {
    let out = demos.slice();
    if (activeFolder === 'favorites') out = out.filter(d => d.fav);
    else if (activeFolder === 'recent') {
      const cutoff = Date.now() - 7 * 86400000;
      out = out.filter(d => new Date(d.date).getTime() > cutoff);
    } else if (activeFolder !== '*') {
      out = out.filter(d => d.folderId === activeFolder);
    }
    if (mapFilter) out = out.filter(d => d.map === mapFilter);
    if (activeTags.length) {
      out = out.filter(d => activeTags.every(tg => d.tags.some(x => x.toLowerCase() === tg.toLowerCase())));
    }
    if (query) {
      const q = query.toLowerCase();
      out = out.filter(d =>
        d.file.toLowerCase().includes(q) ||
        d.t1.toLowerCase().includes(q) ||
        d.t2.toLowerCase().includes(q) ||
        d.server.toLowerCase().includes(q) ||
        d.tags.some(x => x.toLowerCase().includes(q)) ||
        d.players1.some(p => p.name.toLowerCase().includes(q)) ||
        d.players2.some(p => p.name.toLowerCase().includes(q))
      );
    }
    out.sort((a, b) => {
      switch (sort) {
        case 'date-asc':  return new Date(a.date) - new Date(b.date);
        case 'size-desc': return b.size - a.size;
        case 'dur-desc':  return b.dur - a.dur;
        case 'map-asc':   return a.map.localeCompare(b.map);
        default:          return new Date(b.date) - new Date(a.date);
      }
    });
    return out;
  }, [demos, activeFolder, mapFilter, activeTags, query, sort]);

  const selected = demos.find(d => d.id === selectedId) || filtered[0];

  // Counts for sidebar
  const counts = React.useMemo(() => {
    const bySource = {};
    demos.forEach(d => { bySource[d.source] = (bySource[d.source] || 0) + 1; });
    const cutoff = Date.now() - 7 * 86400000;
    return {
      all: demos.length,
      recent: demos.filter(d => new Date(d.date).getTime() > cutoff).length,
      fav: demos.filter(d => d.fav).length,
      folders: folders.length,
      bySource,
    };
  }, [demos, folders]);

  // Recompute per-folder counts whenever demos change so the sidebar's badge stays accurate.
  React.useEffect(() => {
    setFolders(fs => fs.map(f => ({ ...f, count: demos.filter(d => d.folderId === f.id).length })));
  }, [demos]);

  // Tags currently in use across the library, in stable alphabetical order.
  const availableTags = React.useMemo(() => {
    const set = new Set();
    demos.forEach(d => d.tags.forEach(t => set.add(t)));
    return Array.from(set).sort((a, b) => a.localeCompare(b));
  }, [demos]);

  const onFavorite = (id) => {
    const current = demos.find(d => d.id === id);
    if (!current) return;
    const nextFav = !current.fav;
    setDemos(ds => ds.map(d => d.id === id ? { ...d, fav: nextFav } : d));
    window.SDM?.call('toggleFavorite', { demoId: id, fav: nextFav }).catch(() => {});
  };
  const onTagToggle = (id, tag, isAdd) => {
    const current = demos.find(d => d.id === id);
    if (!current) return;
    const nextTags = isAdd
      ? (current.tags.includes(tag) ? current.tags : [...current.tags, tag])
      : current.tags.filter(x => x !== tag);
    setDemos(ds => ds.map(d => d.id === id ? { ...d, tags: nextTags } : d));
    window.SDM?.call('setTags', { demoId: id, tags: nextTags }).catch(() => {});
  };
  const onNoteChange = (id, note) => {
    setDemos(ds => ds.map(d => d.id === id ? { ...d, note } : d));
    window.SDM?.call('setNote', { demoId: id, note }).catch(() => {});
  };
  const onPlayerAction = async (action, player, demo) => {
    if (!player || !demo) return;
    const steamId = player.steamId || '';
    const links = window.PLAYER_PROFILE_LINKS || {};
    const openProfile = async (prefix) => {
      if (!steamId) { window.__toast?.('SteamID64 unavailable'); return; }
      const ok = await window.SDM?.call('openExternal', { url: prefix + steamId }).catch(() => false);
      if (!ok) window.__toast?.('Could not open profile');
    };

    switch (action) {
      case 'copySteamId': {
        if (!steamId) { window.__toast?.('SteamID64 unavailable'); return; }
        const ok = await window.SDM?.call('copyToClipboard', { text: steamId }).catch(() => false);
        window.__toast?.(ok ? `Copied SteamID64 for ${player.name}` : 'Copy failed');
        return;
      }
      case 'openSteam':   return openProfile(links.steam || 'http://steamcommunity.com/profiles/');
      case 'openCswatch': return openProfile(links.cswatch || 'https://cswatch.in/player/');
      case 'openLeetify': return openProfile(links.leetify || 'https://leetify.com/app/profile/');
      case 'openCsstats': return openProfile(links.csstats || 'https://csstats.gg/player/');
      default:
        window.__toast?.('Unknown player action');
    }
  };
  const onTagFilter = (tag) => {
    setActiveTags(ts => ts.includes(tag) ? ts.filter(x => x !== tag) : [...ts, tag]);
  };
  const onAction = async (action) => {
    if (!selected) return;
    switch (action) {
      case 'move-to-cs2': {
        const res = await window.SDM?.call('moveToCs2', { demoId: selected.id }).catch(e => ({ ok: false, error: e.message }));
        if (res?.ok) {
          window.__toast?.('Moved to CS2');
          // Drop the demo from the library list since it's no longer in a watched folder.
          setDemos(ds => ds.filter(d => d.id !== selected.id));
        } else {
          window.__toast?.(res?.error || 'Move failed');
        }
        return;
      }
      case 'reveal':
        await window.SDM?.call('revealInFolder', { path: selected.fullPath }).catch(() => {});
        return;
      case 'rename': {
        const proposed = selected.file.replace(/\.dem$/i, '');
        const name = window.prompt('New filename (without .dem):', proposed);
        if (!name) return;
        const res = await window.SDM?.call('renameDemo', { demoId: selected.id, newName: name });
        if (res?.ok) {
          setDemos(ds => ds.map(d => d.id === selected.id
            ? { ...d, id: res.newId || d.id, fullPath: res.newPath, file: res.newPath.split(/[\\/]/).pop() }
            : d));
          if (res.newId) setSelectedId(res.newId);
          window.__toast?.('Renamed');
        } else {
          window.__toast?.(res?.error || 'Rename failed');
        }
        return;
      }
      case 'delete': {
        if (!window.confirm('Delete this demo file from disk?')) return;
        const res = await window.SDM?.call('deleteDemo', { demoId: selected.id });
        if (res?.ok) {
          setDemos(ds => ds.filter(d => d.id !== selected.id));
          window.__toast?.('Deleted');
        } else {
          window.__toast?.(res?.error || 'Delete failed');
        }
        return;
      }
      default:
        window.__toast?.('Action: ' + action);
    }
  };
  const onScan = async () => {
    if (scan.scanning) return;
    if (!window.SDM?.hasHost) { window.__toast?.('Bridge unavailable'); return; }
    setScan({ scanning: true, progress: 0, path: '', found: 0 });
    try {
      const res = await window.SDM.call('scanAll');
      const fresh = (res?.demos || []).map(window.SDM_NORMALIZE_DEMO);
      setDemos(fresh);
    } catch (e) {
      window.__toast?.('Scan failed: ' + e.message);
    } finally {
      setScan({ scanning: false, progress: 100, path: '', found: 0 });
    }
  };
  const onAddFolder = async () => {
    if (!window.SDM?.hasHost) { window.__toast?.('Bridge unavailable'); return; }
    try {
      const res = await window.SDM.call('addFolder');
      if (!res) return; // user cancelled
      setFolders(fs => fs.some(f => f.id === res.folder.id) ? fs : [...fs, res.folder]);
      const fresh = (res.demos || []).map(window.SDM_NORMALIZE_DEMO);
      setDemos(ds => {
        const known = new Set(ds.map(d => d.id));
        return [...ds, ...fresh.filter(d => !known.has(d.id))];
      });
      window.__toast?.(`Added "${res.folder.label}" (${(res.demos || []).length} demos)`);
    } catch (e) {
      window.__toast?.('Add folder failed: ' + e.message);
    }
  };

  // Listen for scan-progress events from the host while a scan runs.
  React.useEffect(() => {
    if (!window.SDM) return;
    const off = window.SDM.on('scan-progress', ({ progress, path, found }) => {
      setScan(s => ({ ...s, scanning: progress < 100, progress, path: path || '', found: found || 0 }));
    });
    return off;
  }, []);

  // Lazy-parse the selected demo if we only have file-level info so far.
  React.useEffect(() => {
    if (!selectedId) return;
    const current = demos.find(d => d.id === selectedId);
    if (!current || current.parsed) return;
    if (parseInFlight.current.has(selectedId)) return;
    parseInFlight.current.add(selectedId);
    window.SDM?.call('parseDemo', { demoId: selectedId })
      .then(dto => {
        parseInFlight.current.delete(selectedId);
        if (!dto) return;
        const norm = window.SDM_NORMALIZE_DEMO(dto);
        setDemos(ds => ds.map(d => d.id === norm.id ? { ...d, ...norm } : d));
      })
      .catch(() => { parseInFlight.current.delete(selectedId); });
  }, [selectedId, demos]);

  // ── Keyboard ↑/↓ navigation in list
  React.useEffect(() => {
    const onKey = (e) => {
      if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
      const idx = filtered.findIndex(d => d.id === selectedId);
      if (e.key === 'ArrowDown' && idx < filtered.length - 1) { e.preventDefault(); setSelectedId(filtered[idx + 1].id); }
      if (e.key === 'ArrowUp' && idx > 0) { e.preventDefault(); setSelectedId(filtered[idx - 1].id); }
      if ((e.key === 'k' || e.key === 'K') && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        document.querySelector('.search')?.focus();
      }
      if ((e.key === '\\') && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        setListCollapsed(c => !c);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [filtered, selectedId]);

  const showSplit = t.layout === 'hybrid' || t.layout === 'table';

  return (
    <WindowChrome>
      <div className="app-grid">
        <Sidebar
          style={t.sidebarStyle}
          activeFolder={activeFolder}
          onFolder={setActiveFolder}
          counts={counts}
          favCount={counts.fav}
          scan={scan}
          activeTags={activeTags}
          onTagFilter={onTagFilter}
          onScan={onScan}
          onAddFolder={onAddFolder}
          onOpenAbout={() => setAboutOpen(true)}
          onOpenSettings={() => setSettingsOpen(true)}
          cs2Path={settings.cs2Path}
          availableTags={availableTags}
        />
        <main className="main">
          <TopBar
            query={query} onQuery={setQuery}
            layout={t.layout} onLayout={(v) => setTweak('layout', v)}
            sort={sort} onSort={setSort}
            mapFilter={mapFilter} onMapFilter={setMapFilter}
            totalShown={filtered.length} totalAll={demos.length}
            onAddFolder={onAddFolder}
          />

          {/* Active filter chips */}
          {(activeTags.length > 0 || mapFilter) && (
            <div className="active-filters">
              <span style={{ fontSize: 11, color: 'var(--mut)', textTransform: 'uppercase', letterSpacing: '.08em', fontWeight: 600 }}>Filters</span>
              {mapFilter && (
                <span className="chip chip-active">
                  Map: {window.MAPS[mapFilter].name}
                  <button className="chip-x" onClick={() => setMapFilter(null)}><IconClose size={10}/></button>
                </span>
              )}
              {activeTags.map(t => (
                <span key={t} className="chip chip-active">
                  {t}
                  <button className="chip-x" onClick={() => onTagFilter(t)}><IconClose size={10}/></button>
                </span>
              ))}
              <button className="link-btn" onClick={() => { setActiveTags([]); setMapFilter(null); }}>Clear all</button>
            </div>
          )}

          <div className={'content content-' + t.layout + (listCollapsed ? ' list-collapsed' : '')}>
            {!listCollapsed && (
              <div className="list-pane">
                <FolderHeader activeFolder={activeFolder} filtered={filtered} totalAll={demos.length} onCollapse={() => setListCollapsed(true)}/>
                {t.layout === 'cards'
                  ? <LibraryCards demos={filtered} selectedId={selectedId} onSelect={setSelectedId} onFavorite={onFavorite}/>
                  : <LibraryTable demos={filtered} selectedId={selectedId} onSelect={setSelectedId} onFavorite={onFavorite}/>}
              </div>
            )}
            {listCollapsed && (
              <button className="reveal-list" onClick={() => setListCollapsed(false)} title="Show demo list (⌘\)">
                <IconExpand size={14}/>
                <span className="reveal-list-label">Show list</span>
                <span className="reveal-count">{filtered.length}</span>
              </button>
            )}
            {showSplit && (
              <div className="details-pane">
                <DetailsPanel
                  demo={selected}
                  onAction={onAction}
                  onTagToggle={onTagToggle}
                  onNoteChange={onNoteChange}
                  onFavorite={onFavorite}
                  onPlayerAction={onPlayerAction}
                  voiceOpen={voiceOpen}
                  onVoiceOpen={onVoiceOpen}
                />
              </div>
            )}
          </div>
        </main>
      </div>

      <TweaksPanel>
        <TweakSection label="Theme"/>
        <TweakColor label="Accent" value={t.accent}
                    options={['#f59e0b','#3b82f6','#22c55e','#ef4444','#a78bfa','#e879f9']}
                    onChange={(v) => setTweak('accent', v)}/>
        <TweakSection label="Layout"/>
        <TweakRadio label="View"
                    value={t.layout}
                    options={[{value:'table',label:'Table'},{value:'cards',label:'Cards'},{value:'hybrid',label:'Hybrid'}]}
                    onChange={(v) => setTweak('layout', v)}/>
        <TweakSection label="Sidebar"/>
        <TweakRadio label="Style"
                    value={t.sidebarStyle}
                    options={[{value:'full',label:'Full'},{value:'rail',label:'Rail'}]}
                    onChange={(v) => setTweak('sidebarStyle', v)}/>
      </TweaksPanel>

      {aboutOpen && <AboutModal onClose={() => setAboutOpen(false)}/>}
      {settingsOpen && (
        <SettingsModal
          onClose={() => setSettingsOpen(false)}
          initial={settings}
          onSave={(s) => setSettings(s)}
        />
      )}
    </WindowChrome>
  );
}

window.App = App;
