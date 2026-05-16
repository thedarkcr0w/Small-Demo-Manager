using System.Text.Json.Serialization;

namespace SmallDemoManager.Bridge
{
    public sealed class WatchedFolderDto
    {
        public string Id { get; set; } = "";
        public string Label { get; set; } = "";
        public string Path { get; set; } = "";
        public int Count { get; set; }
        public string? Source { get; set; }
    }

    public sealed class PlayerDto
    {
        public string Name { get; set; } = "";
        public string Team { get; set; } = "A";
        public string SteamId { get; set; } = "";
        public int K { get; set; }
        public int A { get; set; }
        public int D { get; set; }
        public int Score { get; set; }
        public int Hs { get; set; }
        public int Adr { get; set; }
        public string Rating { get; set; } = "0.00";
        public bool Flagged { get; set; }
        public bool Favorite { get; set; }
        public int UserId { get; set; }
    }

    public sealed class DemoDto
    {
        public string Id { get; set; } = "";
        public string File { get; set; } = "";
        public string FullPath { get; set; } = "";
        public string FolderId { get; set; } = "";
        public string FolderPath { get; set; } = "";
        public string Map { get; set; } = "de_unknown";
        public string Date { get; set; } = "";
        public int Dur { get; set; }
        public double Size { get; set; }
        public int Tick { get; set; } = 64;
        public string Server { get; set; } = "";
        public string Source { get; set; } = "Other";
        public string T1 { get; set; } = "Team A";
        public string T2 { get; set; } = "Team B";
        public int S1 { get; set; }
        public int S2 { get; set; }
        public List<string> Tags { get; set; } = new();
        public bool Fav { get; set; }
        public string Note { get; set; } = "";
        public bool Parsed { get; set; }
        public string? ParseError { get; set; }
        public List<PlayerDto> Players1 { get; set; } = new();
        public List<PlayerDto> Players2 { get; set; } = new();
        public List<string> Rounds { get; set; } = new();
    }

    public sealed class VoiceClipDto
    {
        public string Id { get; set; } = "";
        public string Player { get; set; } = "";
        public string SteamId { get; set; } = "";
        public int Round { get; set; }
        public int DemoSec { get; set; }
        public double Dur { get; set; }
        public string Format { get; set; } = "wav";
        public string Path { get; set; } = "";
    }

    public sealed class InitialStateDto
    {
        public List<WatchedFolderDto> Folders { get; set; } = new();
        public List<DemoDto> Demos { get; set; } = new();
        public string? StartupDemo { get; set; }
        public string Cs2Path { get; set; } = "";
        public bool MoveOnImport { get; set; } = true;
        public bool AutoBackup { get; set; } = false;
        public string AppVersion { get; set; } = "";
    }

    public sealed class JsRequest
    {
        [JsonPropertyName("id")] public int Id { get; set; }
        [JsonPropertyName("type")] public string Type { get; set; } = "";
        [JsonPropertyName("payload")] public System.Text.Json.JsonElement Payload { get; set; }
    }
}
