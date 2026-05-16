using SmallDemoManager.GUI;

namespace SmallDemoManager
{
    internal static class Program
    {
        [STAThread]
        static void Main(string[] args)
        {
            try
            {
                Application.EnableVisualStyles();
                Application.SetHighDpiMode(HighDpiMode.PerMonitorV2);
                Application.SetCompatibleTextRenderingDefault(false);

                string? startupDemo = null;
                if (args.Length > 0 && File.Exists(args[0]) &&
                    Path.GetExtension(args[0]).Equals(".dem", StringComparison.OrdinalIgnoreCase))
                {
                    startupDemo = args[0];
                }

                Application.Run(new MainForm(startupDemo));
            }
            catch (Exception ex)
            {
                MessageBox.Show(ex.ToString(), "SmallDemoManager — fatal error",
                    MessageBoxButtons.OK, MessageBoxIcon.Error);
            }
        }
    }
}
