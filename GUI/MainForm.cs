using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Text;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;
using SmallDemoManager.Bridge;
using SmallDemoManager.UtilClass;
using ComIDataObject = System.Runtime.InteropServices.ComTypes.IDataObject;
using WinIDataObject = System.Windows.Forms.IDataObject;

namespace SmallDemoManager.GUI
{
    public sealed class MainForm : Form
    {
        private const int WM_NCHITTEST = 0x0084;
        private const int WM_NCLBUTTONDOWN = 0x00A1;
        private const int HTCLIENT = 1, HTCAPTION = 2, HTLEFT = 10, HTRIGHT = 11,
            HTTOP = 12, HTTOPLEFT = 13, HTTOPRIGHT = 14, HTBOTTOM = 15,
            HTBOTTOMLEFT = 16, HTBOTTOMRIGHT = 17;
        private const int ResizeBorder = 6;

        [DllImport("user32.dll")] private static extern bool ReleaseCapture();
        [DllImport("user32.dll", CharSet = CharSet.Auto)]
        private static extern int SendMessage(IntPtr hWnd, int Msg, int wParam, int lParam);
        [DllImport("dwmapi.dll", PreserveSig = true)]
        private static extern int DwmSetWindowAttribute(IntPtr hwnd, int attr, ref int value, int valueSize);
        private const int DWMWA_WINDOW_CORNER_PREFERENCE = 33;
        private const int DWMWCP_ROUND = 2;

        [DllImport("ole32.dll", PreserveSig = true)]
        private static extern int RegisterDragDrop(IntPtr hwnd, IDropTargetCom pDropTarget);
        [DllImport("ole32.dll", PreserveSig = true)]
        private static extern int RevokeDragDrop(IntPtr hwnd);
        [DllImport("user32.dll")]
        private static extern IntPtr GetWindow(IntPtr hWnd, uint uCmd);
        private const uint GW_CHILD = 5;

        [DllImport("user32.dll")]
        private static extern bool EnumChildWindows(IntPtr hWndParent,
            EnumChildProc lpEnumFunc, IntPtr lParam);
        private delegate bool EnumChildProc(IntPtr hWnd, IntPtr lParam);

        private WebViewDropForwarder? _dropForwarder;
        private readonly List<IntPtr> _registeredDropHwnds = new();

        private readonly WebView2 _web;
        private readonly BridgeService _bridge;
        private readonly string? _startupDemo;

        public MainForm(string? startupDemo)
        {
            _startupDemo = startupDemo;

            Text = "SmallDemoManager";
            Icon = LoadIcon();
            // Match the HTML <body> bg so any pixels visible outside the rounded
            // WebView2 (e.g. while resizing or pre-paint) blend in.
            BackColor = Color.FromArgb(6, 7, 9);
            ForeColor = Color.White;
            MinimumSize = new Size(960, 600);
            ClientSize = new Size(1280, 800);
            StartPosition = FormStartPosition.CenterScreen;
            FormBorderStyle = FormBorderStyle.None;
            DoubleBuffered = true;

            // AllowExternalDrop=false stops WebView2 from registering its own drop
            // target (which only delivers HTML5 File objects — no archive-virtual
            // file support). After init we replace it with WebViewDropForwarder so
            // we can read CFSTR_FILEDESCRIPTORW/CFSTR_FILECONTENTS for 7-Zip/WinRAR.
            _web = new WebView2 { Dock = DockStyle.Fill, AllowExternalDrop = false };
            Controls.Add(_web);

            _bridge = new BridgeService(this);
            _bridge.WindowDragRequested = BeginWindowDrag;

            AllowDrop = true;
            DragEnter += OnDragEnter;
            DragLeave += OnDragLeave;
            DragDrop += OnDragDrop;

            Load += async (_, _) => await InitializeWebViewAsync();
            HandleCreated += (_, _) => ApplyWindowCorners();
            FormClosed += (_, _) => _bridge.Dispose();
        }

        // CFSTR_FILEDESCRIPTORW / CFSTR_FILECONTENTS — the shell's virtual-file
        // drag/drop format used by WinRAR, 7-Zip, the Explorer zip handler, etc.
        // (Files inside an archive don't exist on disk until you extract them.)
        private const string CF_FILEDESCRIPTORW = "FileGroupDescriptorW";
        private const string CF_FILECONTENTS = "FileContents";
        // FILEDESCRIPTORW: dwFlags(4)+clsid(16)+SIZEL(8)+POINTL(8)+dwFileAttributes(4)
        //   +3*FILETIME(24)+nFileSizeHigh(4)+nFileSizeLow(4)+WCHAR[260](520) = 592 bytes,
        //   with cFileName at offset 72.
        private const int FD_SIZE = 592;
        private const int FD_NAME_OFFSET = 72;
        private const int FD_NAME_BYTES = 520;

        [DllImport("ole32.dll")]
        private static extern void ReleaseStgMedium(ref STGMEDIUM stg);
        [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern ushort RegisterClipboardFormatW(string lpszFormat);
        [DllImport("kernel32.dll")] private static extern IntPtr GlobalLock(IntPtr h);
        [DllImport("kernel32.dll")] private static extern bool GlobalUnlock(IntPtr h);
        [DllImport("kernel32.dll")] private static extern int GlobalSize(IntPtr h);

        private static string[] ExtractRealDemoPaths(WinIDataObject? data)
        {
            if (data == null || !data.GetDataPresent(DataFormats.FileDrop)) return Array.Empty<string>();
            if (data.GetData(DataFormats.FileDrop) is not string[] paths) return Array.Empty<string>();
            return paths
                .Where(p => !string.IsNullOrWhiteSpace(p)
                    && p.EndsWith(".dem", StringComparison.OrdinalIgnoreCase)
                    && File.Exists(p))
                .ToArray();
        }

        private static List<string> ReadVirtualDemoNames(WinIDataObject? data)
        {
            var names = new List<string>();
            if (data == null || !data.GetDataPresent(CF_FILEDESCRIPTORW)) return names;
            if (data.GetData(CF_FILEDESCRIPTORW) is not MemoryStream desc) return names;
            var bytes = desc.ToArray();
            if (bytes.Length < 4) return names;
            int count = BitConverter.ToInt32(bytes, 0);
            for (int i = 0; i < count; i++)
            {
                int off = 4 + i * FD_SIZE + FD_NAME_OFFSET;
                if (off + FD_NAME_BYTES > bytes.Length) break;
                var raw = Encoding.Unicode.GetString(bytes, off, FD_NAME_BYTES);
                int nul = raw.IndexOf('\0');
                if (nul >= 0) raw = raw[..nul];
                if (raw.EndsWith(".dem", StringComparison.OrdinalIgnoreCase)) names.Add(raw);
                else names.Add(""); // keep index alignment; "" means "skip"
            }
            return names;
        }

        private static string UniqueStagingPath(string dir, string fileName)
        {
            var dest = Path.Combine(dir, fileName);
            if (!File.Exists(dest)) return dest;
            var stem = Path.GetFileNameWithoutExtension(fileName);
            var ext = Path.GetExtension(fileName);
            for (int i = 1; i < 1000; i++)
            {
                var c = Path.Combine(dir, $"{stem} ({i}){ext}");
                if (!File.Exists(c)) return c;
            }
            return Path.Combine(dir, Guid.NewGuid().ToString("N") + ext);
        }

        private static byte[] HGlobalToBytes(IntPtr h)
        {
            int size = GlobalSize(h);
            var ptr = GlobalLock(h);
            try
            {
                var buf = new byte[size];
                Marshal.Copy(ptr, buf, 0, size);
                return buf;
            }
            finally { GlobalUnlock(h); }
        }

        private static byte[] IStreamToBytes(IntPtr p)
        {
            var stream = (IStream)Marshal.GetObjectForIUnknown(p);
            try
            {
                stream.Stat(out var stat, 0 /* STATFLAG_DEFAULT */);
                long total = stat.cbSize;
                using var ms = new MemoryStream(capacity: total > int.MaxValue ? 0 : (int)total);
                var chunk = new byte[64 * 1024];
                while (true)
                {
                    int read = 0;
                    IntPtr pcb = Marshal.AllocHGlobal(sizeof(int));
                    try
                    {
                        stream.Read(chunk, chunk.Length, pcb);
                        read = Marshal.ReadInt32(pcb);
                    }
                    finally { Marshal.FreeHGlobal(pcb); }
                    if (read <= 0) break;
                    ms.Write(chunk, 0, read);
                }
                return ms.ToArray();
            }
            finally { Marshal.ReleaseComObject(stream); }
        }

        internal static bool HasAnyDemoSource(WinIDataObject? data)
        {
            if (data == null) return false;
            if (ExtractRealDemoPaths(data).Length > 0) return true;
            var names = ReadVirtualDemoNames(data);
            if (names.Any(n => !string.IsNullOrEmpty(n))) return true;
            // 7-Zip / shell namespace extensions: items come as Shell IDLists, not
            // CFSTR_FILEDESCRIPTORW. Resolve via the shell to detect .dem entries.
            if (data is ComIDataObject com && ShellDropResolver.CountDemoItems(com) > 0) return true;
            return false;
        }

        private void OnDragEnter(object? sender, DragEventArgs e)
        {
            e.Effect = HasAnyDemoSource(e.Data) ? DragDropEffects.Copy : DragDropEffects.None;
            if (e.Effect == DragDropEffects.Copy && e.Data != null) HandleDragEnter(e.Data);
        }
        private void OnDragLeave(object? sender, EventArgs e) => HandleDragLeave();
        private void OnDragDrop(object? sender, DragEventArgs e)
        {
            if (e.Data != null) HandleDragDrop(e.Data);
        }

        // Reusable from both the WinForms event handlers (kept as a fallback for the
        // form's bare margins) and the WebView2-attached IDropTarget.

        internal void HandleDragEnter(WinIDataObject data)
        {
            BridgeService.DebugLog($"DragEnter formats={string.Join(",", data.GetFormats())}");
            if (data is ComIDataObject comEnter) ShellDropResolver.DumpFormats(comEnter, "DragEnter");
            bool fromArchive = ExtractRealDemoPaths(data).Length == 0;
            _bridge.Emit("drag-active", new { active = true, fromArchive });
        }

        internal void HandleDragLeave()
        {
            BridgeService.DebugLog("DragLeave");
            _bridge.Emit("drag-active", new { active = false });
        }

        // Used by the IDropTarget forwarder when it already resolved the dropped
        // files (e.g. raw CF_HDROP path) and just needs to notify React.
        internal void EmitFilesDropped(string[] paths, bool staged)
        {
            _bridge.Emit("drag-active", new { active = false });
            _bridge.Emit("files-dropped", new { paths, staged });
        }

        internal void HandleDragDrop(WinIDataObject data)
        {
            BridgeService.DebugLog($"DragDrop formats={string.Join(",", data.GetFormats())}");
            _bridge.Emit("drag-active", new { active = false });

            var real = ExtractRealDemoPaths(data);
            BridgeService.DebugLog($"DragDrop realPaths={real.Length}");
            if (real.Length > 0)
            {
                _bridge.Emit("files-dropped", new { paths = real, staged = false });
                return;
            }

            // Virtual files (WinRAR / 7-Zip CFSTR_FILECONTENTS path). Extraction is
            // synchronous on the UI thread because the OLE IDataObject is only safe to
            // read for the lifetime of this event, so emit "extracting" +
            // Application.DoEvents() to flush the overlay into the WebView before the
            // loop blocks the message pump.
            var names = ReadVirtualDemoNames(data);
            int total = names.Count(n => !string.IsNullOrEmpty(n));
            BridgeService.DebugLog($"DragDrop virtualNames={total}");

            if (data is not ComIDataObject com)
            {
                BridgeService.DebugLog("DragDrop: data is not ComIDataObject");
                return;
            }

            if (total == 0)
            {
                // Try the Shell namespace path (7-Zip, Explorer ZIP, any IShellFolder
                // namespace extension). Items arrive as PIDLs in CFSTR_SHELLIDLIST and
                // we resolve them to IStream via BindToHandler(BHID_Stream).
                var shellStaging = LocalAppDataFolder.EnsureSubDirectoryExists("drop-staging");
                _bridge.Emit("extracting", new { active = true, current = 0, total = 1 });
                Application.DoEvents();
                var shellPaths = ShellDropResolver.ExtractDemoItems(com, shellStaging);
                _bridge.Emit("extracting", new { active = false, current = shellPaths.Count, total = shellPaths.Count });
                BridgeService.DebugLog($"DragDrop shellPaths={shellPaths.Count}");
                if (shellPaths.Count > 0)
                    _bridge.Emit("files-dropped", new { paths = shellPaths.ToArray(), staged = true });
                return;
            }

            _bridge.Emit("extracting", new { active = true, current = 0, total });
            Application.DoEvents();

            var staging = LocalAppDataFolder.EnsureSubDirectoryExists("drop-staging");
            short fcFormat = (short)RegisterClipboardFormatW(CF_FILECONTENTS);
            var written = new List<string>();
            int done = 0;

            for (int i = 0; i < names.Count; i++)
            {
                if (string.IsNullOrEmpty(names[i])) continue;
                var fileName = Path.GetFileName(names[i].Replace('/', '\\'));
                if (string.IsNullOrWhiteSpace(fileName)) { done++; continue; }

                _bridge.Emit("extracting", new {
                    active = true, current = done, total, file = fileName,
                });
                Application.DoEvents();

                var fmt = new FORMATETC
                {
                    cfFormat = fcFormat,
                    ptd = IntPtr.Zero,
                    dwAspect = DVASPECT.DVASPECT_CONTENT,
                    lindex = i,
                    tymed = TYMED.TYMED_HGLOBAL | TYMED.TYMED_ISTREAM,
                };
                var stg = default(STGMEDIUM);
                try
                {
                    com.GetData(ref fmt, out stg);
                    byte[]? content = stg.tymed switch
                    {
                        TYMED.TYMED_HGLOBAL => HGlobalToBytes(stg.unionmember),
                        TYMED.TYMED_ISTREAM => IStreamToBytes(stg.unionmember),
                        _ => null,
                    };
                    if (content != null)
                    {
                        var dest = UniqueStagingPath(staging, fileName);
                        File.WriteAllBytes(dest, content);
                        written.Add(dest);
                    }
                }
                catch { /* skip entry */ }
                finally
                {
                    if (stg.tymed != TYMED.TYMED_NULL) ReleaseStgMedium(ref stg);
                    done++;
                }
            }

            _bridge.Emit("extracting", new { active = false, current = total, total });
            if (written.Count > 0)
                _bridge.Emit("files-dropped", new { paths = written.ToArray(), staged = true });
        }

        private void ApplyWindowCorners()
        {
            // Win11 22H1+ rounds the whole window via DWM. On Win10 this no-ops with a
            // harmless non-zero return code, so the rectangular form still works fine.
            try
            {
                int preference = DWMWCP_ROUND;
                DwmSetWindowAttribute(Handle, DWMWA_WINDOW_CORNER_PREFERENCE,
                    ref preference, sizeof(int));
            }
            catch { }
        }

        private void BeginWindowDrag()
        {
            if (InvokeRequired) { BeginInvoke(BeginWindowDrag); return; }
            ReleaseCapture();
            SendMessage(Handle, WM_NCLBUTTONDOWN, HTCAPTION, 0);
        }

        protected override void WndProc(ref Message m)
        {
            if (m.Msg == WM_NCHITTEST)
            {
                base.WndProc(ref m);
                if ((int)m.Result == HTCLIENT)
                {
                    int x = (short)(m.LParam.ToInt64() & 0xFFFF);
                    int y = (short)((m.LParam.ToInt64() >> 16) & 0xFFFF);
                    var pt = PointToClient(new Point(x, y));
                    int w = ClientSize.Width, h = ClientSize.Height;
                    bool onLeft = pt.X < ResizeBorder;
                    bool onRight = pt.X >= w - ResizeBorder;
                    bool onTop = pt.Y < ResizeBorder;
                    bool onBottom = pt.Y >= h - ResizeBorder;
                    int hit = HTCLIENT;
                    if (onTop && onLeft) hit = HTTOPLEFT;
                    else if (onTop && onRight) hit = HTTOPRIGHT;
                    else if (onBottom && onLeft) hit = HTBOTTOMLEFT;
                    else if (onBottom && onRight) hit = HTBOTTOMRIGHT;
                    else if (onLeft) hit = HTLEFT;
                    else if (onRight) hit = HTRIGHT;
                    else if (onTop) hit = HTTOP;
                    else if (onBottom) hit = HTBOTTOM;
                    m.Result = (IntPtr)hit;
                }
                return;
            }
            base.WndProc(ref m);
        }

        private static Icon LoadIcon()
        {
            try
            {
                var icoPath = Path.Combine(AppContext.BaseDirectory, "iconApp.ico");
                if (File.Exists(icoPath)) return new Icon(icoPath);
            }
            catch { }
            return SystemIcons.Application;
        }

        private async Task InitializeWebViewAsync()
        {
            LocalAppDataFolder.EnsureRootDirectoryExists();
            var userDataDir = LocalAppDataFolder.EnsureSubDirectoryExists("WebView2");
            var env = await CoreWebView2Environment.CreateAsync(null, userDataDir);
            await _web.EnsureCoreWebView2Async(env);

            var settings = _web.CoreWebView2.Settings;
            settings.AreDevToolsEnabled = true;
            settings.AreDefaultContextMenusEnabled = false;
            settings.IsStatusBarEnabled = false;
            settings.IsZoomControlEnabled = false;
            settings.IsSwipeNavigationEnabled = false;

            var uiDir = Path.Combine(AppContext.BaseDirectory, "NewUI");
            _web.CoreWebView2.SetVirtualHostNameToFolderMapping(
                "sdm.local", uiDir, CoreWebView2HostResourceAccessKind.Allow);

            _web.CoreWebView2.WebMessageReceived += (_, e) =>
            {
                try { _bridge.HandleMessage(e.TryGetWebMessageAsString()); }
                catch (Exception ex) { Console.Error.WriteLine(ex); }
            };

            _bridge.SetMessagePoster(json =>
            {
                if (_web.IsDisposed) return;
                // Both CoreWebView2 getter and PostWebMessageAsString must run on the UI thread.
                void Send()
                {
                    if (_web.IsDisposed) return;
                    var core = _web.CoreWebView2;
                    if (core == null) return;
                    try { core.PostWebMessageAsString(json); }
                    catch (Exception ex) { Console.Error.WriteLine("PostWebMessageAsString failed: " + ex); }
                }
                if (InvokeRequired)
                {
                    try { BeginInvoke((Action)Send); }
                    catch (Exception ex) { Console.Error.WriteLine("BeginInvoke failed: " + ex); }
                }
                else Send();
            });

            _bridge.StartupDemo = _startupDemo;

            _web.CoreWebView2.Navigate("https://sdm.local/index.html");

            // Once Navigate kicks off WebView2 creates its renderer child window. Wait
            // for the first non-empty render of the document and then take over its
            // drop target. NavigationCompleted is a reliable signal that the renderer
            // HWND is alive.
            void OnNavigated(object? s, CoreWebView2NavigationCompletedEventArgs e)
            {
                _web.CoreWebView2.NavigationCompleted -= OnNavigated;
                BeginInvoke((Action)InstallWebViewDropTarget);
            }
            _web.CoreWebView2.NavigationCompleted += OnNavigated;
        }

        private void InstallWebViewDropTarget()
        {
            // WebView2 nests multiple child HWNDs (Chromium widget host + render-
            // process surfaces). The OS routes a drop to whichever HWND is under
            // the cursor at release time — and the cursor can cross between those
            // children mid-drag — so we register our drop target on every
            // descendant. Same forwarder instance receives the drop regardless.
            _dropForwarder = new WebViewDropForwarder(this);
            _registeredDropHwnds.Clear();

            var descendants = new List<IntPtr> { _web.Handle };
            EnumChildWindows(_web.Handle, (hWnd, _) => { descendants.Add(hWnd); return true; }, IntPtr.Zero);

            foreach (var h in descendants)
            {
                try { RevokeDragDrop(h); } catch { /* none registered — fine */ }
                int hr = RegisterDragDrop(h, _dropForwarder);
                if (hr == 0) _registeredDropHwnds.Add(h);
            }

            BridgeService.DebugLog(
                $"InstallWebViewDropTarget: registered on {_registeredDropHwnds.Count}/{descendants.Count} HWNDs " +
                $"[{string.Join(",", _registeredDropHwnds.Select(h => "0x" + h.ToInt64().ToString("X")))}]");
        }

        protected override void OnFormClosed(FormClosedEventArgs e)
        {
            foreach (var h in _registeredDropHwnds)
            {
                try { RevokeDragDrop(h); } catch { }
            }
            _registeredDropHwnds.Clear();
            _dropForwarder = null;
            base.OnFormClosed(e);
        }
    }
}
