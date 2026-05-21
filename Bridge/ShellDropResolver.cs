using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Text;

namespace SmallDemoManager.Bridge
{
    /// <summary>
    /// Resolves dragged Shell items (CFSTR_SHELLIDLIST / "Shell IDList Array")
    /// into actual file bytes via the Shell namespace. This is how 7-Zip exposes
    /// files dragged out of an archive: the IDataObject contains PIDLs into the
    /// archive's virtual namespace and the receiver is expected to bind each
    /// item to an <c>IStream</c> to read the content.
    /// </summary>
    internal static class ShellDropResolver
    {
        private const string CFSTR_SHELLIDLIST = "Shell IDList Array";
        private static readonly Guid BHID_Stream = new("1cebb3ab-7c10-499a-a417-92ca16c4cb83");
        private static readonly Guid IID_IStream = typeof(IStream).GUID;
        private static readonly Guid IID_IShellItem = new("43826d1e-e718-42ee-bc55-a1e261c37bfe");

        private const int SIGDN_NORMALDISPLAY         = 0x00000000;
        private const int SIGDN_PARENTRELATIVEPARSING = unchecked((int)0x80018001);
        private const int SIGDN_DESKTOPABSOLUTEPARSING = unchecked((int)0x80028000);

        [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern ushort RegisterClipboardFormatW(string lpszFormat);
        [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern int GetClipboardFormatNameW(uint format, StringBuilder lpszFormatName, int cchMaxCount);

        [DllImport("ole32.dll")] private static extern void ReleaseStgMedium(ref STGMEDIUM stg);
        [DllImport("kernel32.dll")] private static extern IntPtr GlobalLock(IntPtr h);
        [DllImport("kernel32.dll")] private static extern bool GlobalUnlock(IntPtr h);

        [DllImport("shell32.dll", PreserveSig = false)]
        private static extern void SHCreateItemFromIDList(IntPtr pidl, [In] ref Guid riid,
            [MarshalAs(UnmanagedType.Interface)] out IShellItem ppv);

        public static int CountDemoItems(System.Runtime.InteropServices.ComTypes.IDataObject data)
        {
            return ResolveItems(data, justCount: true, stagingDir: null, written: null);
        }

        public static List<string> ExtractDemoItems(System.Runtime.InteropServices.ComTypes.IDataObject data, string stagingDir)
        {
            var written = new List<string>();
            ResolveItems(data, justCount: false, stagingDir: stagingDir, written: written);
            return written;
        }

        // Direct CF_HDROP read off a raw COM IDataObject. We bypass WinForms'
        // DataObject wrapper because that wrapper sometimes hides CF_HDROP when
        // the data comes straight from an IDropTarget event (which is exactly
        // our case — the forwarder hands us pDataObj from OLE).
        public static string[] ReadHDropFiles(System.Runtime.InteropServices.ComTypes.IDataObject data)
        {
            const short CF_HDROP = 15;
            if (data == null) return Array.Empty<string>();
            var fmt = new FORMATETC
            {
                cfFormat = CF_HDROP,
                ptd = IntPtr.Zero,
                dwAspect = DVASPECT.DVASPECT_CONTENT,
                lindex = -1,
                tymed = TYMED.TYMED_HGLOBAL,
            };
            var stg = default(STGMEDIUM);
            try { data.GetData(ref fmt, out stg); }
            catch (Exception ex)
            {
                BridgeService.DebugLog("GetData(CF_HDROP) failed: " + ex.Message);
                return Array.Empty<string>();
            }
            if (stg.tymed != TYMED.TYMED_HGLOBAL || stg.unionmember == IntPtr.Zero)
            {
                if (stg.tymed != TYMED.TYMED_NULL) ReleaseStgMedium(ref stg);
                return Array.Empty<string>();
            }
            var paths = new List<string>();
            IntPtr hMem = stg.unionmember;
            IntPtr basePtr = GlobalLock(hMem);
            try
            {
                if (basePtr == IntPtr.Zero)
                {
                    BridgeService.DebugLog("ReadHDropFiles: GlobalLock returned null");
                    return Array.Empty<string>();
                }
                // DROPFILES layout: DWORD pFiles, POINT pt(8), BOOL fNC(4), BOOL fWide(4)
                int pFiles = Marshal.ReadInt32(basePtr, 0);
                bool wide = Marshal.ReadInt32(basePtr, 16) != 0;
                int offset = pFiles;
                while (true)
                {
                    string? s = wide
                        ? Marshal.PtrToStringUni(IntPtr.Add(basePtr, offset))
                        : Marshal.PtrToStringAnsi(IntPtr.Add(basePtr, offset));
                    if (string.IsNullOrEmpty(s)) break;
                    paths.Add(s);
                    offset += (s.Length + 1) * (wide ? 2 : 1);
                }
            }
            finally
            {
                GlobalUnlock(hMem);
                ReleaseStgMedium(ref stg);
            }
            BridgeService.DebugLog($"ReadHDropFiles: {paths.Count} path(s) → [{string.Join(" | ", paths.Take(5))}]");
            return paths.ToArray();
        }

        // Dump every format the IDataObject exposes (with its supported tymed) into
        // the bridge log. Useful when SHCreate/our resolution path can't find what
        // we expect.
        public static void DumpFormats(System.Runtime.InteropServices.ComTypes.IDataObject data, string label)
        {
            if (data == null) return;
            try
            {
                var enumerator = data.EnumFormatEtc(DATADIR.DATADIR_GET);
                if (enumerator == null) return;
                var arr = new FORMATETC[1];
                var sb = new StringBuilder();
                while (true)
                {
                    int hr = enumerator.Next(1, arr, null);
                    if (hr != 0) break;
                    var nameBuf = new StringBuilder(256);
                    var n = GetClipboardFormatNameW((uint)(ushort)arr[0].cfFormat, nameBuf, 256);
                    string fmtName = n > 0 ? nameBuf.ToString() : ("CF_" + ((ushort)arr[0].cfFormat).ToString("X"));
                    sb.Append($"[fmt={fmtName} cf=0x{(ushort)arr[0].cfFormat:X} tymed=0x{(int)arr[0].tymed:X} lindex={arr[0].lindex}] ");
                }
                BridgeService.DebugLog($"{label} formats: {sb}");
            }
            catch (Exception ex)
            {
                BridgeService.DebugLog($"{label} format dump failed: {ex.Message}");
            }
        }

        private static int ResolveItems(System.Runtime.InteropServices.ComTypes.IDataObject data, bool justCount, string? stagingDir, List<string>? written)
        {
            if (data == null) return 0;

            // CFSTR_SHELLIDLIST is delivered as an HGLOBAL containing a CIDA struct:
            //   UINT cidl;            // number of items
            //   UINT aoffset[cidl+1]; // 0 = parent folder PIDL, 1..cidl = child PIDLs
            //   <PIDLs back-to-back>
            // The exact aspect/lindex/tymed the source advertises varies (7-Zip uses
            // dwAspect != CONTENT in some configurations) so we enumerate every
            // available FORMATETC and find the matching cfFormat instead of guessing.
            short cfId = (short)RegisterClipboardFormatW(CFSTR_SHELLIDLIST);
            if (!TryGetSourceFormatEtc(data, cfId, out var fmt))
            {
                BridgeService.DebugLog("No FORMATETC entry matches Shell IDList Array");
                return 0;
            }
            // Force HGLOBAL — CIDA is always delivered as a memory block.
            fmt.tymed = TYMED.TYMED_HGLOBAL;
            var stg = default(STGMEDIUM);
            try { data.GetData(ref fmt, out stg); }
            catch (Exception ex)
            {
                BridgeService.DebugLog(
                    $"GetData(Shell IDList Array) failed: {ex.Message} " +
                    $"[cf=0x{(ushort)fmt.cfFormat:X} aspect={fmt.dwAspect} lindex={fmt.lindex} tymed=0x{(int)fmt.tymed:X}]");
                return 0;
            }

            if (stg.tymed != TYMED.TYMED_HGLOBAL || stg.unionmember == IntPtr.Zero)
            {
                BridgeService.DebugLog($"Shell IDList tymed=0x{(int)stg.tymed:X} member=0x{stg.unionmember.ToInt64():X}");
                if (stg.tymed != TYMED.TYMED_NULL) ReleaseStgMedium(ref stg);
                return 0;
            }

            int hits = 0;
            IntPtr hMem = stg.unionmember;
            IntPtr basePtr = GlobalLock(hMem);
            try
            {
                if (basePtr == IntPtr.Zero) { BridgeService.DebugLog("GlobalLock returned null"); return 0; }
                int cidl = Marshal.ReadInt32(basePtr);
                // aoffset[0] is offset to parent PIDL, aoffset[1..cidl] are child offsets.
                // The parent is folder-relative; combining parent+child gives an absolute PIDL.
                IntPtr parentRelPidl = IntPtr.Add(basePtr, Marshal.ReadInt32(basePtr, 4));
                IntPtr parentAbsPidl = IlClone(parentRelPidl);

                for (int i = 1; i <= cidl; i++)
                {
                    int offset = Marshal.ReadInt32(basePtr, 4 + i * 4);
                    IntPtr childRelPidl = IntPtr.Add(basePtr, offset);
                    IntPtr absPidl = IlCombine(parentAbsPidl, childRelPidl);
                    if (absPidl == IntPtr.Zero) continue;
                    try
                    {
                        IShellItem? item = null;
                        try
                        {
                            var iid = IID_IShellItem;
                            SHCreateItemFromIDList(absPidl, ref iid, out item);
                        }
                        catch (Exception ex)
                        {
                            BridgeService.DebugLog($"SHCreateItemFromIDList[{i}] failed: {ex.Message}");
                            continue;
                        }
                        if (item == null) continue;

                        var name = GetDisplayName(item);
                        bool isDem = !string.IsNullOrEmpty(name) &&
                                     name.EndsWith(".dem", StringComparison.OrdinalIgnoreCase);

                        if (justCount)
                        {
                            if (isDem) hits++;
                        }
                        else if (isDem && stagingDir != null && written != null)
                        {
                            try
                            {
                                var leaf = Path.GetFileName(name.Replace('/', '\\'));
                                var safe = string.Concat(leaf.Where(c => !Path.GetInvalidFileNameChars().Contains(c)));
                                if (!safe.EndsWith(".dem", StringComparison.OrdinalIgnoreCase)) safe += ".dem";
                                var dest = UniquePath(stagingDir, safe);

                                var bhid = BHID_Stream;
                                var iidStream = IID_IStream;
                                item.BindToHandler(IntPtr.Zero, ref bhid, ref iidStream, out IntPtr streamUnk);
                                if (streamUnk == IntPtr.Zero) { BridgeService.DebugLog($"BindToHandler(BHID_Stream) returned null for {name}"); continue; }
                                IStream stream;
                                try { stream = (IStream)Marshal.GetObjectForIUnknown(streamUnk); }
                                finally { Marshal.Release(streamUnk); }
                                try { WriteIStreamToFile(stream, dest); written.Add(dest); hits++; }
                                finally { Marshal.ReleaseComObject(stream); }
                            }
                            catch (Exception ex) { BridgeService.DebugLog($"item[{i}] '{name}' extract failed: {ex.Message}"); }
                        }

                        Marshal.ReleaseComObject(item);
                    }
                    finally { if (absPidl != IntPtr.Zero) Marshal.FreeCoTaskMem(absPidl); }
                }

                if (parentAbsPidl != IntPtr.Zero) Marshal.FreeCoTaskMem(parentAbsPidl);
            }
            finally
            {
                GlobalUnlock(hMem);
                ReleaseStgMedium(ref stg);
            }
            return hits;
        }

        // Walk the IDataObject's advertised FORMATETC entries and find one whose
        // cfFormat matches what we want. Returns the source's own struct so we
        // GetData() with values it actually supports (right aspect, lindex, etc.).
        private static bool TryGetSourceFormatEtc(System.Runtime.InteropServices.ComTypes.IDataObject data, short cfFormat, out FORMATETC found)
        {
            found = default;
            try
            {
                var enumerator = data.EnumFormatEtc(DATADIR.DATADIR_GET);
                if (enumerator == null) return false;
                var arr = new FORMATETC[1];
                while (enumerator.Next(1, arr, null) == 0)
                {
                    if ((short)arr[0].cfFormat == cfFormat)
                    {
                        found = arr[0];
                        return true;
                    }
                }
            }
            catch (Exception ex)
            {
                BridgeService.DebugLog("EnumFormatEtc failed: " + ex.Message);
            }
            return false;
        }

        // ILClone / ILCombine via shell32 ordinals. Available since Win 2000.
        [DllImport("shell32.dll", EntryPoint = "ILClone", CharSet = CharSet.Unicode, SetLastError = false)]
        private static extern IntPtr IlClone(IntPtr pidl);
        [DllImport("shell32.dll", EntryPoint = "ILCombine", CharSet = CharSet.Unicode, SetLastError = false)]
        private static extern IntPtr IlCombine(IntPtr pidl1, IntPtr pidl2);

        private static string GetDisplayName(IShellItem item)
        {
            foreach (var sigdn in new[] { SIGDN_PARENTRELATIVEPARSING, SIGDN_NORMALDISPLAY, SIGDN_DESKTOPABSOLUTEPARSING })
            {
                IntPtr p = IntPtr.Zero;
                try
                {
                    item.GetDisplayName(sigdn, out p);
                    if (p != IntPtr.Zero)
                    {
                        var s = Marshal.PtrToStringUni(p);
                        if (!string.IsNullOrEmpty(s)) return s!;
                    }
                }
                catch { }
                finally { if (p != IntPtr.Zero) Marshal.FreeCoTaskMem(p); }
            }
            return "";
        }

        private static void WriteIStreamToFile(IStream src, string destPath)
        {
            using var fs = File.Create(destPath);
            var buffer = new byte[81920];
            var pcb = Marshal.AllocCoTaskMem(sizeof(int));
            try
            {
                while (true)
                {
                    src.Read(buffer, buffer.Length, pcb);
                    int read = Marshal.ReadInt32(pcb);
                    if (read <= 0) break;
                    fs.Write(buffer, 0, read);
                }
            }
            finally { Marshal.FreeCoTaskMem(pcb); }
        }

        private static string UniquePath(string dir, string fileName)
        {
            var dest = Path.Combine(dir, fileName);
            if (!File.Exists(dest)) return dest;
            var stem = Path.GetFileNameWithoutExtension(fileName);
            var ext = Path.GetExtension(fileName);
            for (int i = 1; i < 1000; i++)
            {
                dest = Path.Combine(dir, $"{stem} ({i}){ext}");
                if (!File.Exists(dest)) return dest;
            }
            return Path.Combine(dir, $"{stem}-{Guid.NewGuid():N}{ext}");
        }

        [ComImport]
        [Guid("43826d1e-e718-42ee-bc55-a1e261c37bfe")]
        [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
        private interface IShellItem
        {
            void BindToHandler(IntPtr pbc, ref Guid bhid, ref Guid riid, out IntPtr ppv);
            void GetParent(out IShellItem ppsi);
            void GetDisplayName(int sigdnName, out IntPtr ppszName);
            void GetAttributes(uint sfgaoMask, out uint psfgaoAttribs);
            void Compare(IShellItem psi, uint hint, out int piOrder);
        }
    }
}
