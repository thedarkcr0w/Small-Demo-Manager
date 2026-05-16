using System.Security.Cryptography;

namespace SmallDemoManager.UtilClass
{
    internal static class Extensions
    {
        public static bool IsNullOrEmptyOrWhiteSpace(this string? s) => string.IsNullOrWhiteSpace(s);

        public static byte[] ComputeFileHash(this string path)
        {
            using var sha256 = SHA256.Create();
            using var stream = File.OpenRead(path);
            return sha256.ComputeHash(stream);
        }

        public static bool HashesAreEqual(this byte[] hash1, byte[] hash2)
        {
            if (hash1.Length != hash2.Length) return false;
            for (int i = 0; i < hash1.Length; i++)
                if (hash1[i] != hash2[i]) return false;
            return true;
        }
    }
}
