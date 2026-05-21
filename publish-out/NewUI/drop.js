// JS-side handler for .dem files dragged onto the WebView.
// HTML5's File API doesn't expose the OS path — Chromium hides it for security —
// so we stream the file in 4 MB chunks to C#, which writes them straight to a
// staging dir under %LocalAppData% and routes through the existing import flow.
// Streaming (vs. arrayBuffer+base64 in one shot) keeps memory flat for any
// file size and gives us granular error reporting for unreadable sources
// (e.g. some archive viewers' virtual files).
(function () {
  if (!window.SDM || !window.SDM.hasHost) return;

  const CHUNK_SIZE = 4 * 1024 * 1024; // 4 MB

  let dragDepth = 0;
  const hasFiles = (dt) =>
    !!(dt && dt.types && Array.prototype.indexOf.call(dt.types, 'Files') >= 0);

  document.addEventListener('dragenter', (e) => {
    if (!hasFiles(e.dataTransfer)) return;
    e.preventDefault();
    dragDepth++;
    if (dragDepth === 1) window.SDM?.call('dragActive', { active: true }).catch(() => {});
  }, true);

  document.addEventListener('dragover', (e) => {
    if (!hasFiles(e.dataTransfer)) return;
    e.preventDefault();
    try { e.dataTransfer.dropEffect = 'copy'; } catch (_) {}
  }, true);

  document.addEventListener('dragleave', (e) => {
    if (!hasFiles(e.dataTransfer)) return;
    dragDepth = Math.max(0, dragDepth - 1);
    if (dragDepth === 0) window.SDM?.call('dragActive', { active: false }).catch(() => {});
  }, true);

  document.addEventListener('drop', async (e) => {
    if (!hasFiles(e.dataTransfer)) return;
    e.preventDefault();
    dragDepth = 0;
    window.SDM?.call('dragActive', { active: false }).catch(() => {});

    const dt = e.dataTransfer;
    const files = collectFilesFromDrop(dt);

    if (files.length === 0) {
      // Tell C# what was actually in the DataTransfer so we can diagnose
      // archive viewer drops where Chromium exposes a File but with a name
      // that doesn't end in .dem (e.g. WinRAR sometimes passes the archive
      // basename or an empty string for virtual entries).
      const fileDump = [];
      if (dt.files) for (const f of dt.files) fileDump.push({ src: 'files', name: f.name, size: f.size, type: f.type });
      if (dt.items) for (const it of dt.items) {
        if (it.kind === 'file') {
          const f = it.getAsFile && it.getAsFile();
          if (f) fileDump.push({ src: 'items', name: f.name, size: f.size, type: f.type });
        }
      }
      const diag = {
        types: Array.from(dt.types || []),
        items: dt.items ? Array.from(dt.items).map(i => ({ kind: i.kind, type: i.type })) : [],
        files: fileDump,
      };
      window.SDM?.call('dropDebug', diag).catch(() => {});
      window.__toast?.('No .dem file in this drop. If dragging from an archive viewer, extract the file first.');
      return;
    }

    const importedPaths = [];
    for (const file of files) {
      try {
        const path = await streamFile(file);
        if (path) importedPaths.push(path);
      } catch (err) {
        const msg = describeReadError(err, file);
        console.error('[drop] failed for', file.name, err);
        window.__toast?.(`${file.name}: ${msg}`);
        // Best-effort: tell C# to discard any in-progress session.
        window.SDM?.call('importChunkAbort', { name: file.name }).catch(() => {});
      }
    }

    if (importedPaths.length > 0) {
      window.SDM?.call('importChunkFinalize', { paths: importedPaths }).catch(() => {});
    }
  }, true);

  // Gather any .dem files the OS surfaced for this drop. Prefer dataTransfer.files
  // (the standard collection); fall back to dataTransfer.items (some archive
  // viewers populate items but leave files empty).
  function collectFilesFromDrop(dt) {
    const out = [];
    const seen = new Set();
    const consider = (f) => {
      if (!f || !f.name || !f.name.toLowerCase().endsWith('.dem')) return;
      const key = f.name + '|' + (f.size || 0);
      if (seen.has(key)) return;
      seen.add(key);
      out.push(f);
    };
    if (dt.files && dt.files.length) {
      for (const f of dt.files) consider(f);
    }
    if (dt.items && dt.items.length) {
      for (const item of dt.items) {
        if (item.kind !== 'file') continue;
        const f = item.getAsFile && item.getAsFile();
        consider(f);
      }
    }
    return out;
  }

  // Stream one file to C# in chunks. Returns the staging path on success.
  async function streamFile(file) {
    const sessionId = (window.crypto && window.crypto.randomUUID)
      ? window.crypto.randomUUID()
      : (Date.now().toString(36) + '-' + Math.random().toString(36).slice(2));

    // Open the session — C# creates the staging file and remembers the path.
    const begin = await window.SDM.call('importChunkBegin', {
      sessionId, name: file.name, size: file.size,
    });
    if (!begin || !begin.ok) throw new Error(begin?.error || 'failed to open staging file');

    const total = file.size;
    let sent = 0;
    while (sent < total) {
      const end = Math.min(sent + CHUNK_SIZE, total);
      // file.slice + arrayBuffer materializes only the slice — for virtual
      // archive files this is what surfaces NotReadableError if the source
      // can't be read.
      const blob = file.slice(sent, end);
      const buf = await blob.arrayBuffer();
      const b64 = bytesToBase64(new Uint8Array(buf));
      const res = await window.SDM.call('importChunkData', { sessionId, dataBase64: b64 });
      if (!res || !res.ok) throw new Error(res?.error || 'chunk write failed');
      sent = end;
    }

    const close = await window.SDM.call('importChunkEnd', { sessionId });
    if (!close || !close.ok) throw new Error(close?.error || 'failed to finalize staging file');
    return close.path || null;
  }

  function bytesToBase64(bytes) {
    let bin = '';
    const SUB = 0x8000;
    for (let i = 0; i < bytes.length; i += SUB) {
      bin += String.fromCharCode.apply(null, bytes.subarray(i, i + SUB));
    }
    return btoa(bin);
  }

  function describeReadError(err, file) {
    const name = err && err.name;
    if (name === 'NotReadableError' || /permission|denied/i.test(String(err))) {
      return 'unable to read the file from its source — extract the archive first, then drag the .dem.';
    }
    if (name === 'NotFoundError') return 'source no longer available — try the drop again.';
    if (name === 'SecurityError') return 'the source blocked access — extract the archive first.';
    if (file && file.size === 0)   return 'the file appears to be empty (some archive viewers can\'t stream virtual files).';
    return err && err.message ? err.message : 'import failed.';
  }
})();
