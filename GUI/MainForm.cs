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

            _web = new WebView2 { Dock = DockStyle.Fill, AllowExternalDrop = false };
            Controls.Add(_web);

            _bridge = new BridgeService(this);
            _bridge.WindowDragRequested = BeginWindowDrag;

            // AllowExternalDrop=false makes WebView2 decline OLE drops, so dragging
            // a .dem file from Explorer onto the window falls through to the Form.
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

        private static bool HasAnyDemoSource(WinIDataObject? data)
        {
            if (ExtractRealDemoPaths(data).Length > 0) return true;
            var names = ReadVirtualDemoNames(data);
            return names.Any(n => !string.IsNullOrEmpty(n));
        }

        private void OnDragEnter(object? sender, DragEventArgs e)
        {
            bool has = HasAnyDemoSource(e.Data);
            e.Effect = has ? DragDropEffects.Copy : DragDropEffects.None;
            if (has)
            {
                bool fromArchive = ExtractRealDemoPaths(e.Data).Length == 0;
                _bridge.Emit("drag-active", new { active = true, fromArchive });
            }
        }

        private void OnDragLeave(object? sender, EventArgs e)
        {
            _bridge.Emit("drag-active", new { active = false });
        }

        private void OnDragDrop(object? sender, DragEventArgs e)
        {
            _bridge.Emit("drag-active", new { active = false });

            var real = ExtractRealDemoPaths(e.Data);
            if (real.Length > 0)
            {
                _bridge.Emit("files-dropped", new { paths = real, staged = false });
                return;
            }

            // Virtual files (WinRAR / 7-Zip etc.). Extraction is synchronous on the UI
            // thread because the OLE IDataObject is only safe to read for the lifetime
            // of this event, so emit "extracting" + Application.DoEvents() to flush the
            // overlay into the WebView before the loop blocks the message pump.
            var names = ReadVirtualDemoNames(e.Data);
            int total = names.Count(n => !string.IsNullOrEmpty(n));
            if (total == 0 || e.Data is not ComIDataObject com) return;

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
        }
    }
}
