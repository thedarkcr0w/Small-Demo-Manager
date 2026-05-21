window.__toast = (msg) => {
  const old = document.querySelector('.toast');
  if (old) old.remove();
  const el = document.createElement('div');
  el.className = 'toast';
  el.textContent = msg;
  document.body.appendChild(el);
  setTimeout(() => el.remove(), 3000);
};

// Custom window-chrome drag/maximize for borderless host form.
document.addEventListener('mousedown', (e) => {
  const inTitle = e.target.closest('.win-title');
  if (!inTitle) return;
  if (e.target.closest('.win-btn') || e.target.closest('button')) return;
  if (e.button !== 0) return;
  window.SDM?.call('windowDrag').catch(() => {});
});

(async function boot() {
  try {
    window.__bootStatus('Waiting for bridge…');
    await window.SDM.ready;
    let state = { folders: [], demos: [], startupDemo: null };
    if (window.SDM.hasHost) {
      window.__bootStatus('Loading library…');
      try { state = await window.SDM.call('getInitialState'); }
      catch (e) { window.__fatal('getInitialState failed: ' + (e.stack || e.message || e)); return; }
    }
    window.SDM_HYDRATE(state);
    window.__bootStatus('Rendering…');
    const rootEl = document.getElementById('root');
    rootEl.innerHTML = '';
    const root = ReactDOM.createRoot(rootEl);
    root.render(React.createElement(window.App));
  } catch (e) {
    window.__fatal('boot crashed: ' + (e.stack || e.message || e));
  }
})();
