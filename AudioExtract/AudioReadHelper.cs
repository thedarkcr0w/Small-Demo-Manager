using SmallDemoManager.HelperClass;
using NAudio.Wave;
using System.Text.RegularExpressions;

namespace SmallDemoManager.AudioExtract
{
    public static class AudioReadHelper
    {
        private static IWavePlayer? _waveOut;
        private static AudioFileReader? _audioFileReader;
        private static readonly object _lock = new();
        private static EventHandler<StoppedEventArgs>? _playbackStoppedHandler;
        public static event Action? PlaybackEnded;

        public static List<AudioEntry> GetAudioEntries(string subFolderName, string audioFolderPath)
        {
            var result = new List<AudioEntry>();
            string userFolderPath = Path.Combine(audioFolderPath, subFolderName);
            if (!Directory.Exists(userFolderPath)) return result;

            string[] files = Directory.EnumerateFiles(userFolderPath)
                .Where(f => f.EndsWith(".wav", StringComparison.OrdinalIgnoreCase)
                         || f.EndsWith(".mp3", StringComparison.OrdinalIgnoreCase)
                         || f.EndsWith(".flac", StringComparison.OrdinalIgnoreCase))
                .ToArray();
            var regex = new Regex(@"round_(\d+)_t_(\d+)s", RegexOptions.IgnoreCase);

            foreach (var file in files)
            {
                var fileName = Path.GetFileNameWithoutExtension(file);
                var match = regex.Match(fileName);
                if (!match.Success) continue;

                int round = int.Parse(match.Groups[1].Value);
                int seconds = int.Parse(match.Groups[2].Value);
                double duration = GetAudioDuration(file);

                result.Add(new AudioEntry
                {
                    Round = round,
                    Time = TimeSpan.FromSeconds(seconds),
                    DurationSeconds = duration,
                    FilePath = file,
                });
            }
            return result.OrderBy(e => e.Round).ThenBy(e => e.Time).ToList();
        }

        public static bool Play(string filePath)
        {
            if (!File.Exists(filePath)) return false;
            lock (_lock)
            {
                Stop();
                _waveOut = new WaveOutEvent();
                _audioFileReader = new AudioFileReader(filePath);
                _waveOut.Init(_audioFileReader);
                _playbackStoppedHandler = (s, e) =>
                {
                    Stop();
                    PlaybackEnded?.Invoke();
                };
                _waveOut.PlaybackStopped += _playbackStoppedHandler;
                _waveOut.Play();
                return true;
            }
        }

        public static void Stop()
        {
            lock (_lock)
            {
                if (_waveOut != null && _playbackStoppedHandler != null)
                {
                    _waveOut.PlaybackStopped -= _playbackStoppedHandler;
                    _playbackStoppedHandler = null;
                }
                _waveOut?.Stop();
                _audioFileReader?.Dispose();
                _waveOut?.Dispose();
                _audioFileReader = null;
                _waveOut = null;
            }
        }

        private static double GetAudioDuration(string filePath)
        {
            using var reader = new AudioFileReader(filePath);
            var seconds = reader.TotalTime.TotalSeconds;
            if (seconds > 0) return seconds;

            // Windows Media Foundation writes FLAC files without a total-sample count in the
            // STREAMINFO header, so MediaFoundationReader reports a zero duration even though the
            // audio is intact. Fall back to decoding the stream and counting samples.
            long sampleFrames = 0;
            int channels = Math.Max(1, reader.WaveFormat.Channels);
            var buffer = new float[reader.WaveFormat.SampleRate * channels];
            int read;
            while ((read = reader.Read(buffer, 0, buffer.Length)) > 0) sampleFrames += read;
            return (double)sampleFrames / reader.WaveFormat.SampleRate / channels;
        }
    }
}
