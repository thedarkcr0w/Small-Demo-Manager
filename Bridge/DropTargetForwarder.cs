using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using SmallDemoManager.GUI;
using WinIDataObject = System.Windows.Forms.IDataObject;

namespace SmallDemoManager.Bridge
{
    /// <summary>
    /// COM <see cref="IDropTarget"/> we register on WebView2's child HWND so we
    /// can read OLE drag/drop data ourselves — including archive-virtual files
    /// (<c>CFSTR_FILEDESCRIPTORW</c> / <c>CFSTR_FILECONTENTS</c>) that 7-Zip
    /// and WinRAR expose. The standard HTML5 drag API can't materialize those.
    /// </summary>
    [ComVisible(true)]
    [ClassInterface(ClassInterfaceType.None)]
    internal sealed class WebViewDropForwarder : IDropTargetCom
    {
        private readonly MainForm _form;

        public WebViewDropForwarder(MainForm form) { _form = form; }

        public int DragEnter(System.Runtime.InteropServices.ComTypes.IDataObject pDataObj, int grfKeyState, POINT pt, ref int pdwEffect)
        {
            try
            {
                // Always dump the IDataObject's real FORMATETC list so we can see what
                // each drag-source actually provides (and with which tymed/lindex).
                ShellDropResolver.DumpFormats(pDataObj, "DragEnter");

                // Read CF_HDROP straight off the COM IDataObject. We accept the drop
                // if any path looks promising — by .dem extension OR by demo-magic
                // content sniff (7-Zip extracts to temp files like "7zECA13167A" with
                // no extension; we still want to import those).
                bool hasReal = ShellDropResolver.ReadHDropFiles(pDataObj)
                    .Any(LooksLikeDemoCandidate);

                var data = new DataObject(pDataObj);
                bool has = hasReal || MainForm.HasAnyDemoSource(data);
                pdwEffect = has ? DROPEFFECT_COPY : DROPEFFECT_NONE;
                if (has) _form.HandleDragEnter(data);
            }
            catch (Exception ex)
            {
                BridgeService.DebugLog("DropTarget.DragEnter threw: " + ex.Message);
                pdwEffect = DROPEFFECT_NONE;
            }
            return S_OK;
        }

        public int DragOver(int grfKeyState, POINT pt, ref int pdwEffect)
        {
            // pdwEffect on entry is the bitmask of effects the source allows
            // (e.g. COPY|MOVE = 0x3 for Explorer / 7-Zip). On exit it must be a
            // SINGLE effect or NONE. Treat any bitmask that includes COPY as a
            // copy operation — otherwise the OS sees DROPEFFECT_NONE at release
            // time and fires DragLeave instead of Drop.
            pdwEffect = (pdwEffect & DROPEFFECT_COPY) != 0 ? DROPEFFECT_COPY : DROPEFFECT_NONE;
            return S_OK;
        }

        public int DragLeave()
        {
            try { _form.HandleDragLeave(); }
            catch (Exception ex) { BridgeService.DebugLog("DropTarget.DragLeave threw: " + ex.Message); }
            return S_OK;
        }

        public int Drop(System.Runtime.InteropServices.ComTypes.IDataObject pDataObj, int grfKeyState, POINT pt, ref int pdwEffect)
        {
            try
            {
                // Same CF_HDROP shortcut as DragEnter — bypass WinForms' wrapper and
                // hand the real on-disk paths straight to the bridge.
                var hdropPaths = ShellDropResolver.ReadHDropFiles(pDataObj);

                var realDems = hdropPaths
                    .Where(p => !string.IsNullOrWhiteSpace(p) &&
                                p.EndsWith(".dem", StringComparison.OrdinalIgnoreCase) &&
                                File.Exists(p))
                    .ToArray();
                if (realDems.Length > 0)
                {
                    // Archive viewers (7-Zip / WinRAR) delete the extracted temp file
                    // the instant Drop returns. Copy each .dem into our drop-staging
                    // dir synchronously here so the bridge has something to import
                    // when React posts importDemoFiles a few ms later.
                    var staged = StageFiles(realDems);
                    if (staged.Length > 0)
                    {
                        BridgeService.DebugLog($"Drop: CF_HDROP staged {staged.Length} .dem path(s)");
                        _form.EmitFilesDropped(staged, staged: true);
                        pdwEffect = DROPEFFECT_COPY;
                        return S_OK;
                    }
                }

                // Extension-less or oddly-named temp files (7-Zip / WinRAR shell-drop).
                // Sniff the first 8 bytes for the CS2 / CSGO demo magic and, if it
                // matches, copy the file into our drop-staging dir with a real .dem
                // name so the rest of the import pipeline can pick it up.
                var sniffed = new List<string>();
                foreach (var p in hdropPaths)
                {
                    if (string.IsNullOrWhiteSpace(p) || !File.Exists(p)) continue;
                    if (!IsLikelyDemo(p)) continue;
                    try
                    {
                        var staging = SmallDemoManager.UtilClass.LocalAppDataFolder
                            .EnsureSubDirectoryExists("drop-staging");
                        var leaf = Path.GetFileNameWithoutExtension(p);
                        if (string.IsNullOrWhiteSpace(leaf)) leaf = "imported";
                        var safeStem = string.Concat(leaf.Where(c => !Path.GetInvalidFileNameChars().Contains(c)));
                        var dest = Path.Combine(staging, $"{safeStem}.dem");
                        for (int i = 1; File.Exists(dest) && i < 1000; i++)
                            dest = Path.Combine(staging, $"{safeStem} ({i}).dem");
                        File.Copy(p, dest, overwrite: false);
                        sniffed.Add(dest);
                    }
                    catch (Exception ex)
                    {
                        BridgeService.DebugLog($"Drop sniff copy failed for {p}: {ex.Message}");
                    }
                }
                if (sniffed.Count > 0)
                {
                    BridgeService.DebugLog($"Drop: sniffed {sniffed.Count} demo(s) from extension-less paths");
                    _form.EmitFilesDropped(sniffed.ToArray(), staged: true);
                    pdwEffect = DROPEFFECT_COPY;
                    return S_OK;
                }

                var data = new DataObject(pDataObj);
                _form.HandleDragDrop(data);
                pdwEffect = DROPEFFECT_COPY;
            }
            catch (Exception ex)
            {
                BridgeService.DebugLog("DropTarget.Drop threw: " + ex);
                pdwEffect = DROPEFFECT_NONE;
            }
            return S_OK;
        }

        private const int S_OK = 0;
        private const int DROPEFFECT_NONE = 0;
        private const int DROPEFFECT_COPY = 1;

        private static bool LooksLikeDemoCandidate(string? path)
        {
            // Be permissive at DragEnter: 7-Zip / WinRAR only materialize the temp
            // file on actual Drop, so we can't reliably content-sniff yet. As long as
            // the source advertises any CF_HDROP path with a non-empty string, accept
            // the drop and validate at Drop time.
            return !string.IsNullOrWhiteSpace(path);
        }

        // Copy each path into drop-staging so we own a file that won't be deleted
        // by the drag source after Drop returns. Uses the source's basename so the
        // user still recognises the demo in the library.
        private static string[] StageFiles(IEnumerable<string> sources)
        {
            var staging = SmallDemoManager.UtilClass.LocalAppDataFolder
                .EnsureSubDirectoryExists("drop-staging");
            var staged = new List<string>();
            foreach (var src in sources)
            {
                if (string.IsNullOrWhiteSpace(src) || !File.Exists(src)) continue;
                try
                {
                    var leaf = Path.GetFileName(src);
                    var safe = string.Concat(leaf.Where(c => !Path.GetInvalidFileNameChars().Contains(c)));
                    if (!safe.EndsWith(".dem", StringComparison.OrdinalIgnoreCase)) safe += ".dem";
                    var dest = Path.Combine(staging, safe);
                    var stem = Path.GetFileNameWithoutExtension(safe);
                    var ext = Path.GetExtension(safe);
                    for (int i = 1; File.Exists(dest) && i < 1000; i++)
                        dest = Path.Combine(staging, $"{stem} ({i}){ext}");
                    File.Copy(src, dest, overwrite: false);
                    staged.Add(dest);
                }
                catch (Exception ex)
                {
                    BridgeService.DebugLog($"StageFiles: failed for {src}: {ex.Message}");
                }
            }
            return staged.ToArray();
        }

        // CS2 demos begin with the magic string "PBDEMS2\0"; older Source 1 / CSGO
        // demos use "HL2DEMO\0". An 8-byte read is enough to distinguish either from
        // a random binary blob.
        private static bool IsLikelyDemo(string path)
        {
            try
            {
                using var fs = new FileStream(path, FileMode.Open, FileAccess.Read,
                    FileShare.ReadWrite | FileShare.Delete);
                Span<byte> buf = stackalloc byte[8];
                if (fs.Read(buf) < 8) return false;
                // "PBDEMS2" or "HL2DEMO" (both 7 ASCII chars + NUL).
                return (buf[0] == (byte)'P' && buf[1] == (byte)'B' && buf[2] == (byte)'D'
                        && buf[3] == (byte)'E' && buf[4] == (byte)'M' && buf[5] == (byte)'S'
                        && buf[6] == (byte)'2')
                    || (buf[0] == (byte)'H' && buf[1] == (byte)'L' && buf[2] == (byte)'2'
                        && buf[3] == (byte)'D' && buf[4] == (byte)'E' && buf[5] == (byte)'M'
                        && buf[6] == (byte)'O');
            }
            catch { return false; }
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    internal struct POINT { public int X; public int Y; }

    [ComImport]
    [Guid("00000122-0000-0000-C000-000000000046")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IDropTargetCom
    {
        [PreserveSig] int DragEnter([MarshalAs(UnmanagedType.Interface)] System.Runtime.InteropServices.ComTypes.IDataObject pDataObj,
            int grfKeyState, POINT pt, ref int pdwEffect);
        [PreserveSig] int DragOver(int grfKeyState, POINT pt, ref int pdwEffect);
        [PreserveSig] int DragLeave();
        [PreserveSig] int Drop([MarshalAs(UnmanagedType.Interface)] System.Runtime.InteropServices.ComTypes.IDataObject pDataObj,
            int grfKeyState, POINT pt, ref int pdwEffect);
    }
}
