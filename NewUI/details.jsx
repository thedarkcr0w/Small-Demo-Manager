// Details panel — modernized "match view" mirroring the original SmallDemoManager layout
// Drag-drop path strip → side-by-side scoreboards with select-checkboxes →
// Copy Cmd. footer → tags / notes / actions

function FilePathStrip({ demo, onMoveToCS2 }) {
  return (
    <div className="filepath-row">
      <div className="dropzone">
        <div className="dropzone-icon">
          <IconCS size={13}/>
        </div>
        <div className="dropzone-text">
          <div className="dropzone-hint">Loaded demo · drop a .dem to swap</div>
          <div className="dropzone-path">{demo.fullPath || ('D:\\NewCS2FaceitDemos\\' + demo.file)}</div>
        </div>
      </div>
      <button className="btn btn-primary btn-tall" onClick={onMoveToCS2}>
        <IconPlay size={13}/>
        <span>Move to CS2</span>
      </button>
    </div>
  );
}

function MatchCenter({ demo, onOpenVoice }) {
  const map = window.MAPS[demo.map];
  const winA = demo.s1 > demo.s2;
  // Layered background: dark tint gradient on top + map thumbnail underneath.
  // If the thumbnail at /maps/<key>.jpg is missing the browser silently drops
  // that layer and the gradient still renders, so the panel never breaks.
  const tint = map.tint;
  return (
    <div className="match-center" style={{
      backgroundColor: tint + '14',
      backgroundImage: `linear-gradient(180deg, rgba(8,10,14,0.55) 0%, rgba(8,10,14,0.78) 100%), url("maps/${demo.map}.png")`,
      backgroundSize: 'cover',
      backgroundPosition: 'center center',
      backgroundRepeat: 'no-repeat',
      border: '1px solid var(--border)',
    }}>
      <div className="match-center-overlay"/>
      <div style={{ position: 'relative', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'space-between', height: '100%', padding: '10px 6px' }}>
        <button className="voice-btn" onClick={onOpenVoice} title="Per-player voice extractor">
          <IconMic size={12}/>
          <span>Voice extractor</span>
        </button>
        <div style={{ textAlign: 'center' }}>
          <div style={{ fontSize: 9.5, color: 'var(--mut)', textTransform: 'uppercase', letterSpacing: '.12em', fontWeight: 700 }}>{demo.map}</div>
          <div style={{ fontSize: 17, color: '#fff', fontWeight: 700, marginTop: 1, letterSpacing: '.01em' }}>{map.name}</div>
        </div>
        <div style={{ textAlign: 'center' }}>
          <div style={{ fontSize: 11, color: 'var(--mut)', fontWeight: 600, letterSpacing: '.18em' }}>VS</div>
          <div style={{ marginTop: 6, display: 'flex', alignItems: 'baseline', gap: 6, justifyContent: 'center', fontFamily: 'JetBrains Mono, monospace', fontVariantNumeric: 'tabular-nums' }}>
            <span style={{ fontSize: 22, fontWeight: 700, color: winA ? 'var(--acc)' : 'var(--sec)' }}>{demo.s1}</span>
            <span style={{ color: 'var(--mut)', fontSize: 14 }}>:</span>
            <span style={{ fontSize: 22, fontWeight: 700, color: !winA ? 'var(--acc)' : 'var(--sec)' }}>{demo.s2}</span>
          </div>
        </div>
        <div style={{ textAlign: 'center', fontFamily: 'JetBrains Mono, monospace', fontSize: 11, color: 'var(--sec)' }}>
          <div>{window.fmtDur(demo.dur)}</div>
        </div>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────
// Voice extractor — opens as overlay in the details pane
// ─────────────────────────────────────────────────────────────────────────
function Waveform({ seed, active, percent = 100, color = 'var(--acc)' }) {
  // Deterministic pseudo-waveform from a seed
  let s = seed;
  const bars = [];
  for (let i = 0; i < 36; i++) {
    s = (s * 9301 + 49297) % 233280;
    bars.push(20 + (s / 233280) * 80);
  }
  return (
    <div className={'wave ' + (active ? 'wave-active' : '')}>
      {bars.map((h, i) => (
        <span key={i} style={{
          height: h + '%',
          background: (i / bars.length * 100) < percent ? color : 'var(--surf-4)',
        }}/>
      ))}
    </div>
  );
}

// Deterministic mock SteamID64 from a player name (17 digits, prefixed 7656119...)
function mockSteamId(name) {
  let h = 0;
  for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) >>> 0;
  // Generate 10-digit suffix
  const suffix = String(100000000 + (h % 899999999)).padStart(10, '0');
  return '7656119' + suffix;
}

// Deterministic mock voice events for a player on a demo
function mockVoiceEvents(demo, name) {
  let s = (name.charCodeAt(0) * 17 + (demo?.id?.charCodeAt(1) || 1) * 23) >>> 0;
  const rnd = () => { s = (s * 9301 + 49297) % 233280; return s / 233280; };
  const totalRounds = (demo?.s1 || 0) + (demo?.s2 || 0);
  // 4-10 voice events
  const n = 4 + Math.floor(rnd() * 7);
  const events = [];
  for (let i = 0; i < n; i++) {
    const round = Math.min(totalRounds - 1, Math.floor(rnd() * totalRounds));
    // Demo time: each round ≈ 2 min, plus offset
    const baseSec = round * 115 + Math.floor(rnd() * 100);
    const dur = (0.2 + rnd() * 2.2);
    events.push({
      round,
      demoSec: baseSec,
      dur: dur,
    });
  }
  // sort by demoSec
  events.sort((a, b) => a.demoSec - b.demoSec);
  return events;
}
function fmtClock(sec) {
  const m = Math.floor(sec / 60);
  const ss = sec % 60;
  return String(m).padStart(2, '0') + ':' + String(ss).padStart(2, '0');
}

function ratingColor(rating) {
  const value = parseFloat(rating);
  if (!Number.isFinite(value)) return 'var(--mut)';
  if (value < 1) return '#ef4444';
  if (value < 1.25) return '#f59e0b';
  return '#22c55e';
}

function playerSelectionKey(player) {
  if (!player) return '';
  if (Number.isInteger(player.userId) && player.userId > 0) return `uid:${player.userId}`;
  if (player.steamId) return `steam:${player.steamId}`;
  return player.name ? `name:${player.name}` : '';
}

function buildVoiceCommand(players) {
  let mask = 0;
  let validPlayers = 0;

  for (const player of players) {
    const userId = Number(player?.userId);
    if (!Number.isInteger(userId) || userId < 1 || userId > 20) continue;

    mask |= 1 << (userId - 1);
    validPlayers++;
  }

  if (!validPlayers) return null;
  return `tv_listen_voice_indices ${mask}; tv_listen_voice_indices_h ${mask}`;
}

window.PLAYER_PROFILE_LINKS = {
  steam: 'http://steamcommunity.com/profiles/',
  cswatch: 'https://cswatch.in/player/',
  leetify: 'https://leetify.com/app/profile/',
  csstats: 'https://csstats.gg/player/',
};

function VoiceExtractorPanel({ demo, onBack }) {
  const players = React.useMemo(
    () => [...demo.players1, ...demo.players2].map(p => ({
      ...p,
      steamId: p.steamId || mockSteamId(p.name),
    })),
    [demo.id]
  );

  const [selectedPlayer, setSelectedPlayer] = React.useState(players[0]?.name);
  const [extracting, setExtracting] = React.useState(false);
  const [extractProgress, setExtractProgress] = React.useState(0);
  const [saved, setSaved] = React.useState([]); // { id, player, round, demoSec, dur, format, path }
  const [playing, setPlaying] = React.useState(null); // saved id
  const [playProgress, setPlayProgress] = React.useState({});
  const [format, setFormat] = React.useState('wav');
  const [bitrate, setBitrate] = React.useState('192k');

  const selected = players.find(p => p.name === selectedPlayer);
  // Match by SteamID when present (robust to name sanitization), otherwise by name.
  const events = React.useMemo(
    () => selected
      ? saved.filter(s =>
          (selected.steamId && s.steamId && s.steamId === selected.steamId) ||
          s.player === selected.name)
      : [],
    [saved, selected?.name, selected?.steamId]
  );

  // Reset state when demo changes, then load any clips already on disk for this demo.
  React.useEffect(() => {
    setSaved([]);
    setSelectedPlayer(players[0]?.name);
    setExtracting(false);
    setExtractProgress(0);
    if (window.SDM?.hasHost) {
      window.SDM.call('listVoiceClips', { demoId: demo.id })
        .then(list => setSaved((list || []).map(c => ({ ...c }))))
        .catch(() => {});
    }
  }, [demo.id]);

  // Stream extract-progress events from the host while extraction runs.
  React.useEffect(() => {
    if (!window.SDM) return;
    const off = window.SDM.on('extract-progress', ({ demoId, p }) => {
      if (demoId !== demo.id) return;
      setExtractProgress(Math.max(1, Math.round(p * 100)));
    });
    return off;
  }, [demo.id]);

  // Stop indicator when host reports playback ended.
  React.useEffect(() => {
    if (!window.SDM) return;
    const off = window.SDM.on('playback-ended', () => setPlaying(null));
    return off;
  }, []);

  // Smooth playback progress for the currently-playing clip.
  React.useEffect(() => {
    if (!playing) return;
    const item = saved.find(s => s.id === playing);
    if (!item) { setPlaying(null); return; }
    const tickMs = 60;
    const steps = Math.max(8, Math.floor(item.dur * 1000 / tickMs));
    const inc = 100 / steps;
    const iv = setInterval(() => {
      setPlayProgress(p => {
        const cur = (p[playing] || 0) + inc;
        if (cur >= 100) {
          return { ...p, [playing]: 0 };
        }
        return { ...p, [playing]: cur };
      });
    }, tickMs);
    return () => clearInterval(iv);
  }, [playing, saved]);

  // Drive playback through the host so NAudio plays the actual WAV file.
  React.useEffect(() => {
    if (!window.SDM?.hasHost) return;
    if (!playing) { window.SDM.call('stopClip').catch(() => {}); return; }
    const item = saved.find(s => s.id === playing);
    if (!item) return;
    window.SDM.call('playClip', { path: item.path }).catch(() => setPlaying(null));
  }, [playing]);

  const onExtract = async () => {
    if (extracting || !selected) return;
    if (!window.SDM?.hasHost) { window.__toast?.('Bridge unavailable'); return; }
    setExtracting(true);
    setExtractProgress(1);
    try {
      const res = await window.SDM.call('extractVoice', { demoId: demo.id });
      if (res?.ok) {
        setSaved((res.clips || []).map(c => ({ ...c })));
        const mine = (res.clips || []).filter(c => c.player === selected.name).length;
        window.__toast?.(mine > 0
          ? `Extracted ${mine} clip${mine === 1 ? '' : 's'} for ${selected.name}`
          : 'Extraction finished — no voice for this player');
      } else {
        window.__toast?.('Extraction failed');
      }
    } catch (e) {
      window.__toast?.('Extraction failed: ' + e.message);
    } finally {
      setExtracting(false);
      setExtractProgress(0);
    }
  };

  const teamA = new Set(demo.players1.map(p => p.name));

  return (
    <div className="vx">
      {/* Header strip */}
      <div className="vx-head">
        <button className="btn" onClick={onBack}>
          <IconChevron size={13} style={{ transform: 'rotate(180deg)' }}/> <span>Back to match</span>
        </button>
        <div className="vx-title">
          <div className="vx-eyebrow">
            <IconMic size={11} style={{ verticalAlign: 'middle', marginRight: 4 }}/>
            Voice extractor
          </div>
          <div className="vx-file">{demo.file}</div>
        </div>
        <div className="voice-format">
          <label>
            <span>Format</span>
            <select value={format} onChange={(e) => setFormat(e.target.value)}>
              <option value="wav">WAV</option>
              <option value="mp3">MP3</option>
              <option value="ogg">OGG</option>
            </select>
          </label>
          <label>
            <span>Bitrate</span>
            <select value={bitrate} onChange={(e) => setBitrate(e.target.value)}>
              <option>96k</option>
              <option>128k</option>
              <option>192k</option>
              <option>320k</option>
            </select>
          </label>
        </div>
      </div>

      {/* 3-column workspace */}
      <div className="vx-grid">
        {/* Column 1: Playerlist */}
        <div className="vx-col">
          <div className="vx-col-head">
            <span>Playerlist</span>
            <span className="vx-col-count">{players.length}</span>
          </div>
          <div className="vx-col-body">
            {players.map(p => {
              const isSel = p.name === selectedPlayer;
              const isA = teamA.has(p.name);
              return (
                <button
                  key={p.name}
                  className={'vx-player ' + (isSel ? 'vx-player-sel' : '')}
                  onClick={() => setSelectedPlayer(p.name)}
                >
                  <span className="vx-player-dot" style={{ background: isA ? 'var(--acc)' : 'var(--info)' }}/>
                  <span className="vx-player-text">
                    <span className="vx-player-name">{p.name}</span>
                    <span className="vx-player-id">{p.steamId}</span>
                  </span>
                </button>
              );
            })}
          </div>
          <div className="vx-col-foot">
            <button
              className="btn btn-primary"
              style={{ width: '100%', justifyContent: 'center', height: 36 }}
              disabled={extracting || !selected}
              onClick={onExtract}
            >
              <IconDownload size={13}/>
              <span>{extracting ? 'Extracting…' : 'Extract Voice-Audio'}</span>
            </button>
          </div>
        </div>

        {/* Column 2: Selected Player — rounds */}
        <div className="vx-col">
          <div className="vx-col-head">
            <span>Selected Player</span>
            {selected && <span className="vx-col-count">{events.length}</span>}
          </div>
          <div className="vx-col-body">
            {!selected && (
              <div className="vx-empty">Select a player to see their voice events.</div>
            )}
            {selected && events.length === 0 && (
              <div className="vx-empty">No clips yet for this player. Press <span style={{ color: 'var(--acc)', fontWeight: 600 }}>Extract Voice-Audio</span>.</div>
            )}
            {selected && events.map((ev) => (
              <div key={ev.id} className="vx-event">
                <div className="vx-event-row">
                  <span className="vx-event-round">Round {ev.round}</span>
                  <button
                    className="ic-btn"
                    title="Play this clip"
                    onClick={() => setPlaying(playing === ev.id ? null : ev.id)}
                  >
                    <IconPlay size={11}/>
                  </button>
                </div>
                <div className="vx-event-meta">
                  Demo-Time: <span className="vx-mono">{fmtClock(ev.demoSec)}</span>
                  <span className="vx-event-sep">|</span>
                  Duration: <span className="vx-mono">{ev.dur.toFixed(1)} s</span>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Column 3: Saved files */}
        <div className="vx-col vx-col-wide">
          <div className="vx-col-head">
            <span>Saved Audio-Voice files</span>
            <span className="vx-col-count">{saved.length}</span>
          </div>
          <div className="vx-col-body vx-saved-body">
            {saved.length === 0 && (
              <div className="vx-empty vx-empty-saved">
                <IconWave size={22} style={{ color: 'var(--mut-2)', marginBottom: 8 }}/>
                <div style={{ fontWeight: 600, color: 'var(--sec)', marginBottom: 2 }}>No clips yet</div>
                <div style={{ fontSize: 11, color: 'var(--mut)' }}>
                  Pick a player and press <span style={{ color: 'var(--acc)', fontWeight: 600 }}>Extract Voice-Audio</span>.
                </div>
              </div>
            )}
            {saved.map(s => {
              const isPlaying = playing === s.id;
              const prog = playProgress[s.id] || 0;
              return (
                <div key={s.id} className="vx-saved-row">
                  <button
                    className={'play-btn ' + (isPlaying ? 'play-btn-on' : '')}
                    onClick={() => setPlaying(isPlaying ? null : s.id)}
                  >
                    {isPlaying ? <IconMinus size={11}/> : <IconPlay size={11}/>}
                  </button>
                  <div className="vx-saved-text">
                    <div className="vx-saved-name">
                      <span className="vx-mono">{s.player}</span>
                      <span style={{ color: 'var(--mut)' }}>·</span>
                      <span>Round {s.round}</span>
                      <span style={{ color: 'var(--mut)' }}>·</span>
                      <span className="vx-mono" style={{ color: 'var(--mut)' }}>{fmtClock(s.demoSec)}</span>
                    </div>
                    <div className="vx-saved-wave">
                      <Waveform
                        seed={s.id.length * 31 + s.demoSec}
                        active={isPlaying}
                        percent={isPlaying ? prog : 100}
                        color={isPlaying ? 'var(--acc)' : 'var(--sec)'}
                      />
                    </div>
                  </div>
                  <div className="vx-saved-meta">
                    <span className="vx-mono">{s.dur.toFixed(1)}s</span>
                    <span className="src-tag" style={{
                      background: 'var(--acc-soft)', color: 'var(--acc)'
                    }}>{s.format.toUpperCase()}</span>
                  </div>
                  <button
                    className="ic-btn"
                    title="Reveal in folder"
                    onClick={() => window.SDM?.call('revealInFolder', { path: s.path }).catch(() => {})}
                  >
                    <IconReveal size={12}/>
                  </button>
                </div>
              );
            })}
          </div>
        </div>
      </div>

      {/* Bottom progress bar */}
      <div className={'vx-progress-bar ' + (extracting ? 'vx-progress-on' : '')}>
        <div className="vx-progress-track">
          <div className="vx-progress-fill" style={{ width: extracting ? extractProgress + '%' : '100%' }}/>
        </div>
        <div className="vx-progress-label">
          {extracting
            ? <><span>Decoding player_voice stream…</span><span className="vx-mono">{Math.floor(extractProgress)}%</span></>
            : <><span>Idle · output → <span className="vx-mono" style={{ color: 'var(--sec)' }}>{(demo.folderPath || 'D:\\NewCS2FaceitDemos') + '\\voice\\'}</span></span><span className="vx-mono">{saved.length} file{saved.length === 1 ? '' : 's'}</span></>}
        </div>
      </div>
    </div>
  );
}

function PlayerMenu({ player, onClose, anchor, onAction }) {
  // anchor: { x, y } relative to viewport
  const ref = React.useRef(null);
  React.useEffect(() => {
    const onClick = (e) => { if (ref.current && !ref.current.contains(e.target)) onClose(); };
    const onEsc = (e) => { if (e.key === 'Escape') onClose(); };
    document.addEventListener('mousedown', onClick);
    document.addEventListener('keydown', onEsc);
    return () => { document.removeEventListener('mousedown', onClick); document.removeEventListener('keydown', onEsc); };
  }, [onClose]);
  const name = player?.name || 'Unknown player';
  const hasSteamId = !!player?.steamId;
  const items = [
    { key: 'copySteamId', label: 'Copy SteamID64',          icon: <IconCopy size={13}/>, disabled: !hasSteamId },
    { key: 'openSteam',   label: 'Open Steam profile',      icon: <IconCS size={13}/>, disabled: !hasSteamId },
    { key: 'openCswatch', label: 'Open cswatch.in profile', icon: <IconExternal size={13}/>, disabled: !hasSteamId },
    { key: 'openLeetify', label: 'Open leetify.com profile',icon: <IconExternal size={13}/>, disabled: !hasSteamId },
    { key: 'openCsstats', label: 'Open csstats.gg profile', icon: <IconExternal size={13}/>, disabled: !hasSteamId },
  ];
  return (
    <div ref={ref} className="player-menu" style={{
      position: 'fixed', left: anchor.x, top: anchor.y, zIndex: 1000,
      minWidth: 240, whiteSpace: 'nowrap',
    }}>
      <div style={{ padding: '8px 10px', borderBottom: '1px solid var(--border)', color: 'var(--mut)', fontSize: 10.5, fontFamily: 'JetBrains Mono, monospace', fontWeight: 600 }}>
        {name}
      </div>
      <div style={{ padding: 4 }}>
        {items.map((it, i) => (
          <button key={i} className="menu-item" disabled={it.disabled} onClick={() => { onClose(); onAction?.(it.key, player); }}>
            <span style={{ color: 'var(--mut)' }}>{it.icon}</span>
            <span>{it.label}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

function TeamPanel({ team, label, score, players, mirror, selected, onToggle, onPlayerClick, accentTeam }) {
  const isSelected = (player) => selected.has(playerSelectionKey(player));
  const allSel = players.length > 0 && players.every(isSelected);
  const someSel = !allSel && players.some(isSelected);
  const toggleAll = () => {
    const targets = allSel ? players.filter(isSelected) : players.filter(p => !isSelected(p));
    targets.forEach(onToggle);
  };
  return (
    <div className="team-panel">
      <div className="team-head" style={{ flexDirection: mirror ? 'row-reverse' : 'row' }}>
        <div style={{ textAlign: mirror ? 'right' : 'left', minWidth: 0, flex: 1 }}>
          <div style={{ fontSize: 9.5, color: 'var(--mut)', textTransform: 'uppercase', letterSpacing: '.1em', fontWeight: 700 }}>{label}</div>
          <div style={{ fontSize: 13, color: 'var(--fg)', fontWeight: 600, marginTop: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{team}</div>
        </div>
        <div style={{
          fontFamily: 'JetBrains Mono, monospace', fontSize: 22, fontWeight: 700,
          color: accentTeam ? 'var(--acc)' : 'var(--sec)',
          minWidth: 36, textAlign: 'center',
        }}>{score}</div>
      </div>

      <div className="team-rows">
        {players.map((p, i) => {
          const isSel = isSelected(p);
          const rowKey = playerSelectionKey(p) || p.steamId || p.name || i;
          return (
            <div key={rowKey} className={'team-row ' + (isSel ? 'team-row-sel' : '')}
                 style={{ flexDirection: mirror ? 'row-reverse' : 'row' }}
                 onContextMenu={(e) => { e.preventDefault(); onPlayerClick(p, e.clientX, e.clientY); }}
                 onClick={(e) => { if (e.target.tagName !== 'INPUT') onToggle(p); }}>
              <label className="cb-wrap" onClick={(e) => e.stopPropagation()}>
                <input type="checkbox" checked={isSel} onChange={() => onToggle(p)}/>
                <span className="cb-box">{isSel && <IconCheck size={10}/>}</span>
              </label>
              <div className="pname" style={{ textAlign: mirror ? 'right' : 'left' }}>
                <div className="pname-row" style={{ flexDirection: mirror ? 'row-reverse' : 'row' }}>
                  {p.flagged && <IconFlag size={10} style={{ color: '#ef4444', flexShrink: 0 }}/>}
                  <span className="pname-text">{p.name}</span>
                  {p.favorite && <span style={{ color: 'var(--acc)', flexShrink: 0 }}>★</span>}
                </div>
                <div className="pname-stats" style={{ flexDirection: mirror ? 'row-reverse' : 'row' }}>
                  <span><span style={{ color: 'var(--sec)' }}>{p.k}</span><span style={{ color: 'var(--mut-2)' }}>/{p.d}</span></span>
                  <span style={{ color: 'var(--mut-2)' }}>·</span>
                  <span style={{ color: ratingColor(p.rating), fontWeight: 600 }}>{p.rating} rating</span>
                </div>
              </div>
              <button className="more-btn" onClick={(e) => { e.stopPropagation(); onPlayerClick(p, e.clientX, e.clientY); }}>
                <IconChevronD size={11}/>
              </button>
            </div>
          );
        })}
      </div>

      <div className="team-foot" onClick={toggleAll} style={{ flexDirection: mirror ? 'row-reverse' : 'row' }}>
        <label className="cb-wrap" onClick={(e) => e.stopPropagation()}>
          <input type="checkbox" checked={allSel}
                 ref={el => el && (el.indeterminate = someSel)}
                 onChange={toggleAll}/>
          <span className={'cb-box ' + (someSel ? 'cb-box-some' : '')}>
            {allSel && <IconCheck size={10}/>}
            {someSel && <IconMinus size={10}/>}
          </span>
        </label>
        <span style={{ fontSize: 11, color: 'var(--sec)', fontWeight: 500 }}>Select all</span>
        <span style={{ flex: 1 }}/>
        <span style={{ fontSize: 10, color: 'var(--mut)', fontFamily: 'JetBrains Mono, monospace' }}>
          {players.filter(isSelected).length}/{players.length}
        </span>
      </div>
    </div>
  );
}

function MatchView({ demo, selected, onToggle, onPlayerClick, onOpenVoice }) {
  return (
    <div className="match-grid">
      <TeamPanel
        team={demo.t1} label="Team A" score={demo.s1}
        players={demo.players1} mirror={false}
        selected={selected} onToggle={onToggle}
        onPlayerClick={onPlayerClick}
        accentTeam={demo.s1 > demo.s2}
      />
      <MatchCenter demo={demo} onOpenVoice={onOpenVoice}/>
      <TeamPanel
        team={demo.t2} label="Team B" score={demo.s2}
        players={demo.players2} mirror={true}
        selected={selected} onToggle={onToggle}
        onPlayerClick={onPlayerClick}
        accentTeam={demo.s2 > demo.s1}
      />
    </div>
  );
}

function CopyCmdBar({ selectedPlayers, onCopy, onClear, onToggle }) {
  const count = selectedPlayers.length;
  return (
    <div className="copycmd">
      <div className="copycmd-input">
        <IconUsers size={14} style={{ color: count ? 'var(--acc)' : 'var(--mut)', flexShrink: 0 }}/>
        {count === 0 ? (
          <span style={{ fontSize: 12, color: 'var(--mut)' }}>Select one or more players you would like to hear in the demo…</span>
        ) : (
          <>
            <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap', flex: 1, minWidth: 0 }}>
              {selectedPlayers.slice(0, 6).map(player => (
                <span key={playerSelectionKey(player)} className="player-pill">
                  {player.name}
                  <button onClick={() => onToggle(player)} className="chip-x"><IconClose size={9}/></button>
                </span>
              ))}
              {count > 6 && <span className="player-pill player-pill-more">+{count - 6}</span>}
            </div>
            <button className="link-btn" onClick={onClear}>Clear</button>
          </>
        )}
      </div>
      <button className="btn btn-primary btn-tall" disabled={count === 0} onClick={onCopy}>
        <IconCopy size={13}/> <span>Copy Cmd.</span>
      </button>
    </div>
  );
}

function RoundTimeline({ demo }) {
  const rounds = demo.rounds;
  const half = Math.min(12, rounds.length);
  // Match the accent rule used by the scoreboard: the winning team is highlighted in
  // accent (orange). Whichever letter belongs to the winner gets the accent color in
  // the timeline so the two views stay consistent.
  const winnerLetter = demo.s1 === demo.s2 ? null : (demo.s1 > demo.s2 ? 'A' : 'B');
  return (
    <div className="section-card">
      <div className="section-head">
        <span>Round timeline</span>
        <span style={{ color: 'var(--mut)' }}>{rounds.length} rounds</span>
      </div>
      <div style={{ display: 'flex', gap: 2, alignItems: 'stretch', marginTop: 8 }}>
        {rounds.map((r, i) => {
          const isHalf = i === half - 1;
          const isWinner = winnerLetter ? r === winnerLetter : r === 'A';
          return (
            <div key={i} style={{
              flex: 1, height: 22, borderRadius: 2,
              background: isWinner ? 'var(--acc-soft)' : 'rgba(120,140,170,.16)',
              borderTop: '2px solid ' + (isWinner ? 'var(--acc)' : '#8a96a8'),
              marginRight: isHalf ? 6 : 0,
            }}/>
          );
        })}
      </div>
    </div>
  );
}

function NotesTags({ demo, onTagToggle, onNoteChange }) {
  const [note, setNote] = React.useState(demo.note || '');
  const [newTag, setNewTag] = React.useState('');
  React.useEffect(() => setNote(demo.note || ''), [demo.id]);
  return (
    <div className="section-card">
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 14 }}>
        <div>
          <div className="section-head"><IconTag size={11} style={{ marginRight: 4 }}/>Tags</div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 5, marginTop: 8 }}>
            {demo.tags.map((t, i) => (
              <span key={i} className="chip chip-tag">
                {t}
                <button className="chip-x" onClick={() => onTagToggle(demo.id, t)}><IconClose size={10}/></button>
              </span>
            ))}
            <form onSubmit={(e) => { e.preventDefault(); if (newTag.trim()) { onTagToggle(demo.id, newTag.trim(), true); setNewTag(''); } }}>
              <input className="tag-input" placeholder="+ add tag" value={newTag} onChange={(e) => setNewTag(e.target.value)}/>
            </form>
          </div>
        </div>
        <div>
          <div className="section-head"><IconNote size={11} style={{ marginRight: 4 }}/>Notes</div>
          <textarea
            className="notes-area"
            placeholder="Clutches, suspicious moments, tactics…"
            value={note}
            onChange={(e) => setNote(e.target.value)}
            onBlur={() => onNoteChange(demo.id, note)}
          />
        </div>
      </div>
    </div>
  );
}

function ActionBar({ demo, onAction, onFavorite }) {
  return (
    <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
      <button className={'btn ' + (demo.fav ? 'btn-fav-on' : '')} onClick={() => onFavorite(demo.id)}>
        <IconFavorite size={13} fill={demo.fav}/>
        <span>{demo.fav ? 'Favorited' : 'Favorite'}</span>
      </button>
      <button className="btn" onClick={() => onAction('reveal')}>
        <IconReveal size={13}/> <span>Reveal in folder</span>
      </button>
      <button className="btn" onClick={() => onAction('rename')}>
        <IconRename size={13}/> <span>Rename</span>
      </button>
      <button className="btn btn-danger" onClick={() => onAction('delete')}>
        <IconTrash size={13}/>
      </button>
    </div>
  );
}

function DetailsPanel({ demo, onAction, onTagToggle, onNoteChange, onFavorite, onPlayerAction, voiceOpen, onVoiceOpen }) {
  const [selectedPlayers, setSelectedPlayers] = React.useState(new Set());
  const [menu, setMenu] = React.useState(null); // { player, x, y }

  // Reset selection when demo changes
  React.useEffect(() => { setSelectedPlayers(new Set()); onVoiceOpen?.(false); }, [demo?.id]);

  if (!demo) {
    return (
      <div style={{ padding: 40, textAlign: 'center', color: 'var(--mut)', fontSize: 13 }}>
        Select a demo to see details
      </div>
    );
  }

  if (voiceOpen) {
    return (
      <div className="details-scroll">
        <VoiceExtractorPanel demo={demo} onBack={() => onVoiceOpen?.(false)}/>
      </div>
    );
  }

  const allPlayers = [...demo.players1, ...demo.players2];
  const selectedPlayerRows = allPlayers.filter(p => selectedPlayers.has(playerSelectionKey(p)));

  const onToggle = (player) => {
    const key = playerSelectionKey(player);
    if (!key) return;
    setSelectedPlayers(s => {
      const n = new Set(s);
      if (n.has(key)) n.delete(key); else n.add(key);
      return n;
    });
  };
  const onPlayerClick = (p, x, y) => setMenu({ player: p, x, y });
  const onCopy = async () => {
    const cmd = buildVoiceCommand(selectedPlayerRows);
    if (!cmd) {
      window.__toast?.('No valid players selected');
      return;
    }
    const ok = await window.SDM?.call('copyToClipboard', { text: cmd }).catch(() => false);
    window.__toast?.(ok ? `Copied: ${cmd}` : 'Copy failed');
  };
  const onClear = () => setSelectedPlayers(new Set());

  return (
    <div className="details-scroll">
      <FilePathStrip demo={demo} onMoveToCS2={() => onAction('move-to-cs2')}/>
      <MatchView
        demo={demo}
        selected={selectedPlayers}
        onToggle={onToggle}
        onPlayerClick={onPlayerClick}
        onOpenVoice={() => onVoiceOpen?.(true)}
      />
      <CopyCmdBar selectedPlayers={selectedPlayerRows} onCopy={onCopy} onClear={onClear} onToggle={onToggle}/>
      <RoundTimeline demo={demo}/>
      <NotesTags demo={demo} onTagToggle={onTagToggle} onNoteChange={onNoteChange}/>
      <ActionBar demo={demo} onAction={onAction} onFavorite={onFavorite}/>

      {menu && (
        <PlayerMenu
          player={menu.player}
          anchor={{ x: menu.x, y: menu.y }}
          onClose={() => setMenu(null)}
          onAction={(action, player) => onPlayerAction?.(action, player, demo)}
        />
      )}
    </div>
  );
}

window.DetailsPanel = DetailsPanel;
