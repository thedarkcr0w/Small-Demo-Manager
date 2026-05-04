// DemoFile Parser
using DemoFile;
using DemoFile.Game.Cs;
// WinForms Framework ReaLTaiizor
using ReaLTaiizor.Colors;
using ReaLTaiizor.Controls;
using ReaLTaiizor.Enum.Material;
using ReaLTaiizor.Forms;
using ReaLTaiizor.Manager;
using ReaLTaiizor.Util;
using SmallDemoManager.AudioExtract;
// My Class
using SmallDemoManager.HelperClass;
using SmallDemoManager.UtilClass;
// System
using System;
using System.Data;
using System.Diagnostics.Metrics;
using System.Drawing;
using System.Security.Policy;
using System.Text;
using System.Windows.Controls.Primitives;
using System.Windows.Forms;
using static CDataGCCStrike15_v2_TournamentMatchDraft.Types;
using static CMsgGCCStrike15_v2_Account_RequestCoPlays.Types;

namespace SmallDemoManager.GUI
{
    /// <summary>
    /// Main window of the application. Controls the UI, demo parsing, and bitfield calculation.
    /// </summary>
    public partial class NewGUI : MaterialForm
    {
        // =====================================================
        // ====================== FIELDS =======================
        // =====================================================

        // Parser and player snapshots
        private CsDemoParser? _demo = null;
        private PlayerSnapshot[]? _snapshot = null;

        // Bitfields for voice indices
        private int[] _teamA_Array = new int[5];
        private int[] _teamB_Array = new int[5];

        // Tooltip for copy button
        private readonly ToolTip _copyToolTip = new ToolTip();

        // Recursion protection for "Select All"
        private bool _isSyncingSelectAll = false;
        private bool _isUpdatingCheckboxes = false;

        // Paths for demo and audio folders
        private string? _csDemoFolderPath = null;
        private string? _audioFolderPath = null;

        // Hashes for file comparison
        private byte[]? _sourceHash = null;
        private byte[]? _destinationHash = null;

        // Static links to player profile pages
        private static string _steamProfileLink = "http://steamcommunity.com/profiles/";
        private static string _cswatchProfileLink = "https://cswatch.in/player/";
        private static string _csStatsProfileLink = "https://csstats.gg/player/";
        private static string _leetifyProfileLink = "https://leetify.com/app/profile/";

        // Static links to other pages
        private const string _KOFIURL = "https://ko-fi.com/pythaeus";
        private const string _GITHUBURL = "https://github.com/pythaeusone";

        // Other Globals
        private string _mapName = "no Mapname!";
        private string _duration = "00:00:00";
        private string _hostName = "No Hostname";
        private ulong? _matchId = null;
        private int _teamAScore = 0;
        private int _teamBScore = 0;
        private bool _voicePlayerOpen = false;
        private List<AudioEntry> _audioEntries;
        private float _fontScaleFactor = 1f;
        private const string _CHECKUPDATEKEY = "CheckUpdate";
        private const string _THEMEMODEKEY = "IsDarkMode";
        private string _selectedPlayerVoicePlayer = "NoPlayerName";
        private bool _readDemoOnStartUp = false;

        // GUI version
        private const string _GUIVERSIONNR = GlobalVersionInfo.GUI_VERSION;

        // Material stuff
        private MaterialSkinManager _materialSkinManager;
        private int _colorSchemeIndex;
        public Color SnackBarBackColor { get; private set; }
        public Color SnackBarForeColor { get; private set; }
        private MaterialLabel? _matchDetailsSummaryLabel = null;
        private System.Windows.Forms.Panel? _matchRoomSurface = null;
        private System.Windows.Forms.Panel? _matchRoomLoadPanel = null;
        private System.Windows.Forms.Panel? _matchRoomCommandPanel = null;
        private System.Windows.Forms.Panel? _matchRoomProgressFill = null;
        private FlowLayoutPanel? _matchRoomTeamAFlow = null;
        private FlowLayoutPanel? _matchRoomTeamBFlow = null;
        private FlowLayoutPanel? _matchRoomCenterFlow = null;
        private System.Windows.Forms.Label? _matchRoomVoiceHelpLabel = null;
        private System.Windows.Forms.Label? _matchRoomTeamAName = null;
        private System.Windows.Forms.Label? _matchRoomTeamBName = null;
        private System.Windows.Forms.Label? _matchRoomTeamAMeta = null;
        private System.Windows.Forms.Label? _matchRoomTeamBMeta = null;
        private System.Windows.Forms.Label? _matchRoomScoreA = null;
        private System.Windows.Forms.Label? _matchRoomScoreB = null;
        private System.Windows.Forms.Label? _matchRoomStateBadge = null;
        private System.Windows.Forms.Label? _matchRoomMapMeta = null;
        private int _matchRoomProgress = 50;
        private readonly HashSet<int> _selectedVoiceUserIds = new();
        private readonly Dictionary<int, System.Windows.Forms.Panel> _matchRoomPlayerCards = new();

        private static readonly Color KarasuBg = Color.FromArgb(12, 11, 9);
        private static readonly Color KarasuBgAlt = Color.FromArgb(17, 16, 9);
        private static readonly Color KarasuSurface = Color.FromArgb(23, 22, 19);
        private static readonly Color KarasuCard = Color.FromArgb(28, 26, 23);
        private static readonly Color KarasuForeground = Color.FromArgb(244, 243, 239);
        private static readonly Color KarasuMuted = Color.FromArgb(122, 120, 116);
        private static readonly Color KarasuAccent = Color.FromArgb(14, 165, 201);
        private static readonly Color KarasuBorder = Color.FromArgb(47, 45, 40);
        private static readonly Color KarasuBorderStrong = Color.FromArgb(62, 59, 52);
        private static readonly Color KarasuGreen = Color.FromArgb(34, 197, 94);
        private static readonly Color KarasuRed = Color.FromArgb(239, 68, 68);
        private static readonly Color KarasuOrange = Color.FromArgb(249, 115, 22);

        // =====================================================
        // ================= CONSTRUCTOR & INIT ================
        // =====================================================

        /// <summary>
        /// Initializes the form, loads theme, registers handlers, and populates initial UI content.
        /// </summary>
        public NewGUI()
        {
            InitializeComponent();
            ConfigureStartupWindow();
            InitializeRuntimeUi();

            CheckSettings();
            FirstAppStart();
            LoadListViewSettings();
            InitializeEventHandlers();

            // Load Patchnotes if present
            //if (File.Exists("PatchNotes.rtf"))
            //    RTB_PatchNotes.LoadFile("PatchNotes.rtf", RichTextBoxStreamType.RichText);
            var bytes = Properties.Resources.PatchNotes; // <-- Name so wie in Resources.resx
            using var ms = new MemoryStream(bytes);
            RTB_PatchNotes.LoadFile(ms, RichTextBoxStreamType.RichText);

            LoadSavedAudioFiles();
        }

        private void ConfigureStartupWindow()
        {
            MaximumSize = Size.Empty;
            MinimumSize = new Size(1000, 600);
            MaximizeBox = true;
            Sizable = true;

            var workingArea = Screen.FromControl(this).WorkingArea;
            int width = Math.Min(1280, Math.Max(MinimumSize.Width, workingArea.Width - 80));
            int height = Math.Min(760, Math.Max(MinimumSize.Height, workingArea.Height - 80));
            ClientSize = new Size(width, height);
        }

        /// <summary>
        /// Handles the form's Shown event to perform startup initialization.
        /// If demo mode is enabled on startup and a valid demo folder path exists,
        /// prepares the application for startup using the specified demo folder.
        /// </summary>
        /// <param name="sender">The source of the event</param>
        /// <param name="e">An EventArgs object that contains no event data</param>
        private void NewGUI_Shown(object sender, EventArgs e)
        {
            if (_readDemoOnStartUp && _csDemoFolderPath != null)
            {
                PrepareStart(_csDemoFolderPath);
            }
        }

        /// <summary>
        /// One-time initialization for first app start (window title, tab locks, etc.).
        /// </summary>
        private void FirstAppStart()
        {
            // Small fix: enable Drag&Drop for the textbox and its inner control.
            TB_FilePath.AllowDrop = true;
            TB_FilePath.Controls[0].AllowDrop = true;

            //this.Text = "Demo Manager by Pythaeus v." + _GUIVERSIONNR;
            //this.Text = "Small Demo Manager v." + _GUIVERSIONNR;
            LBL_AboutVersion.Text = "v." + _GUIVERSIONNR;

            BTN_ExtractAudio.Enabled = false;
            TabLockHelper.UnlockTab(tabPageMatchResults);
            DrawerNonClickTabPage = TabLockHelper.GetLockedTabs();
        }

        /// <summary>
        /// Wires UI events that are not using designer event hooks.
        /// </summary>
        private void InitializeEventHandlers()
        {
            Btn_CopyToClipboard.Click += (s, e) =>
            {
                System.Windows.Clipboard.SetText(TB_ConsoleCommand.Text);
                ShowCopyTooltip();
                this.ActiveControl = null;
            };

            Btn_MoveToCS2.Click += (s, e) => { MoveToCSFolder(); };
        }

        /// <summary>
        /// Creates small runtime-only UI elements that should not require designer churn.
        /// </summary>
        private void InitializeRuntimeUi()
        {
            _matchDetailsSummaryLabel = new MaterialLabel
            {
                Depth = 0,
                Font = new Font("Roboto Mono", 12F, FontStyle.Bold, GraphicsUnit.Pixel),
                FontType = MaterialSkinManager.FontType.Overline,
                Location = new Point(20, 203),
                MouseState = ReaLTaiizor.Helper.MaterialDrawHelper.MaterialMouseState.HOVER,
                Name = "LBL_MatchDetailsSummary",
                Size = new Size(895, 24),
                TabIndex = 9,
                Text = "Load a demo to see the match results.",
                TextAlign = ContentAlignment.MiddleCenter,
                UseMnemonic = false
            };

            tabPageMatchResults.Controls.Add(_matchDetailsSummaryLabel);
            _matchDetailsSummaryLabel.Visible = false;
            _matchDetailsSummaryLabel.BringToFront();

            InitializeMatchRoomLayout();
            ClearMatchRoom("Load a demo to see the match results.");
        }

        private void InitializeMatchRoomLayout()
        {
            tabPageMatchResults.Text = "Match Room";
            LBL_MatchDetailsPoints_TeamA.Visible = false;
            LBL_MatchDetailsPoints_TeamB.Visible = false;
            LBL_MatchDetails_TeamA.Visible = false;
            LBL_MatchDetails_TeamB.Visible = false;
            LV_MatchDetails_A.Visible = false;
            LV_MatchDetails_B.Visible = false;

            _matchRoomSurface = new System.Windows.Forms.Panel
            {
                Dock = DockStyle.Fill,
                BackColor = KarasuBg,
                Padding = new Padding(0)
            };

            var scoreboard = new System.Windows.Forms.Panel
            {
                Dock = DockStyle.Fill,
                BackColor = KarasuBg,
                Padding = new Padding(16, 8, 16, 0)
            };

            var scoreboardGrid = new TableLayoutPanel
            {
                Dock = DockStyle.Fill,
                BackColor = KarasuBg,
                ColumnCount = 3,
                RowCount = 1,
                Margin = new Padding(0),
                Padding = new Padding(0)
            };
            scoreboardGrid.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 33f));
            scoreboardGrid.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 34f));
            scoreboardGrid.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 33f));

            var teamAHeader = CreateHeaderTeamPanel(false);
            _matchRoomTeamAName = CreateMatchRoomLabel("Team A", 16, FontStyle.Bold, KarasuForeground, ContentAlignment.BottomLeft);
            _matchRoomTeamAMeta = CreateMatchRoomLabel("CT side", 9, FontStyle.Bold, KarasuMuted, ContentAlignment.TopLeft, true);
            teamAHeader.Controls.Add(_matchRoomTeamAName);
            teamAHeader.Controls.Add(_matchRoomTeamAMeta);

            var scorePanel = new System.Windows.Forms.Panel
            {
                Dock = DockStyle.Fill,
                BackColor = KarasuBg,
                Margin = new Padding(0)
            };
            _matchRoomScoreA = CreateMatchRoomLabel("0", 30, FontStyle.Bold, KarasuGreen, ContentAlignment.MiddleRight);
            _matchRoomScoreA.Location = new Point(18, 2);
            _matchRoomScoreA.Size = new Size(58, 38);
            _matchRoomScoreB = CreateMatchRoomLabel("0", 30, FontStyle.Bold, KarasuRed, ContentAlignment.MiddleLeft);
            _matchRoomScoreB.Location = new Point(220, 2);
            _matchRoomScoreB.Size = new Size(58, 38);
            _matchRoomStateBadge = CreateBadgeLabel("DEMO", KarasuOrange);
            _matchRoomStateBadge.Location = new Point(105, 6);
            _matchRoomStateBadge.Size = new Size(86, 20);
            _matchRoomMapMeta = CreateMatchRoomLabel("EU | 5v5 | Map TBD", 9, FontStyle.Bold, KarasuMuted, ContentAlignment.MiddleCenter, true);
            _matchRoomMapMeta.Location = new Point(62, 31);
            _matchRoomMapMeta.Size = new Size(175, 18);
            scorePanel.Controls.Add(_matchRoomScoreA);
            scorePanel.Controls.Add(_matchRoomScoreB);
            scorePanel.Controls.Add(_matchRoomStateBadge);
            scorePanel.Controls.Add(_matchRoomMapMeta);

            var teamBHeader = CreateHeaderTeamPanel(true);
            _matchRoomTeamBName = CreateMatchRoomLabel("Team B", 16, FontStyle.Bold, KarasuForeground, ContentAlignment.BottomRight);
            _matchRoomTeamBMeta = CreateMatchRoomLabel("T side", 9, FontStyle.Bold, KarasuMuted, ContentAlignment.TopRight, true);
            teamBHeader.Controls.Add(_matchRoomTeamBName);
            teamBHeader.Controls.Add(_matchRoomTeamBMeta);

            scoreboardGrid.Controls.Add(teamAHeader, 0, 0);
            scoreboardGrid.Controls.Add(scorePanel, 1, 0);
            scoreboardGrid.Controls.Add(teamBHeader, 2, 0);
            scoreboard.Controls.Add(scoreboardGrid);

            var progressTrack = new System.Windows.Forms.Panel
            {
                Dock = DockStyle.Fill,
                BackColor = KarasuSurface,
                Margin = new Padding(0)
            };
            _matchRoomProgressFill = new System.Windows.Forms.Panel
            {
                Dock = DockStyle.Left,
                BackColor = KarasuGreen,
                Width = 1
            };
            progressTrack.Controls.Add(_matchRoomProgressFill);
            progressTrack.Resize += (s, e) => UpdateMatchRoomProgressWidth();

            var bodyGrid = new TableLayoutPanel
            {
                Dock = DockStyle.Fill,
                BackColor = KarasuBg,
                ColumnCount = 3,
                RowCount = 1,
                Padding = new Padding(0),
                Margin = new Padding(0)
            };
            bodyGrid.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 35f));
            bodyGrid.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 30f));
            bodyGrid.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 35f));

            _matchRoomTeamAFlow = CreateMatchRoomColumn(false);
            _matchRoomCenterFlow = CreateMatchRoomColumn(false);
            _matchRoomTeamBFlow = CreateMatchRoomColumn(true);

            bodyGrid.Controls.Add(_matchRoomTeamAFlow, 0, 0);
            bodyGrid.Controls.Add(_matchRoomCenterFlow, 1, 0);
            bodyGrid.Controls.Add(_matchRoomTeamBFlow, 2, 0);

            _matchRoomLoadPanel = CreateMatchRoomLoadPanel();
            _matchRoomCommandPanel = CreateMatchRoomCommandPanel();

            var shell = new TableLayoutPanel
            {
                Dock = DockStyle.Fill,
                BackColor = KarasuBg,
                ColumnCount = 1,
                RowCount = 5,
                Padding = new Padding(0),
                Margin = new Padding(0)
            };
            shell.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100f));
            shell.RowStyles.Add(new RowStyle(SizeType.Absolute, 66f));
            shell.RowStyles.Add(new RowStyle(SizeType.Absolute, 74f));
            shell.RowStyles.Add(new RowStyle(SizeType.Absolute, 3f));
            shell.RowStyles.Add(new RowStyle(SizeType.Percent, 100f));
            shell.RowStyles.Add(new RowStyle(SizeType.Absolute, 78f));
            shell.Controls.Add(_matchRoomLoadPanel, 0, 0);
            shell.Controls.Add(scoreboard, 0, 1);
            shell.Controls.Add(progressTrack, 0, 2);
            shell.Controls.Add(bodyGrid, 0, 3);
            shell.Controls.Add(_matchRoomCommandPanel, 0, 4);

            _matchRoomSurface.Controls.Add(shell);
            tabPageMatchResults.Controls.Add(_matchRoomSurface);
            _matchRoomSurface.BringToFront();
            HideSeparateVoiceTab();
        }

        private System.Windows.Forms.Panel CreateMatchRoomLoadPanel()
        {
            var panel = new System.Windows.Forms.Panel
            {
                Dock = DockStyle.Fill,
                BackColor = KarasuBg,
                Padding = new Padding(16, 8, 16, 6),
                AllowDrop = true
            };
            panel.DragEnter += TB_FilePath_DragEnter;
            panel.DragDrop += TB_FilePath_DragDrop;

            TB_FilePath.Hint = "Drop a demo here to load it...";
            TB_FilePath.Margin = new Padding(0);
            TB_FilePath.TextAlign = HorizontalAlignment.Left;

            Btn_MoveToCS2.Text = "Move to CS2";
            Btn_MoveToCS2.Margin = new Padding(0);

            PBar_LoadDemo.Margin = new Padding(0);

            panel.Controls.Add(TB_FilePath);
            panel.Controls.Add(Btn_MoveToCS2);
            panel.Controls.Add(PBar_LoadDemo);
            panel.Resize += (s, e) => LayoutMatchRoomLoadPanel(panel);
            LayoutMatchRoomLoadPanel(panel);
            return panel;
        }

        private void LayoutMatchRoomLoadPanel(System.Windows.Forms.Panel panel)
        {
            int buttonWidth = 128;
            int gap = 12;
            int contentWidth = Math.Max(0, panel.ClientSize.Width - panel.Padding.Horizontal);
            int textWidth = Math.Max(260, contentWidth - buttonWidth - gap);

            TB_FilePath.Location = new Point(panel.Padding.Left, 8);
            TB_FilePath.Size = new Size(textWidth, 48);

            Btn_MoveToCS2.Location = new Point(panel.Padding.Left + textWidth + gap, 14);
            Btn_MoveToCS2.Size = new Size(buttonWidth, 36);

            PBar_LoadDemo.Location = new Point(panel.Padding.Left, panel.ClientSize.Height - 6);
            PBar_LoadDemo.Size = new Size(contentWidth, 4);
        }

        private System.Windows.Forms.Panel CreateMatchRoomCommandPanel()
        {
            var panel = new System.Windows.Forms.Panel
            {
                Dock = DockStyle.Fill,
                BackColor = KarasuBg,
                Padding = new Padding(16, 2, 16, 12),
                AllowDrop = true
            };
            panel.DragEnter += TB_FilePath_DragEnter;
            panel.DragDrop += TB_FilePath_DragDrop;

            _matchRoomVoiceHelpLabel = CreateMatchRoomLabel("Click player cards to choose who you want to hear.", 8.5F, FontStyle.Bold, KarasuMuted, ContentAlignment.MiddleLeft, true);
            _matchRoomVoiceHelpLabel.Location = new Point(16, 0);
            _matchRoomVoiceHelpLabel.Size = new Size(520, 20);

            TB_ConsoleCommand.Hint = "Pick players, then copy this command...";
            TB_ConsoleCommand.Margin = new Padding(0);
            TB_ConsoleCommand.TextAlign = HorizontalAlignment.Left;

            Btn_CopyToClipboard.Text = "Copy command";
            Btn_CopyToClipboard.Margin = new Padding(0);

            panel.Controls.Add(_matchRoomVoiceHelpLabel);
            panel.Controls.Add(TB_ConsoleCommand);
            panel.Controls.Add(Btn_CopyToClipboard);
            panel.Resize += (s, e) => LayoutMatchRoomCommandPanel(panel);
            LayoutMatchRoomCommandPanel(panel);
            UpdateVoiceCommandHelp();
            return panel;
        }

        private void LayoutMatchRoomCommandPanel(System.Windows.Forms.Panel panel)
        {
            int buttonWidth = 132;
            int gap = 12;
            int contentWidth = Math.Max(0, panel.ClientSize.Width - panel.Padding.Horizontal);
            int textWidth = Math.Max(260, contentWidth - buttonWidth - gap);

            if (_matchRoomVoiceHelpLabel != null)
            {
                _matchRoomVoiceHelpLabel.Location = new Point(panel.Padding.Left, 0);
                _matchRoomVoiceHelpLabel.Size = new Size(contentWidth, 20);
            }

            TB_ConsoleCommand.Location = new Point(panel.Padding.Left, 22);
            TB_ConsoleCommand.Size = new Size(textWidth, 48);

            Btn_CopyToClipboard.Location = new Point(panel.Padding.Left + textWidth + gap, 28);
            Btn_CopyToClipboard.Size = new Size(buttonWidth, 36);
        }

        private void HideSeparateVoiceTab()
        {
            TabLockHelper.UnlockTab(tabPageBitfieldCalc);
            if (materialTabControlMain.TabPages.Contains(tabPageBitfieldCalc))
                materialTabControlMain.TabPages.Remove(tabPageBitfieldCalc);
        }

        private static FlowLayoutPanel CreateHeaderTeamPanel(bool alignRight)
        {
            return new FlowLayoutPanel
            {
                Dock = DockStyle.Fill,
                BackColor = KarasuBg,
                FlowDirection = FlowDirection.TopDown,
                WrapContents = false,
                Padding = new Padding(0, 6, 0, 0),
                Margin = new Padding(0),
                RightToLeft = alignRight ? RightToLeft.Yes : RightToLeft.No
            };
        }

        private static FlowLayoutPanel CreateMatchRoomColumn(bool alignRight)
        {
            return new FlowLayoutPanel
            {
                Dock = DockStyle.Fill,
                AutoScroll = true,
                BackColor = KarasuBg,
                BorderStyle = BorderStyle.FixedSingle,
                FlowDirection = FlowDirection.TopDown,
                WrapContents = false,
                Padding = new Padding(10, 10, 10, 10),
                Margin = new Padding(0),
                RightToLeft = alignRight ? RightToLeft.Yes : RightToLeft.No
            };
        }

        private static System.Windows.Forms.Label CreateMatchRoomLabel(string text, float size, FontStyle style, Color color, ContentAlignment alignment, bool mono = false)
        {
            return new System.Windows.Forms.Label
            {
                AutoSize = false,
                BackColor = Color.Transparent,
                Font = new Font(mono ? "Consolas" : "Segoe UI", size, style, GraphicsUnit.Point),
                ForeColor = color,
                Size = new Size(280, 24),
                Text = text,
                TextAlign = alignment,
                UseMnemonic = false
            };
        }

        private static System.Windows.Forms.Label CreateBadgeLabel(string text, Color color)
        {
            return new System.Windows.Forms.Label
            {
                AutoSize = false,
                BackColor = Color.FromArgb(37, 31, 25),
                BorderStyle = BorderStyle.FixedSingle,
                Font = new Font("Consolas", 8F, FontStyle.Bold, GraphicsUnit.Point),
                ForeColor = color,
                Text = text.ToUpperInvariant(),
                TextAlign = ContentAlignment.MiddleCenter,
                UseMnemonic = false
            };
        }

        private void ClearMatchRoom(string message)
        {
            _matchRoomTeamAFlow?.Controls.Clear();
            _matchRoomTeamBFlow?.Controls.Clear();
            _matchRoomCenterFlow?.Controls.Clear();
            _matchRoomPlayerCards.Clear();

            if (_matchRoomTeamAName != null) _matchRoomTeamAName.Text = "Team A";
            if (_matchRoomTeamBName != null) _matchRoomTeamBName.Text = "Team B";
            if (_matchRoomTeamAMeta != null) _matchRoomTeamAMeta.Text = "CT SIDE";
            if (_matchRoomTeamBMeta != null) _matchRoomTeamBMeta.Text = "T SIDE";
            if (_matchRoomScoreA != null) _matchRoomScoreA.Text = "0";
            if (_matchRoomScoreB != null) _matchRoomScoreB.Text = "0";
            if (_matchRoomStateBadge != null) _matchRoomStateBadge.Text = "DEMO";
            if (_matchRoomMapMeta != null) _matchRoomMapMeta.Text = "EU | 5v5 | Map TBD";

            SetMatchRoomProgress(50);

            if (_matchRoomCenterFlow != null)
            {
                AddMatchRoomSectionLabel(_matchRoomCenterFlow, "Match room", false);
                AddCenterCard("Status", new[] { message }, KarasuOrange);
            }
            UpdateVoiceCommandHelp();
        }

        private void RenderMatchRoom(List<PlayerSnapshot> ctPlayers, List<PlayerSnapshot> tPlayers, int ctScore, int tScore)
        {
            if (_matchRoomSurface == null || _matchRoomTeamAFlow == null || _matchRoomTeamBFlow == null || _matchRoomCenterFlow == null)
                return;

            string teamAName = ctPlayers.FirstOrDefault()?.TeamName ?? "Counter-Terrorists";
            string teamBName = tPlayers.FirstOrDefault()?.TeamName ?? "Terrorists";
            string mapName = FormatMatchRoomMapName(_mapName);

            if (_matchRoomTeamAName != null) _matchRoomTeamAName.Text = TrimForUi(teamAName, 28);
            if (_matchRoomTeamBName != null) _matchRoomTeamBName.Text = TrimForUi(teamBName, 28);
            if (_matchRoomTeamAMeta != null) _matchRoomTeamAMeta.Text = $"CT SIDE | {ctPlayers.Count} PLAYERS";
            if (_matchRoomTeamBMeta != null) _matchRoomTeamBMeta.Text = $"T SIDE | {tPlayers.Count} PLAYERS";
            if (_matchRoomScoreA != null) _matchRoomScoreA.Text = ctScore.ToString();
            if (_matchRoomScoreB != null) _matchRoomScoreB.Text = tScore.ToString();
            if (_matchRoomStateBadge != null) _matchRoomStateBadge.Text = DemoHasTeamVoice() ? "VOICE" : "DEMO";
            if (_matchRoomMapMeta != null) _matchRoomMapMeta.Text = $"EU | 5v5 | {mapName}";

            SetMatchRoomProgress(ctScore + tScore > 0 ? (int)Math.Round((double)ctScore * 100 / (ctScore + tScore)) : 50);

            _matchRoomTeamAFlow.Controls.Clear();
            _matchRoomTeamBFlow.Controls.Clear();
            _matchRoomCenterFlow.Controls.Clear();
            _matchRoomPlayerCards.Clear();

            AddMatchRoomSectionLabel(_matchRoomTeamAFlow, teamAName, false);
            foreach (var player in ctPlayers.OrderByDescending(p => p.Score).ThenByDescending(p => p.Kills))
            {
                _matchRoomTeamAFlow.Controls.Add(CreatePlayerCard(player, false, _matchRoomTeamAFlow));
            }

            AddMatchRoomSectionLabel(_matchRoomCenterFlow, "What to do next", false);
            AddCenterCard("Demo loaded", new[]
            {
                $"Map: {mapName}",
                $"Time: {_duration}",
                $"Score: {ctScore} - {tScore}"
            }, KarasuGreen);
            AddCenterCard("Players found", new[]
            {
                $"CT side: {ctPlayers.Count} players",
                $"T side: {tPlayers.Count} players",
                DemoHasTeamVoice() ? "Click cards to choose who you hear" : "Stats are available on the cards"
            }, _matchId.HasValue ? KarasuAccent : KarasuOrange);
            AddCenterCard("Round stats", new[]
            {
                $"Kills: {ctPlayers.Concat(tPlayers).Sum(p => p.Kills)}",
                $"Damage: {ctPlayers.Concat(tPlayers).Sum(p => p.Damage)}",
                $"Utility damage: {ctPlayers.Concat(tPlayers).Sum(p => p.UtilityDamage)}"
            }, KarasuAccent);
            AddCenterCard("Voice command", new[]
            {
                DemoHasTeamVoice() ? "Click players you want to hear" : "This demo has no team voice",
                DemoHasTeamVoice() ? "Selected players turn blue" : "Load a demo with voice instead",
                DemoHasTeamVoice() ? "Copy and paste the command below" : "Then click player cards to choose voices"
            }, DemoHasTeamVoice() ? KarasuGreen : KarasuRed);

            AddMatchRoomSectionLabel(_matchRoomTeamBFlow, teamBName, true);
            foreach (var player in tPlayers.OrderByDescending(p => p.Score).ThenByDescending(p => p.Kills))
            {
                _matchRoomTeamBFlow.Controls.Add(CreatePlayerCard(player, true, _matchRoomTeamBFlow));
            }

            UpdateVoiceCommandHelp();
        }

        private void AddMatchRoomSectionLabel(FlowLayoutPanel flow, string text, bool alignRight)
        {
            var label = CreateMatchRoomLabel(TrimForUi(text, 34), 8F, FontStyle.Bold, KarasuMuted, alignRight ? ContentAlignment.MiddleRight : ContentAlignment.MiddleLeft, true);
            label.Width = GetFlowContentWidth(flow, 210);
            label.Height = 18;
            label.Margin = new Padding(0, 0, 0, 8);
            flow.Controls.Add(label);
        }

        private void AddCenterCard(string title, IEnumerable<string> lines, Color dotColor)
        {
            if (_matchRoomCenterFlow == null)
                return;

            int width = GetFlowContentWidth(_matchRoomCenterFlow, 210);
            var card = CreateMatchRoomCard(width, 0);
            card.Padding = new Padding(10, 8, 10, 8);

            var dot = new System.Windows.Forms.Panel
            {
                BackColor = dotColor,
                Location = new Point(10, 13),
                Size = new Size(7, 7)
            };
            var titleLabel = CreateMatchRoomLabel(title, 9F, FontStyle.Bold, dotColor, ContentAlignment.MiddleLeft, true);
            titleLabel.Location = new Point(23, 6);
            titleLabel.Size = new Size(width - 36, 20);
            card.Controls.Add(dot);
            card.Controls.Add(titleLabel);

            int top = 30;
            foreach (var line in lines)
            {
                var lineLabel = CreateMatchRoomLabel(line, 8F, FontStyle.Regular, KarasuMuted, ContentAlignment.MiddleLeft, true);
                lineLabel.Location = new Point(10, top);
                lineLabel.Size = new Size(width - 20, 16);
                card.Controls.Add(lineLabel);
                top += 17;
            }

            card.Height = top + 6;
            _matchRoomCenterFlow.Controls.Add(card);
        }

        private System.Windows.Forms.Panel CreatePlayerCard(PlayerSnapshot player, bool flip, FlowLayoutPanel owner)
        {
            int width = GetFlowContentWidth(owner, 250);
            var card = CreateMatchRoomCard(width, 78);
            card.Padding = new Padding(8);

            string playerName = player.PlayerName ?? "Unknown Player";
            var avatar = CreateBadgeLabel(GetInitials(playerName), player.TeamNumber == 3 ? KarasuAccent : KarasuOrange);
            avatar.BackColor = Color.FromArgb(25, 36, 42);
            avatar.Font = new Font("Segoe UI", 9F, FontStyle.Bold, GraphicsUnit.Point);
            avatar.Location = flip ? new Point(width - 42, 8) : new Point(8, 8);
            avatar.Size = new Size(32, 32);

            int textLeft = flip ? 8 : 48;
            int textWidth = width - 96;
            var nameLabel = CreateMatchRoomLabel(TrimForUi(playerName, 24), 10F, FontStyle.Bold, KarasuForeground, flip ? ContentAlignment.MiddleRight : ContentAlignment.MiddleLeft);
            nameLabel.Location = new Point(textLeft, 6);
            nameLabel.Size = new Size(textWidth, 22);

            var metaLabel = CreateMatchRoomLabel(BuildPlayerMeta(player), 8F, FontStyle.Bold, KarasuMuted, flip ? ContentAlignment.MiddleRight : ContentAlignment.MiddleLeft, true);
            metaLabel.Location = new Point(textLeft, 28);
            metaLabel.Size = new Size(textWidth, 16);

            var sideBadge = CreateBadgeLabel(player.TeamNumber == 3 ? "CT" : "T", player.TeamNumber == 3 ? KarasuAccent : KarasuOrange);
            sideBadge.Location = flip ? new Point(8, 13) : new Point(width - 36, 13);
            sideBadge.Size = new Size(28, 18);

            var metrics = new FlowLayoutPanel
            {
                BackColor = Color.Transparent,
                FlowDirection = flip ? FlowDirection.RightToLeft : FlowDirection.LeftToRight,
                Location = new Point(8, 50),
                Size = new Size(width - 16, 22),
                WrapContents = false,
                RightToLeft = flip ? RightToLeft.Yes : RightToLeft.No
            };
            metrics.Controls.Add(CreateMetricPill("K", player.Kills.ToString(), KarasuForeground));
            metrics.Controls.Add(CreateMetricPill("D", player.Death.ToString(), KarasuForeground));
            metrics.Controls.Add(CreateMetricPill("A", player.Assists.ToString(), KarasuForeground));
            metrics.Controls.Add(CreateMetricPill("KD", player.Kd.ToString("F2"), KarasuGreen));
            metrics.Controls.Add(CreateMetricPill("DMG", player.Damage.ToString(), KarasuAccent));

            bool canSelectVoice = DemoHasTeamVoice() && IsValidVoiceUserId(player.UserId);
            card.Paint += (s, e) =>
            {
                if (!_selectedVoiceUserIds.Contains(player.UserId))
                    return;

                using var pen = new Pen(KarasuAccent, 2F);
                e.Graphics.DrawRectangle(pen, 1, 1, card.Width - 3, card.Height - 3);
            };

            card.Controls.Add(avatar);
            card.Controls.Add(nameLabel);
            card.Controls.Add(metaLabel);
            card.Controls.Add(sideBadge);
            card.Controls.Add(metrics);

            _matchRoomPlayerCards[player.UserId] = card;
            ApplyPlayerCardSelectionState(card, player.UserId);
            if (canSelectVoice)
                WirePlayerCardClick(card, () => ToggleVoicePlayer(player.UserId));

            return card;
        }

        private void WirePlayerCardClick(Control control, Action clickAction)
        {
            control.Cursor = Cursors.Hand;
            control.Click += (s, e) => clickAction();

            foreach (Control child in control.Controls)
            {
                WirePlayerCardClick(child, clickAction);
            }
        }

        private void ToggleVoicePlayer(int userId)
        {
            if (!DemoHasTeamVoice() || !IsValidVoiceUserId(userId))
                return;

            if (!_selectedVoiceUserIds.Add(userId))
                _selectedVoiceUserIds.Remove(userId);

            if (_matchRoomPlayerCards.TryGetValue(userId, out var card))
                ApplyPlayerCardSelectionState(card, userId);

            ChangeConsoleCommand();
        }

        private void ApplyPlayerCardSelectionState(System.Windows.Forms.Panel card, int userId)
        {
            bool selected = _selectedVoiceUserIds.Contains(userId);
            card.BackColor = selected ? Color.FromArgb(8, 54, 66) : KarasuCard;
            card.Invalidate();
        }

        private static System.Windows.Forms.Panel CreateMatchRoomCard(int width, int height)
        {
            return new System.Windows.Forms.Panel
            {
                BackColor = KarasuCard,
                BorderStyle = BorderStyle.FixedSingle,
                Margin = new Padding(0, 0, 0, 8),
                Size = new Size(width, height)
            };
        }

        private static System.Windows.Forms.Label CreateMetricPill(string label, string value, Color valueColor)
        {
            var pill = CreateMatchRoomLabel($"{label} {value}", 7.5F, FontStyle.Bold, valueColor, ContentAlignment.MiddleCenter, true);
            pill.BackColor = KarasuSurface;
            pill.BorderStyle = BorderStyle.FixedSingle;
            pill.Margin = new Padding(0, 0, 4, 0);
            pill.Size = new Size(label == "DMG" ? 56 : label == "KD" ? 46 : 38, 20);
            return pill;
        }

        private bool DemoHasTeamVoice()
        {
            return _hostName.Contains("SourceTV", StringComparison.OrdinalIgnoreCase);
        }

        private static bool IsValidVoiceUserId(int userId)
        {
            return userId >= 1 && userId <= 20;
        }

        private int GetSelectedVoiceBitField()
        {
            int voiceBitField = 0;
            foreach (int userId in _selectedVoiceUserIds)
            {
                if (IsValidVoiceUserId(userId))
                    voiceBitField += 1 << (userId - 1);
            }

            return voiceBitField;
        }

        private void UpdateVoiceCommandHelp()
        {
            if (_matchRoomVoiceHelpLabel == null)
                return;

            if (_snapshot == null)
            {
                _matchRoomVoiceHelpLabel.Text = "Load a demo, then click player cards to choose voices.";
                _matchRoomVoiceHelpLabel.ForeColor = KarasuMuted;
                return;
            }

            if (!DemoHasTeamVoice())
            {
                _matchRoomVoiceHelpLabel.Text = "This demo does not include team voice.";
                _matchRoomVoiceHelpLabel.ForeColor = KarasuMuted;
                return;
            }

            int selectedCount = _selectedVoiceUserIds.Count;
            if (selectedCount == 0)
            {
                _matchRoomVoiceHelpLabel.Text = "Click player cards to choose who you want to hear.";
                _matchRoomVoiceHelpLabel.ForeColor = KarasuMuted;
                return;
            }

            _matchRoomVoiceHelpLabel.Text = selectedCount == 1
                ? "1 player selected. Copy the command and paste it into CS2."
                : $"{selectedCount} players selected. Copy the command and paste it into CS2.";
            _matchRoomVoiceHelpLabel.ForeColor = KarasuAccent;
        }

        private static int GetFlowContentWidth(FlowLayoutPanel owner, int minimumWidth)
        {
            int scrollbarWidth = owner.AutoScroll ? SystemInformation.VerticalScrollBarWidth : 0;
            int width = owner.ClientSize.Width - owner.Padding.Horizontal - scrollbarWidth - 8;
            return Math.Max(minimumWidth, width);
        }

        private void SetMatchRoomProgress(int progress)
        {
            _matchRoomProgress = Math.Clamp(progress, 0, 100);
            UpdateMatchRoomProgressWidth();
        }

        private void UpdateMatchRoomProgressWidth()
        {
            if (_matchRoomProgressFill?.Parent == null)
                return;

            _matchRoomProgressFill.Width = Math.Max(1, _matchRoomProgressFill.Parent.ClientSize.Width * _matchRoomProgress / 100);
        }

        private static string BuildPlayerMeta(PlayerSnapshot player)
        {
            return $"Score {player.Score}";
        }

        private static string FormatMatchRoomMapName(string mapName)
        {
            if (string.IsNullOrWhiteSpace(mapName))
                return "Map TBD";

            string cleaned = mapName.StartsWith("de_", StringComparison.OrdinalIgnoreCase) ? mapName[3..] : mapName;
            if (string.IsNullOrWhiteSpace(cleaned))
                return "Map TBD";

            return char.ToUpperInvariant(cleaned[0]) + cleaned[1..];
        }

        private static string GetInitials(string text)
        {
            var parts = text.Split(' ', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
            if (parts.Length >= 2)
                return string.Concat(parts[0][0], parts[1][0]).ToUpperInvariant();

            return text.Length >= 2 ? text[..2].ToUpperInvariant() : text.ToUpperInvariant();
        }

        private static string TrimForUi(string text, int maxLength)
        {
            if (text.Length <= maxLength)
                return text;

            return text[..Math.Max(0, maxLength - 1)] + "...";
        }

        private void CheckSettings()
        {
            LoadTheme(true);
            JsonClass.WriteJson(_THEMEMODEKEY, true);

            // Check if shell integration is enabled and update the settings checkbox accordingly.
            if (AddShellContextMenu.ValidateShellIntegration(this))
                CB_AddToShellContectMenu.Checked = true;

            // Load configured paths (demo folder, saved voice files).
            GetPathFromConfig();
        }

        /// <summary>
        /// Reads file paths (e.g., CS2 demo and saved audio folder) from the configuration
        /// and updates the settings UI fields if valid paths are found.
        /// </summary>
        private void GetPathFromConfig()
        {
            // Read paths from configuration
            var cs2DemoFolderPath = PathPicker.ReadPath(this, "CS2DemoPath");
            var savedVoiceFilesPath = PathPicker.ReadPath(this, "SavedVoiceFilesPath");

            // Update text boxes in the settings UI if paths are available.
            if (!string.IsNullOrEmpty(cs2DemoFolderPath))
            {
                TB_SettingsDemoPath.Text = cs2DemoFolderPath;
                BTN_ChangeDemoPath.Text = "Change";
            }
            else
            {
                BTN_ChangeDemoPath.Text = "Add Path";
            }


            if (!string.IsNullOrEmpty(savedVoiceFilesPath))
            {
                TB_SettingsSavedAudioPath.Text = savedVoiceFilesPath;
                BTN_ChangeAudioSavePath.Text = "Change";
            }
            else
            {
                BTN_ChangeAudioSavePath.Text = "Add Path";
            }

        }


        // =====================================================
        // ================== THEME & UI HELPERS ===============
        // =====================================================

        /// <summary>
        /// Shows a tooltip when copying to clipboard.
        /// </summary>
        private void ShowCopyTooltip()
        {
            _copyToolTip.AutomaticDelay = 0;
            _copyToolTip.AutoPopDelay = 1000;
            _copyToolTip.InitialDelay = 0;
            _copyToolTip.ReshowDelay = 0;
            _copyToolTip.ShowAlways = true;
            var offset = new System.Drawing.Point(Btn_CopyToClipboard.Width / 2, -Btn_CopyToClipboard.Height / 2);
            _copyToolTip.Show("Copied!", Btn_CopyToClipboard, offset, 1000);
        }

        /// <summary>
        /// Applies the Material theme (light/dark) and updates Snackbar colors.
        /// </summary>
        public void LoadTheme(bool dark)
        {
            dark = true;

            // Initialize MaterialSkinManager
            _materialSkinManager = MaterialSkinManager.Instance;

            // Enforce backcolor on non-materialSkin components (must be set before AddFormToManage).
            _materialSkinManager.EnforceBackcolorOnAllComponents = true;

            // Bind this form to Material manager.
            _materialSkinManager.AddFormToManage(this);

            if (!dark)
            {
                _materialSkinManager.Theme = MaterialSkinManager.Themes.LIGHT;
                _materialSkinManager.ColorScheme = new MaterialColorScheme("#f4f3ef", "#d8d3c7", "#ebe7dd", "#0ea5c9", MaterialTextShade.DARK);
                MSwitch.Checked = false;

                // PictureBox Change Color
                PBoxShell.Image = Properties.Resources.iconShellB;
                PBoxTheme.Image = Properties.Resources.iconBrush;
                PBoxFolder.Image = Properties.Resources.iconFolderB;

            }
            else
            {
                _materialSkinManager.Theme = MaterialSkinManager.Themes.DARK;
                _materialSkinManager.ColorScheme = new MaterialColorScheme("#0f0e0b", "#171613", "#1c1a17", "#0ea5c9", MaterialTextShade.LIGHT);
                MSwitch.Checked = true;

                // PictureBox Change Color
                PBoxShell.Image = Properties.Resources.iconShellW;
                PBoxTheme.Image = Properties.Resources.iconBrushW;
                PBoxFolder.Image = Properties.Resources.iconFolderW;

            }

            ApplyKarasuPalette(dark);

            SnackBarBackColor = KarasuAccent;
            SnackBarForeColor = dark ? KarasuBg : Color.White;

            Invalidate();
            Refresh();
        }

        /// <summary>
        /// Applies KarasuCS2-inspired colors to non-MaterialSkin controls.
        /// </summary>
        private void ApplyKarasuPalette(bool dark)
        {
            Color bg = dark ? KarasuBg : KarasuForeground;
            Color bgAlt = dark ? KarasuBgAlt : Color.FromArgb(234, 231, 222);
            Color surface = dark ? KarasuSurface : Color.White;
            Color card = dark ? KarasuCard : Color.White;
            Color foreground = dark ? KarasuForeground : KarasuBg;
            Color muted = dark ? KarasuMuted : Color.FromArgb(92, 88, 82);
            Color border = dark ? KarasuBorder : Color.FromArgb(211, 206, 194);

            BackColor = bg;
            materialTabControlMain.BackColor = bg;

            foreach (System.Windows.Forms.TabPage tabPage in materialTabControlMain.TabPages)
            {
                tabPage.BackColor = bg;
            }

            foreach (var materialCard in new[] { CardWelcome, CardShell, CardPath, CardAboutMe, CardSupport, CardAboutB, CardHowTo })
            {
                materialCard.BackColor = card;
                materialCard.ForeColor = foreground;
            }

            foreach (var listView in new[] { LV_TeamA, LV_TeamB, LV_MatchDetails_A, LV_MatchDetails_B })
            {
                listView.BackColor = surface;
                listView.ForeColor = foreground;
                listView.BorderStyle = BorderStyle.FixedSingle;
            }

            foreach (var listBox in new[] { LB_PlayerListAudio, LB_PlayerAudios, LB_PlayerSavedAudios })
            {
                listBox.BackColor = surface;
                listBox.BorderColor = border;
                listBox.ForeColor = foreground;
            }

            foreach (var textBox in GetChildControls<MaterialTextBoxEdit>(this))
            {
                textBox.BackColor = surface;
                textBox.ForeColor = foreground;
            }

            foreach (var label in GetChildControls<MaterialLabel>(this))
            {
                label.BackColor = Color.Transparent;
                label.ForeColor = foreground;
            }

            foreach (var mutedLabel in new[] { LBL_Duration, _matchDetailsSummaryLabel })
            {
                if (mutedLabel != null)
                    mutedLabel.ForeColor = muted;
            }

            RTB_PatchNotes.BackColor = card;
            RTB_PatchNotes.ForeColor = foreground;
            tabPageAudioPlayer.BackColor = bg;
            tabPageSettings.BackColor = bg;
            tabPageAbout.BackColor = bgAlt;
            tabPageHowTo.BackColor = bg;
            PBar_LoadDemo.UseAccentColor = true;
            PBar_LoadAudio.UseAccentColor = true;
        }

        private static IEnumerable<T> GetChildControls<T>(Control parent) where T : Control
        {
            foreach (Control child in parent.Controls)
            {
                if (child is T typedChild)
                    yield return typedChild;

                foreach (var descendant in GetChildControls<T>(child))
                    yield return descendant;
            }
        }

        /// <summary>
        /// Configures initial ListView properties such as alignment and checkbox behavior.
        /// </summary>
        private void LoadListViewSettings()
        {
            columnHeader1.TextAlign = System.Windows.Forms.HorizontalAlignment.Center;
            columnHeader2.TextAlign = System.Windows.Forms.HorizontalAlignment.Center;
            columnHeader3.TextAlign = System.Windows.Forms.HorizontalAlignment.Center;
            columnHeader4.TextAlign = System.Windows.Forms.HorizontalAlignment.Center;
            columnHeader5.TextAlign = System.Windows.Forms.HorizontalAlignment.Center;
            columnHeader6.TextAlign = System.Windows.Forms.HorizontalAlignment.Center;

            LV_TeamA.OwnerDraw = false;
            LV_TeamB.OwnerDraw = false;

            LV_TeamA.CheckBoxes = true;
            LV_TeamB.CheckBoxes = true;

            ConfigureMatchDetailsListView(LV_MatchDetails_A);
            ConfigureMatchDetailsListView(LV_MatchDetails_B);
        }

        private static void ConfigureMatchDetailsListView(MaterialListView listView)
        {
            listView.Columns.Clear();
            listView.AutoSizeTable = false;
            listView.Scrollable = false;

            AddColumn(listView, "Player", 195, HorizontalAlignment.Left);
            AddColumn(listView, "K", 45, HorizontalAlignment.Center);
            AddColumn(listView, "D", 45, HorizontalAlignment.Center);
            AddColumn(listView, "A", 45, HorizontalAlignment.Center);
            AddColumn(listView, "K/D", 55, HorizontalAlignment.Center);
            AddColumn(listView, "HS", 45, HorizontalAlignment.Center);
            AddColumn(listView, "HS%", 55, HorizontalAlignment.Center);
            AddColumn(listView, "Score", 55, HorizontalAlignment.Center);
            AddColumn(listView, "MVP", 50, HorizontalAlignment.Center);
            AddColumn(listView, "3K", 40, HorizontalAlignment.Center);
            AddColumn(listView, "4K", 40, HorizontalAlignment.Center);
            AddColumn(listView, "5K", 40, HorizontalAlignment.Center);
            AddColumn(listView, "UD", 60, HorizontalAlignment.Center);
            AddColumn(listView, "EF", 55, HorizontalAlignment.Center);
            AddColumn(listView, "DMG", 70, HorizontalAlignment.Center);
        }

        private static void AddColumn(ListView listView, string text, int width, HorizontalAlignment alignment)
        {
            listView.Columns.Add(new ColumnHeader
            {
                Text = text,
                Width = width,
                TextAlign = alignment
            });
        }

        // =====================================================
        // ================= FILE / PATH ACTIONS ===============
        // =====================================================

        /// <summary>
        /// Moves the demo file to the CS folder and verifies the file hash. Updates UI accordingly.
        /// </summary>
        private void MoveToCSFolder()
        {
            string? movedFullFilePath = null;
            string? sourceFile = null;
            _sourceHash = null; _destinationHash = null;
            TabLockHelper.LockTab(tabPageMatchResults);
            TabLockHelper.LockTab(tabPageAudioPlayer);
            DrawerNonClickTabPage = TabLockHelper.GetLockedTabs();

            if (string.IsNullOrWhiteSpace(_csDemoFolderPath) || !Directory.Exists(_csDemoFolderPath))
            {
                var title = "Select the CS2 Demo folder";
                var pathKey = "CS2DemoPath";

                _csDemoFolderPath = PathPicker.GetPath(this, title, pathKey);
                if (!Directory.Exists(_csDemoFolderPath))
                {
                    PathPicker.EnsurePathConfigured(this, title, pathKey);
                    _csDemoFolderPath = PathPicker.GetPath(this, title, pathKey);
                }
                GetPathFromConfig();
            }

            if (!string.IsNullOrWhiteSpace(_csDemoFolderPath) && Directory.Exists(_csDemoFolderPath))
            {
                sourceFile = TB_FilePath.Text;
                if (!string.IsNullOrWhiteSpace(sourceFile))
                {
                    try
                    {
                        _sourceHash = sourceFile.ComputeFileHash();
                        movedFullFilePath = sourceFile.MoveAndRenameFile(_csDemoFolderPath, this);
                    }
                    catch
                    {
                        movedFullFilePath = null;
                    }
                }
            }

            if (!string.IsNullOrWhiteSpace(movedFullFilePath))
            {
                _destinationHash = movedFullFilePath.ComputeFileHash();
                if (_sourceHash != null && _destinationHash != null)
                {
                    if (_sourceHash.HashesAreEqual(_destinationHash))
                    {
                        TB_FilePath.Text = movedFullFilePath;
                        ReadDemoFile(movedFullFilePath);
                        MaterialUiHelper.ShowSnack(this, "File was moved successfully.", false);
                    }
                    else
                    {
                        MaterialUiHelper.ShowSnack(this, "Error while moving the file!", true);
                    }
                }
            }
            else
            {
                MaterialUiHelper.ShowSnack(this, "File move was canceled or failed..", true);
                TabLockHelper.UnlockTab(tabPageMatchResults);
                TabLockHelper.UnlockTab(tabPageAudioPlayer);
                DrawerNonClickTabPage = TabLockHelper.GetLockedTabs();
                BTN_ExtractAudio.Enabled = true;
            }
        }

        // =====================================================
        // ===================== DEMO FLOW =====================
        // =====================================================

        /// <summary>
        /// Sets the demo file on startup if the file exists and is a .dem file.
        /// </summary>
        public void SetDemoFileOnStartup(string filePath)
        {
            if (!File.Exists(filePath) || Path.GetExtension(filePath).ToLower() != ".dem") return;
            _csDemoFolderPath = filePath;
            _readDemoOnStartUp = true;
            if (materialTabControlMain.TabPages.Count > 1)
                materialTabControlMain.SelectedIndex = 1;
        }

        /// <summary>
        /// Prepares the UI and state for loading a new demo file and triggers parsing.
        /// </summary>
        private async void PrepareStart(string demoPath)
        {
            PBar_LoadDemo.Value = 0;

            Btn_CopyToClipboard.Enabled = false;
            Btn_MoveToCS2.Enabled = false;
            BTN_ExtractAudio.Enabled = false;
            _snapshot = null;

            // Bitfield Page
            LBL_TeamA.Text = "Reading Team-A data..";
            LBL_TeamB.Text = "Reading Team-B data..";
            LV_TeamA.Items.Clear();
            LV_TeamB.Items.Clear();
            LBL_MapName.Text = "";
            LBL_Duration.Text = "";
            Array.Clear(_teamA_Array, 0, _teamA_Array.Length);
            Array.Clear(_teamB_Array, 0, _teamB_Array.Length);
            _selectedVoiceUserIds.Clear();

            // MatchDetails Page
            LBL_MatchDetailsPoints_TeamA.Text = "";
            LBL_MatchDetailsPoints_TeamB.Text = "";
            LBL_MatchDetails_TeamA.Text = "";
            LBL_MatchDetails_TeamB.Text = "";
            if (_matchDetailsSummaryLabel != null)
                _matchDetailsSummaryLabel.Text = "Reading demo metadata...";

            ClearMatchRoom("Reading demo metadata...");

            LV_MatchDetails_A.Items.Clear();
            LV_MatchDetails_B.Items.Clear();

            TB_FilePath.Text = demoPath;
            TB_ConsoleCommand.Text = "";
            _matchId = null;
            _mapName = "no Mapname!";
            _duration = "00:00:00";
            _hostName = "No Hostname";
            ChangeConsoleCommand();
            DrawerNonClickTabPage = TabLockHelper.GetLockedTabs();

            LB_PlayerListAudio.Clear();
            LB_PlayerAudios.Clear();

            RemoveAudioFolder();

            ReadDemoFile(demoPath);
        }

        /// <summary>
        /// Reads a CS demo file, collects player snapshots at match end, and updates UI state.
        /// </summary>
        private async void ReadDemoFile(string demoPath)
        {
            _demo = new CsDemoParser();
            _snapshot = null;
            _matchId = null;
            var tcs = new TaskCompletionSource<bool>();
            var collected = new List<PlayerSnapshot>();
            bool readingFinished = false;

            // Progress bar setup
            PBar_LoadDemo.Minimum = 0;
            PBar_LoadDemo.Maximum = 100;
            PBar_LoadDemo.Value = 0;

            _demo.UserMessageEvents.ServerRankRevealAll += msg =>
            {
                var reservation = msg.Reservation;
                if (reservation != null && reservation.HasMatchId && reservation.MatchId != 0)
                    _matchId = reservation.MatchId;
            };

            // Use RoundEnd but check explicitly for MatchEnded phase.
            _demo.Source1GameEvents.RoundEnd += (Source1RoundEndEvent e) =>
            {
                if (_demo.GameRules.CSGamePhase == CSGamePhase.MatchEnded)
                {
                    return;
                }                    

                collected.Clear();

                foreach (var team in new[] { _demo.TeamCounterTerrorist, _demo.TeamTerrorist })
                {
                    foreach (var p in team.CSPlayerControllers)
                    {
                        if (string.IsNullOrWhiteSpace(p.PlayerName))
                            continue;

                        var matchStats = p.ActionTrackingServices?.MatchStats ?? new CSMatchStats();

                        // skip spectators/admins/bots
                        if (p.PlayerInfo?.Fakeplayer == true)
                            continue;
                        if (matchStats.LiveTime == 0 || (matchStats.Kills + matchStats.Deaths + matchStats.Assists + matchStats.Damage) == 0)
                            continue;


                        double kd = matchStats.Deaths > 0 ? (double)matchStats.Kills / matchStats.Deaths : matchStats.Kills;
                        double hsPercent = matchStats.Kills > 0 ? (double)matchStats.HeadShotKills * 100 / matchStats.Kills : 0;

                        collected.Add(new PlayerSnapshot
                        {
                            UserId = (int)p.EntityIndex.Value,
                            PlayerName = p.PlayerName!,
                            TeamNumber = (int)p.CSTeamNum,
                            TeamName = p.Team.ClanTeamname.IsNullOrEmptyOrWhiteSpace() ? "No Team name.." : p.Team.ClanTeamname,
                            PlayerSteamID = p.PlayerInfo?.Steamid ?? p.SteamID,
                            Kills = matchStats.Kills,
                            Death = matchStats.Deaths,
                            Assists = matchStats.Assists,
                            HeadShotKill = matchStats.HeadShotKills,
                            HeadShotPerecent = hsPercent,
                            Kd = kd,
                            Damage = matchStats.Damage,
                            Score = p.Score,
                            UtilityDamage = matchStats.UtilityDamage,
                            EnemiesFlashed = matchStats.EnemiesFlashed,
                            MVP = p.MVPs,
                            ThreeK = matchStats.Enemy3Ks,
                            FourK = matchStats.Enemy4Ks,
                            FiveK = matchStats.Enemy5Ks,
                            EndScore = p.Team.Score
                        });
                    }
                }
                _snapshot = collected.ToArray();
                _mapName = _demo.ServerInfo?.MapName ?? "no Mapname!";
                _duration = TicksToTimeString(_demo.TickCount.Value);
                _hostName = _demo.ServerInfo?.HostName ?? "No Hostname";

                tcs.TrySetResult(true);
            };

            try
            {
                using var stream = new FileStream(demoPath, FileMode.Open, FileAccess.Read, FileShare.Read, 4096 * 1024);
                var reader = DemoFileReader.Create(_demo, stream);

                var progressTask = Task.Run(async () =>
                {
                    while (!readingFinished)
                    {
                        double ratio = (double)stream.Position / stream.Length;
                        int percent = (int)(ratio * 100);
                        if (percent >= 0 && percent <= 100)
                        {
                            PBar_LoadDemo.Invoke((Action)(() =>
                            {
                                PBar_LoadDemo.Value = percent;
                            }));
                        }
                        await Task.Delay(100);
                    }
                });

                await reader.ReadAllAsync();
                readingFinished = true;
                await tcs.Task; // Wait until MatchEnded is detected
            }
            catch (Exception ex)
            {
                MaterialUiHelper.ShowLongMessageBox($"Error when reading the demo", $"{ex.Message}", MessageBoxButtons.OK);
                return;
            }
            finally
            {
                PBar_LoadDemo.Value = 100; // Ensure it's full at the end
            }

            LoadListViewData();
        }

        /// <summary>
        /// Fills all UI list views (bitfield and match details) with the parsed snapshot data.
        /// </summary>
        private void LoadListViewData()
        {
            if (_snapshot == null)
            {
                MaterialUiHelper.ShowMaterialMsgBox(this, "Error", "Error reading demo file!", "OK", false, "CANCLE");
                return;
            }

            // Bitfield tables
            var ctPlayers = _snapshot.Where(p => p.TeamNumber == 3).OrderBy(p => p.UserId).ToList();
            var tPlayers = _snapshot.Where(p => p.TeamNumber == 2).OrderBy(p => p.UserId).ToList();

            // Match details tables
            var ctDetailPlayer = _snapshot.Where(p => p.TeamNumber == 3).OrderByDescending(p => p.Kills).ToList();
            var tDetailPlayer = _snapshot.Where(p => p.TeamNumber == 2).OrderByDescending(p => p.Kills).ToList();

            LBL_TeamA.Text = ctPlayers.FirstOrDefault()?.TeamName ?? "Unknown";
            LBL_TeamB.Text = tPlayers.FirstOrDefault()?.TeamName ?? "Unknown";

            LBL_MatchDetails_TeamA.Text = ctPlayers.FirstOrDefault()?.TeamName ?? "Unknown";
            LBL_MatchDetails_TeamB.Text = tPlayers.FirstOrDefault()?.TeamName ?? "Unknown";

            var ctScore = ctPlayers.FirstOrDefault()?.EndScore ?? 0;
            var tScore = tPlayers.FirstOrDefault()?.EndScore ?? 0;
            _teamAScore = ctScore;
            _teamBScore = tScore;
            LBL_MatchDetailsPoints_TeamA.Text = FormatTeamResult(ctScore, tScore);
            LBL_MatchDetailsPoints_TeamB.Text = FormatTeamResult(tScore, ctScore);

            if (_matchDetailsSummaryLabel != null)
                _matchDetailsSummaryLabel.Text = BuildMatchDetailsSummary();

            LBL_MapName.Text = _mapName;
            LBL_Duration.Text = _duration;

            FillListView(LV_TeamA, ctPlayers, true);
            FillListView(LV_TeamB, tPlayers, false);

            FillMatchDetailsListView(LV_MatchDetails_A, ctDetailPlayer);
            FillMatchDetailsListView(LV_MatchDetails_B, tDetailPlayer);
            RenderMatchRoom(ctPlayers, tPlayers, ctScore, tScore);

            CostumContextMenu.ConfigureContextMenuMainGrid(this, LV_TeamA, ctPlayers, _steamProfileLink, _cswatchProfileLink, _leetifyProfileLink, _csStatsProfileLink, true);
            CostumContextMenu.ConfigureContextMenuMainGrid(this, LV_TeamB, tPlayers, _steamProfileLink, _cswatchProfileLink, _leetifyProfileLink, _csStatsProfileLink, false);

            Btn_MoveToCS2.Enabled = true;
            if (!DemoHasTeamVoice())
            {
                TB_ConsoleCommand.Hint = "This demo does not include team voice...";
                TB_ConsoleCommand.Text = "";
                Btn_CopyToClipboard.Enabled = false;
                BTN_ExtractAudio.Enabled = false;
            }
            else
            {
                TB_ConsoleCommand.Hint = "Pick the players you want to hear, then copy the command...";
                TabLockHelper.UnlockTab(tabPageAudioPlayer);
                BTN_ExtractAudio.Enabled = true;
                Btn_CopyToClipboard.Enabled = false;
            }

            ChangeConsoleCommand();

            TabLockHelper.UnlockTab(tabPageMatchResults);
            DrawerNonClickTabPage = TabLockHelper.GetLockedTabs();

            ClearLVSelection();
        }

        private string BuildMatchDetailsSummary()
        {
            return $"Map {_mapName}  |  Time {_duration}  |  Score {_teamAScore} - {_teamBScore}";
        }

        private static string FormatTeamResult(int score, int opponentScore)
        {
            string result = score > opponentScore ? "Wins" : score < opponentScore ? "Loses" : "Draw";
            return $"{score} - {result}";
        }

        /// <summary>
        /// Populates a team ListView with player rows and an extra checkbox row (index 5) for select-all.
        /// </summary>
        private void FillListView(MaterialListView listView, List<PlayerSnapshot> players, bool left)
        {
            listView.Items.Clear(); // First clear old items

            var selectAll = "Select all";
            var empty = "";

            int counter = 0;
            foreach (var p in players)
            {
                string[] row = { "", left ? p.UserId.ToString() : p.PlayerName, left ? p.PlayerName : p.UserId.ToString() };
                ListViewItem item = new(row);
                listView.Items.Add(item);

                counter++;

                // After the 5th player → insert an empty line with only a checkbox (used for select-all)
                if (counter == 5)
                {
                    string[] emptyRow = { "", left ? empty : selectAll, left ? selectAll : empty };
                    ListViewItem emptyItem = new(emptyRow);
                    listView.Items.Add(emptyItem);
                }
            }
        }

        /// <summary>
        /// Populates the match-details ListView with per-player stats (kills, deaths, KD, HS%, MVP, etc.).
        /// </summary>
        private void FillMatchDetailsListView(MaterialListView listView, List<PlayerSnapshot> players)
        {
            listView.Items.Clear();

            foreach (var p in players)
            {
                string team = p.TeamNumber == 2 ? "T" : "CT";
                string kd = $"{p.Kd:F2}";
                string hs = $"{p.HeadShotPerecent:F2}";

                string[] row = {
                    p.PlayerName,
                    p.Kills.ToString(),
                    p.Death.ToString(),
                    p.Assists.ToString(),
                    kd,
                    p.HeadShotKill.ToString(),
                    hs,
                    p.Score.ToString(),
                    p.MVP.ToString(),
                    p.ThreeK.ToString(),
                    p.FourK.ToString(),
                    p.FiveK.ToString(),
                    p.UtilityDamage.ToString(),
                    p.EnemiesFlashed.ToString(),
                    p.Damage.ToString()
                };
                ListViewItem item = new(row);
                listView.Items.Add(item);
            }
        }

        // =====================================================
        // ======== BITFIELD / CHECKBOXES & CONSOLE CMD ========
        // =====================================================

        /// <summary>
        /// Auto-select logic for item check/uncheck on team list views, including select-all row.
        /// Updates bitfield arrays and regenerates the console command.
        /// </summary>
        private void CheckBoxAutoChecker(ItemCheckEventArgs e, MaterialListView lv, int[] _team, int leftRight)
        {
            if (_isUpdatingCheckboxes)
                return;

            if (e.Index <= 4)
            {
                if (e.NewValue == CheckState.Checked)
                {
                    int userId = GetVoiceUserId(lv.Items[e.Index].SubItems[leftRight].Text);
                    _team[e.Index] = GetPlayBitField(userId);
                    _selectedVoiceUserIds.Add(userId);
                }
                else if (e.NewValue == CheckState.Unchecked)
                {
                    int userId = GetVoiceUserId(lv.Items[e.Index].SubItems[leftRight].Text);
                    _team[e.Index] = 0;
                    _selectedVoiceUserIds.Remove(userId);
                }

                int countChecked = 0;
                for (int i = 0; i <= 4; i++)
                {
                    if (i == e.Index)
                    {
                        if (e.NewValue == CheckState.Checked)
                            countChecked++;
                    }
                    else
                    {
                        if (lv.Items[i].Checked)
                            countChecked++;
                    }
                }

                _isUpdatingCheckboxes = true;
                lv.Items[5].Checked = (countChecked == 5);
                _isUpdatingCheckboxes = false;
            }
            else if (e.Index == 5)
            {
                _isUpdatingCheckboxes = true;

                bool targetState = e.NewValue == CheckState.Checked;
                for (int i = 0; i <= 4; i++)
                {
                    lv.Items[i].Checked = targetState;

                    if (targetState)
                    {
                        int userId = GetVoiceUserId(lv.Items[i].SubItems[leftRight].Text);
                        _team[i] = GetPlayBitField(userId);
                        _selectedVoiceUserIds.Add(userId);
                    }
                    else
                    {
                        int userId = GetVoiceUserId(lv.Items[i].SubItems[leftRight].Text);
                        _team[i] = 0;
                        _selectedVoiceUserIds.Remove(userId);
                    }
                }

                _isUpdatingCheckboxes = false;
            }

            UpdateAllPlayerCardSelectionStates();
            ChangeConsoleCommand();
        }

        /// <summary>
        /// ItemCheck handler for Team A list view. Delegates to <see cref="CheckBoxAutoChecker"/>.
        /// </summary>
        private void LV_TeamA_ItemCheck(object sender, ItemCheckEventArgs e)
        {
            CheckBoxAutoChecker(e, LV_TeamA, _teamA_Array, 1);
        }

        /// <summary>
        /// ItemCheck handler for Team B list view. Delegates to <see cref="CheckBoxAutoChecker"/>.
        /// </summary>
        private void LV_TeamB_ItemCheck(object sender, ItemCheckEventArgs e)
        {
            CheckBoxAutoChecker(e, LV_TeamB, _teamB_Array, 2);
        }

        /// <summary>
        /// Recomputes the voice bitfield from both teams and updates the console command text.
        /// </summary>
        public void ChangeConsoleCommand()
        {
            int voiceBitField = GetSelectedVoiceBitField();

            if (voiceBitField != 0 && DemoHasTeamVoice())
            {
                TB_ConsoleCommand.Text =
                    $"tv_listen_voice_indices {voiceBitField}; tv_listen_voice_indices_h {voiceBitField}";
                Btn_CopyToClipboard.Enabled = true;
            }
            else
            {
                TB_ConsoleCommand.Text = "";
                Btn_CopyToClipboard.Enabled = false;
            }

            UpdateVoiceCommandHelp();
        }

        /// <summary>
        /// Calculates the bitfield value (1 &lt;&lt; (specId-1)) for a given player spec ID. Validates input range 1..20.
        /// </summary>
        private int GetVoiceUserId(object cellValue)
        {
            if (cellValue == null || !int.TryParse(cellValue.ToString(), out int specPlayerId))
                throw new ArgumentException("Invalid spec ID in the cell.");
            if (specPlayerId < 1 || specPlayerId > 20)
                throw new ArgumentOutOfRangeException(nameof(specPlayerId), "Spec-ID must be between 1 and 20.");
            return specPlayerId;
        }

        private int GetPlayBitField(object cellValue)
        {
            int specPlayerId = cellValue is int intValue ? intValue : GetVoiceUserId(cellValue);
            if (specPlayerId < 1 || specPlayerId > 20)
                throw new ArgumentOutOfRangeException(nameof(specPlayerId), "Spec-ID must be between 1 and 20.");
            return 1 << (specPlayerId - 1);
        }

        private void UpdateAllPlayerCardSelectionStates()
        {
            foreach (var entry in _matchRoomPlayerCards)
            {
                ApplyPlayerCardSelectionState(entry.Value, entry.Key);
            }
        }

        /// <summary>
        /// Selects or deselects all items in the provided list view. Uses a guard to prevent recursion.
        /// </summary>
        private void ToggleListViewItems(MaterialListView listView, bool isChecked)
        {
            _isSyncingSelectAll = true;
            foreach (ListViewItem item in listView.Items)
            {
                item.Checked = isChecked;
            }
            _isSyncingSelectAll = false;
        }

        // =====================================================
        // ===================== UTILITIES =====================
        // =====================================================

        /// <summary>
        /// Converts demo ticks to a time string (hh:mm:ss). Uses <paramref name="tickRate"/> to compute seconds.
        /// </summary>
        public static string TicksToTimeString(int ticks, double tickRate = 64.0)
        {
            if (tickRate <= 0)
                throw new ArgumentException("Tick rate must be greater than 0.");
            double totalSeconds = ticks / tickRate;
            TimeSpan duration = TimeSpan.FromSeconds(totalSeconds);
            return duration.ToString(@"hh\:mm\:ss");
        }

        // =====================================================
        // ===================== DRAG & DROP ===================
        // =====================================================

        /// <summary>
        /// DragEnter for the demo path textbox; shows copy effect only for file drops.
        /// </summary>
        private void TB_FilePath_DragEnter(object sender, System.Windows.Forms.DragEventArgs e)
        {
            e.Effect = e.Data.GetDataPresent(System.Windows.Forms.DataFormats.FileDrop)
                ? System.Windows.Forms.DragDropEffects.Copy
                : System.Windows.Forms.DragDropEffects.None;
        }

        /// <summary>
        /// DragDrop for the demo path textbox; accepts first .dem file and starts parsing.
        /// </summary>
        private void TB_FilePath_DragDrop(object sender, System.Windows.Forms.DragEventArgs e)
        {
            if (!e.Data.GetDataPresent(System.Windows.Forms.DataFormats.FileDrop)) return;
            string[] files = (string[])e.Data.GetData(System.Windows.Forms.DataFormats.FileDrop);

            foreach (string file in files)
            {
                if (Path.GetExtension(file).Equals(".dem", StringComparison.OrdinalIgnoreCase))
                {
                    PrepareStart(file);
                    return;
                }
            }
            MaterialUiHelper.ShowSnack(this, "Please only drop files with the extension .dem", true);
        }

        // =====================================================
        // =============== SELECTION & COLUMN SIZING ===========
        // =====================================================

        /// <summary>
        /// Clears both team list view selections.
        /// </summary>
        private void ClearLVSelection()
        {
            LV_TeamA.SelectedItems.Clear();
            LV_TeamB.SelectedItems.Clear();
        }

        /// <summary>
        /// When an item in Team B is selected, clear previous selections (keep single-select UX).
        /// </summary>
        private void LV_TeamB_ItemSelectionChanged(object sender, ListViewItemSelectionChangedEventArgs e)
        {
            if (e.IsSelected)
            {
                ClearLVSelection();
            }
        }

        /// <summary>
        /// When an item in Team A is selected, clear previous selections (keep single-select UX).
        /// </summary>
        private void LV_TeamA_ItemSelectionChanged(object sender, ListViewItemSelectionChangedEventArgs e)
        {
            if (e.IsSelected)
            {
                ClearLVSelection();
            }
        }

        /// <summary>
        /// Prevent column width changes for Match Details A.
        /// </summary>
        private void LV_MatchDetails_A_ColumnWidthChanging(object sender, ColumnWidthChangingEventArgs e)
        {
            e.Cancel = true;
            e.NewWidth = LV_MatchDetails_A.Columns[e.ColumnIndex].Width;
        }

        /// <summary>
        /// Prevent column width changes for Match Details B.
        /// </summary>
        private void LV_MatchDetails_B_ColumnWidthChanging(object sender, ColumnWidthChangingEventArgs e)
        {
            e.Cancel = true;
            e.NewWidth = LV_MatchDetails_B.Columns[e.ColumnIndex].Width;
        }

        /// <summary>
        /// Clear selection when mouse leaves Team A list view (avoids lingering highlights).
        /// </summary>
        private void LV_TeamA_MouseLeave(object sender, EventArgs e)
        {
            ClearLVSelection();
        }

        // =====================================================
        // ===================== AUDIO FLOW ====================
        // =====================================================

        /// <summary>
        /// Click handler to extract audio from the current demo. Shows progress and loads folders on success.
        /// </summary>
        private async void BTN_ExtractAudio_Click(object sender, EventArgs e)
        {
            PBar_LoadAudio.Value = 0;
            LB_PlayerListAudio.Clear();
            LB_PlayerAudios.Clear();
            BTN_ExtractAudio.Enabled = false;
            TabLockHelper.LockTab(tabPageBitfieldCalc);
            DrawerNonClickTabPage = TabLockHelper.GetLockedTabs();

            var progress = new Progress<float>(value =>
            {
                PBar_LoadAudio.Value = Math.Min(100, (int)(value * 100));
            });

            bool result = await Task.Run(() => AudioExtractor.ExtractAsync(TB_FilePath.Text, progress));

            BTN_ExtractAudio.Enabled = true;

            if (result)
            {
                MaterialUiHelper.ShowSnack(this, "Extraction complete!", false);
                LoadPlayerFolders();
            }
            else
                MaterialUiHelper.ShowSnack(this, "Error during extraction!", true);

            TabLockHelper.UnlockTab(tabPageBitfieldCalc);
            DrawerNonClickTabPage = TabLockHelper.GetLockedTabs();
        }

        /// <summary>
        /// Loads per-player audio folders for the current demo and populates the player list.
        /// </summary>
        private void LoadPlayerFolders()
        {
            string appPath = System.Windows.Forms.Application.StartupPath;
            string demoPath = TB_FilePath.Text;
            string? demoName = demoPath != null ? Path.GetFileNameWithoutExtension(demoPath) : null;
            string audioFolder = LocalAppDataFolder.EnsureSubDirectoryExists("Audio");
            _audioFolderPath = demoName != null ? Path.Combine(appPath, audioFolder, demoName) : null;

            var displayItems = PlayerAudioHelper.LoadPlayerFolders(_snapshot, _audioFolderPath);

            LB_PlayerListAudio.Items.Clear();

            if (displayItems.Count > 0)
            {
                foreach (var item in displayItems)
                {
                    LB_PlayerListAudio.Items.Add(new ReaLTaiizor.Child.Material.MaterialListBoxItem()
                    {
                        Text = item.DisplayName,
                        SecondaryText = item.SteamId
                    });
                }
            }
            else
            {
                MaterialUiHelper.ShowLongMessageBox("Error", "The 'Audio' folder could not be found.\\nTry running ‘Extract voice from demo’!");
            }
        }

        /// <summary>
        /// Removes focus visual leftovers after tab page change.
        /// </summary>
        private void materialTabControlMain_SelectedIndexChanged(object sender, EventArgs e)
        {
            PBar_LoadAudio.Focus();
            PBar_LoadDemo.Focus();
        }

        /// <summary>
        /// Loads voice entries for a specific player (by SteamID), sorts them, and populates the audio list.
        /// </summary>
        public void LoadVoiceData(string steamID)
        {
            LB_PlayerAudios.Clear();

            _audioEntries = AudioReadHelper.GetAudioEntries(steamID, _audioFolderPath ?? "");
            var sorted = _audioEntries.OrderBy(x => x.Round).ToList();
            _audioEntries = sorted;

            foreach (var entry in sorted)
            {
                LB_PlayerAudios.Items.Add(new ReaLTaiizor.Child.Material.MaterialListBoxItem()
                {
                    Text = $"Round {entry.Round}",
                    SecondaryText = "Demo-Time: " + entry.Time.ToString(@"hh\:mm\:ss") + " | Duration: " + $"{entry.DurationSeconds:F1} s"
                });

            }
        }

        /// <summary>
        /// Handles selection of a player in the audio list and loads corresponding voice files.
        /// </summary>
        private void LB_PlayerListAudio_SelectedIndexChanged(object sender, ReaLTaiizor.Child.Material.MaterialListBoxItem selectedItem)
        {
            // SteamId is in SecondaryText, DisplayName in Text
            string? steamId = selectedItem.SecondaryText;
            string? displayName = selectedItem.Text;

            string? demoPath = TB_FilePath.Text;
            string? demoName = demoPath != null ? Path.GetFileNameWithoutExtension(demoPath) : null;
            string? audioFolder = LocalAppDataFolder.EnsureSubDirectoryExists("Audio");
            string? folderPath = demoName != null
                ? Path.Combine(audioFolder, demoName, steamId ?? "00")
                : string.Empty;

            _selectedPlayerVoicePlayer = displayName;

            if (!string.IsNullOrEmpty(steamId) && Directory.Exists(folderPath))
            {
                LoadVoiceData(steamId);
            }
            else
            {
                MaterialUiHelper.ShowLongMessageBox("Error", $"The folder '{steamId}' could not be found in '{demoName}'.");
            }
        }

        /// <summary>
        /// Double-click on a voice entry plays the selected audio file.
        /// </summary>
        private void LB_PlayerAudios_DoubleClick(object sender, EventArgs e)
        {
            if (LB_PlayerAudios.SelectedItem is ReaLTaiizor.Child.Material.MaterialListBoxItem selectedItem)
            {
                // Determine index in the internal list
                int index = LB_PlayerAudios.SelectedIndex;

                if (index >= 0 && index < _audioEntries.Count)
                {
                    var entry = _audioEntries[index];
                    AudioReadHelper.PlayAudio(this, entry);
                }
            }
        }

        /// <summary>
        /// Removes the Audio folder under local app data (if present) to ensure a clean run.
        /// </summary>
        private void RemoveAudioFolder()
        {
            try
            {
                string audioFolderPath = LocalAppDataFolder.EnsureSubDirectoryExists("Audio");

                if (Directory.Exists(audioFolderPath))
                {
                    Directory.Delete(audioFolderPath, true);
                }
            }
            catch (Exception ex)
            {
                MaterialUiHelper.ShowLongMessageBox("Error deleting the folder", $"ex.Message");
            }
        }

        private void MSwitch_CheckStateChanged(object sender, EventArgs e)
        {
            MSwitch.Checked = true;
            JsonClass.WriteJson(_THEMEMODEKEY, true);
        }

        /// <summary>
        /// Loads saved audio files from the configured path and populates the saved list.
        /// </summary>
        private void LoadSavedAudioFiles()
        {
            LB_PlayerAudios.SelectedIndex = -1; // Clear selection from copied file.

            if (!PathPicker.HasPath(this, "SavedVoiceFilesPath"))
                return;

            string savedAudioPath = PathPicker.ReadPath(this, "SavedVoiceFilesPath");

            if (savedAudioPath != null && AudioFileManager.SavedVoiceFilesExists(savedAudioPath))
            {
                AudioFileManager.LoadAudioFiles(savedAudioPath);
                AudioFileManager.PopulateListBox(LB_PlayerSavedAudios);
            }

            if (string.IsNullOrEmpty(TB_SettingsSavedAudioPath.Text))
            {
                // Load configured paths (demo folder, saved voice files).
                GetPathFromConfig();
            }
        }

        /// <summary>
        /// Double-click on saved audio plays it.
        /// </summary>
        private void LB_PlayerSavedAudios_DoubleClick(object sender, EventArgs e)
        {
            if (LB_PlayerSavedAudios.SelectedItem is ReaLTaiizor.Child.Material.MaterialListBoxItem selectedItem)
            {
                // Determine index in the internal list
                int index = LB_PlayerSavedAudios.SelectedIndex;

                if (index >= 0 && index < SavedAudioFiles.Files.Count)
                {
                    var entry = SavedAudioFiles.Files[index];
                    AudioReadHelper.PlaySavedAudio(this, entry);
                }
            }
        }

        /// <summary>
        /// Handles right-click actions on the <c>LB_PlayerAudios</c> list box.  
        /// If no audio item is selected, a Material message box is shown to inform the user.  
        /// Otherwise, configures the context menu for copying the selected audio file.
        /// </summary>
        /// <param name="sender">The source of the event (the list box).</param>
        /// <param name="e">Mouse event data that provides button information and cursor position.</param>
        private void LB_PlayerAudios_MouseDown(object sender, MouseEventArgs e)
        {
            if (e.Button == MouseButtons.Right && LB_PlayerAudios.SelectedIndex < 0)
            {
                MaterialUiHelper.ShowMaterialMsgBox(this, "Info", "You must select an audio voice before attempting to copy it.", "OK", false, "Cancel");

                return;
            }

            CostumContextMenu.ConfigureContextMenuAudioFileCopy(this, LB_PlayerAudios, _audioEntries, _selectedPlayerVoicePlayer, TB_FilePath.Text, () => LoadSavedAudioFiles());
        }

        // =====================================================
        // ========= EVENT HANDLERS FOR MENUS AND UI ===========
        // =====================================================

        /// <summary>
        /// Opens a folder picker and stores the CS2 demo folder path.
        /// </summary>
        private void BTN_ChangeDemoPath_Click(object sender, EventArgs e)
        {
            var title = "Select the CS2 Demo folder";
            var pathKey = "CS2DemoPath";
            PathPicker.EnsurePathConfigured(this, title, pathKey);
            _csDemoFolderPath = PathPicker.GetPath(this, title, pathKey);

            GetPathFromConfig();
        }

        /// <summary>
        /// Opens a folder picker and stores the path where copied voice files should be saved.
        /// </summary>
        private void BTN_ChangeAudioSavePath_Click(object sender, EventArgs e)
        {
            var title = "Select the path where the copied voice files should be saved.";
            var pathKey = "SavedVoiceFilesPath";
            PathPicker.EnsurePathConfigured(this, title, pathKey);

            GetPathFromConfig();
            LoadSavedAudioFiles();
        }

        /// <summary>
        /// Adds or Removes the shell context menu integration.
        /// </summary>
        private void CB_AddToShellContectMenu_Click(object sender, EventArgs e)
        {
            if (CB_AddToShellContectMenu.Checked)
            {
                AddShellContextMenu.AddShellIntegration(this);
            }
            else
            {
                AddShellContextMenu.RemoveShellIntegration(this);
            }
        }

        /// <summary>
        /// Opens the Ko-fi support page in the user's default web browser.
        /// </summary>
        private void BTN_SupportKoFi_Click(object sender, EventArgs e)
        {
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            {
                FileName = _KOFIURL,
                UseShellExecute = true
            });
        }

        /// <summary>
        /// Opens the GitHub repository page in the user's default web browser.
        /// </summary>
        private void BTN_GitHub_Click(object sender, EventArgs e)
        {
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            {
                FileName = _GITHUBURL,
                UseShellExecute = true
            });
        }
    }
}
