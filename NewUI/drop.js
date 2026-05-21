// JS-side handler for .dem files dragged onto the WebView.
// HTML5's File API doesn't expose the OS path — Chromium hides it for security —
// so we stream the file's bytes (as base64) back to C#, which writes them to a
// staging dir under %LocalAppData% and then routes them through the existing
// `files-dropped` import flow.
(function () {
  if (!window.SDM || !window.SDM.hasHost) return;

  let dragDepth = 0;
  const emitOverlay = (active, fromArchive = false) => {
    // Reuse the React listener already wired for the Form-level drop overlay.
    const handlers = window.__sdmDragOverlayListeners;
    if (handlers) handlers.forEach(h => h({ active, fromArchive }));
  };

  const hasFiles = (dt) =>
    !!(dt && dt.types && Array.prototype.indexOf.call(dt.types, 'Files') >= 0);

  document.addEventListener('dragenter', (e) => {
    if (!hasFiles(e.dataTransfer)) return;
    e.preventDefault();
    dragDepth++;
    if (dragDepth === 1) {
      window.SDM?.call('dragActive', { active: true }).catch(() => {});
      emitOverlay(true);
    }
  }, true);

  document.addEventListener('dragover', (e) => {
    if (!hasFiles(e.dataTransfer)) return;
    e.preventDefault();
    try { e.dataTransfer.dropEffect = 'copy'; } catch (_) {}
  }, true);

  document.addEventListener('dragleave', (e) => {
    if (!hasFiles(e.dataTransfer)) return;
    dragDepth = Math.max(0, dragDepth - 1);
    if (dragDepth === 0) {
      window.SDM?.call('dragActive', { active: false }).catch(() => {});
      emitOverlay(false);
    }
  }, true);

  document.addEventListener('drop', async (e) => {
    if (!hasFiles(e.dataTransfer)) return;
    e.preventDefault();
    dragDepth = 0;
    window.SDM?.call('dragActive', { active: false }).catch(() => {});
    emitOverlay(false);

    const files = Array.from(e.dataTransfer.files || [])
      .filter(f => f && f.name && f.name.toLowerCase().endsWith('.dem'));
    if (files.length === 0) {
      window.__toast?.('Drop a .dem file to import it');
      return;
    }

    try {
      // Tell the host to show the "extracting"/import overlay while we ferry bytes.
      window.SDM?.call('dragActive', { active: true }).catch(() => {});
      const items = [];
      for (let i = 0; i < files.length; i++) {
        const f = files[i];
        const dataBase64 = await readAsBase64(f);
        items.push({ name: f.name, dataBase64 });
      }
      await window.SDM.call('importDroppedBytes', { items });
    } catch (err) {
      console.error('[drop] importDroppedBytes failed', err);
      window.__toast?.('Import failed: ' + (err.message || err));
    } finally {
      window.SDM?.call('dragActive', { active: false }).catch(() => {});
    }
  }, true);

  // Read an entire File as a base64 string. Chunks the ArrayBuffer so
  // String.fromCharCode doesn't blow the call stack on large demos.
  async function readAsBase64(file) {
    const buf = await file.arrayBuffer();
    const bytes = new Uint8Array(buf);
    const CHUNK = 0x8000;
    let bin = '';
    for (let i = 0; i < bytes.length; i += CHUNK) {
      bin += String.fromCharCode.apply(null, bytes.subarray(i, i + CHUNK));
    }
    return btoa(bin);
  }
})();
