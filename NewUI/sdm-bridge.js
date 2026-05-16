// JS-side of the C# <-> JS bridge.
// Exposes:
//   window.SDM.call(type, payload?) -> Promise<result>
//   window.SDM.on(event, handler)   -> unsubscribe()
//   window.SDM.ready                -> Promise that resolves once host is connected
(function () {
  const hasHost = !!(window.chrome && window.chrome.webview);
  const pending = new Map();
  const handlers = new Map();
  let nextId = 1;
  let readyResolve;
  const ready = new Promise((r) => { readyResolve = r; });

  function dispatch(msg) {
    console.log('[SDM] ←', msg);
    if (msg && typeof msg.id === 'number') {
      const p = pending.get(msg.id);
      if (p) {
        pending.delete(msg.id);
        if (msg.ok) p.resolve(msg.result);
        else p.reject(new Error(msg.error || 'Bridge call failed'));
      }
      return;
    }
    if (msg && msg.event) {
      const list = handlers.get(msg.event);
      if (list) list.forEach(h => { try { h(msg.payload); } catch (e) { console.error(e); } });
    }
  }

  if (hasHost) {
    window.chrome.webview.addEventListener('message', (e) => {
      console.log('[SDM] raw message event, typeof data =', typeof e.data);
      let msg;
      try { msg = typeof e.data === 'string' ? JSON.parse(e.data) : e.data; }
      catch (err) { console.error('[SDM] failed to parse incoming message', err, e.data); return; }
      dispatch(msg);
    });
    console.log('[SDM] bridge ready, host detected');
    readyResolve();
  } else {
    console.warn('[SDM] No WebView2 host detected — bridge is inert.');
    readyResolve();
  }

  // Long-running operations (parse a whole demo, extract all voice tracks)
  // legitimately take many seconds. Give them a generous ceiling so progress
  // events have time to play out.
  const LONG_OPS = new Set(['parseDemo', 'extractVoice', 'scanAll', 'scanFolder', 'applyUpdate']);
  const DEFAULT_TIMEOUT_MS = 8000;
  const LONG_TIMEOUT_MS = 10 * 60 * 1000;

  function call(type, payload, opts) {
    if (!hasHost) return Promise.reject(new Error('Bridge not available (host missing)'));
    const id = nextId++;
    const timeoutMs = (opts && typeof opts.timeoutMs === 'number')
      ? opts.timeoutMs
      : (LONG_OPS.has(type) ? LONG_TIMEOUT_MS : DEFAULT_TIMEOUT_MS);
    console.log('[SDM] →', id, type, payload, '(timeout=' + timeoutMs + 'ms)');
    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
      const timer = setTimeout(() => {
        if (pending.has(id)) {
          pending.delete(id);
          console.error('[SDM] TIMEOUT after', timeoutMs, 'ms waiting for response to', type, '(id=' + id + ')');
          reject(new Error('Bridge timeout: ' + type));
        }
      }, timeoutMs);
      try {
        window.chrome.webview.postMessage(JSON.stringify({ id, type, payload: payload ?? {} }));
      } catch (e) {
        clearTimeout(timer);
        pending.delete(id);
        console.error('[SDM] postMessage threw', e);
        reject(e);
      }
    });
  }

  function on(event, handler) {
    if (!handlers.has(event)) handlers.set(event, new Set());
    handlers.get(event).add(handler);
    return () => handlers.get(event).delete(handler);
  }

  window.SDM = { ready, call, on, hasHost };
})();
