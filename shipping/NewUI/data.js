// Map registry — known CS2 maps + tinted swatches for the UI.
const _MAPS_KNOWN = {
  de_mirage:   { name: 'Mirage',   tint: '#caa46a', hue: 38 },
  de_inferno:  { name: 'Inferno',  tint: '#b35a3a', hue: 18 },
  de_dust2:    { name: 'Dust II',  tint: '#c2a06a', hue: 42 },
  de_nuke:     { name: 'Nuke',     tint: '#7e8794', hue: 220 },
  de_overpass: { name: 'Overpass', tint: '#7aa37c', hue: 140 },
  de_anubis:   { name: 'Anubis',   tint: '#d2b572', hue: 48 },
  de_ancient:  { name: 'Ancient',  tint: '#5e8a64', hue: 150 },
  de_vertigo:  { name: 'Vertigo',  tint: '#6a8aa3', hue: 210 },
  de_train:    { name: 'Train',    tint: '#8b8478', hue: 32 },
  de_basalt:   { name: 'Basalt',   tint: '#5e6873', hue: 215 },
  de_brewery:  { name: 'Brewery',  tint: '#b06b3a', hue: 22 },
  de_dogtown:  { name: 'Dogtown',  tint: '#c4956b', hue: 30 },
  de_jura:     { name: 'Jura',     tint: '#7f8c6e', hue: 90 },
  de_unknown:  { name: 'Unknown',  tint: '#5b6470', hue: 220 },
};

// Proxy: unknown keys synthesize a neutral entry so the UI never reads `undefined.tint`.
window.MAPS = new Proxy(_MAPS_KNOWN, {
  get(target, key) {
    if (key in target) return target[key];
    if (typeof key !== 'string') return target.de_unknown;
    const stripped = key.replace(/^de_/, '').replace(/_se$/, '').replace(/_/g, ' ');
    const name = stripped ? stripped.charAt(0).toUpperCase() + stripped.slice(1) : 'Unknown';
    return { name, tint: target.de_unknown.tint, hue: target.de_unknown.hue };
  },
});

window.ALL_TAGS = ['Faceit','MM','ESEA','Scrim','Pug','Tournament','Server',
  'good clutch','cheater review','aim','admin','retake','finals','OT','tournament'];

// Populated at runtime by SDM_HYDRATE().
window.DEMOS = [];
window.FOLDERS = [];
window.STARTUP_DEMO = null;
window.SDM_SETTINGS = { cs2Path: '', moveOnImport: true, autoBackup: false };
window.SDM_VERSION = '';

window.getMap = function (key) {
  return window.MAPS[key] || window.MAPS.de_unknown;
};

window.SDM_NORMALIZE_DEMO = function (d) {
  if (!d) return d;
  return {
    id: d.id,
    file: d.file || '',
    fullPath: d.fullPath || '',
    folderId: d.folderId || '',
    folderPath: d.folderPath || '',
    map: d.map || 'de_unknown',
    date: d.date || '',
    dur: d.dur || 0,
    size: d.size || 0,
    tick: d.tick || 64,
    server: d.server || '',
    source: d.source || 'Other',
    t1: d.t1 || 'Team A',
    t2: d.t2 || 'Team B',
    s1: d.s1 || 0,
    s2: d.s2 || 0,
    tags: Array.isArray(d.tags) ? d.tags.slice() : [],
    fav: !!d.fav,
    note: d.note || '',
    parsed: !!d.parsed,
    parseError: d.parseError || null,
    players1: Array.isArray(d.players1) ? d.players1 : [],
    players2: Array.isArray(d.players2) ? d.players2 : [],
    rounds: Array.isArray(d.rounds) ? d.rounds : [],
  };
};

window.SDM_HYDRATE = function (state) {
  const folders = (state && state.folders) || [];
  const demos   = (state && state.demos) || [];
  window.FOLDERS = folders.map(f => ({
    id: f.id,
    label: f.label,
    path: f.path,
    source: f.source,
    count: typeof f.count === 'number' ? f.count : 0,
  }));
  window.DEMOS = demos.map(window.SDM_NORMALIZE_DEMO);
  window.STARTUP_DEMO = state && state.startupDemo || null;
  window.SDM_SETTINGS = {
    cs2Path: (state && state.cs2Path) || '',
    moveOnImport: state && typeof state.moveOnImport === 'boolean' ? state.moveOnImport : true,
    autoBackup: !!(state && state.autoBackup),
  };
  window.SDM_VERSION = (state && state.appVersion) || '';
};

window.fmtDate = function (iso) {
  if (!iso) return '—';
  const d = new Date(iso);
  if (isNaN(d.getTime())) return iso;
  const now = new Date();
  const diff = (now - d) / 86400000;
  if (diff < 1) return 'Today, ' + d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  if (diff < 2) return 'Yesterday';
  if (diff < 7) return Math.floor(diff) + 'd ago';
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
};

window.fmtDur = function (s) {
  if (!s) return '0:00';
  const m = Math.floor(s / 60), ss = s % 60;
  return m + ':' + String(ss).padStart(2, '0');
};

window.fmtSize = function (mb) {
  if (!mb) return '0 MB';
  return mb >= 1024 ? (mb / 1024).toFixed(2) + ' GB' : mb.toFixed(1) + ' MB';
};
