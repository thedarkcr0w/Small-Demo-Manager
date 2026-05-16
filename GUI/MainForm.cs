using System.Runtime.InteropServices;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;
using SmallDemoManager.Bridge;
using SmallDemoManager.UtilClass;

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

            Load += async (_, _) => await InitializeWebViewAsync();
            HandleCreated += (_, _) => ApplyWindowCorners();
            FormClosed += (_, _) => _bridge.Dispose();
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
