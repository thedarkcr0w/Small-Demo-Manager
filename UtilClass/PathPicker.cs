using Microsoft.WindowsAPICodePack.Dialogs;

namespace SmallDemoManager.UtilClass
{
    internal static class PathPicker
    {
        public static bool HasPath(string pathKey)
        {
            if (!JsonClass.KeyExists(pathKey)) return false;
            var stored = JsonClass.ReadJson<string>(pathKey);
            return !string.IsNullOrWhiteSpace(stored);
        }

        public static string? PickFolder(string title, string? initialDirectory = null)
        {
            using var dialog = new CommonOpenFileDialog
            {
                IsFolderPicker = true,
                Title = title,
                InitialDirectory = initialDirectory ?? AppDomain.CurrentDomain.BaseDirectory,
                EnsurePathExists = true,
                Multiselect = false
            };
            return dialog.ShowDialog() == CommonFileDialogResult.Ok ? dialog.FileName : null;
        }

        public static string? EnsurePathConfigured(string title, string pathKey)
        {
            var picked = PickFolder(title);
            if (picked != null && Directory.Exists(picked))
            {
                JsonClass.WriteJson(pathKey, picked);
                return picked;
            }
            return null;
        }

        public static string? GetPath(string title, string pathKey)
        {
            if (HasPath(pathKey)) return JsonClass.ReadJson<string>(pathKey);
            return EnsurePathConfigured(title, pathKey);
        }

        public static string? ReadPath(string pathKey)
        {
            return HasPath(pathKey) ? JsonClass.ReadJson<string>(pathKey) : null;
        }
    }
}
