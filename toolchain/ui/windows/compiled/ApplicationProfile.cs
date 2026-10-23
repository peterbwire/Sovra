using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Web.Script.Serialization;

namespace Sovra.Desktop
{
    // Private packaging profile, independent of any application or renderer tree.
    internal sealed class ApplicationProfile
    {
        internal readonly string Id, ViewCommand;
        private ApplicationProfile(string id, string command) { Id = id; ViewCommand = command; }

        internal static ApplicationProfile Load(string path)
        {
            using (var stream = File.OpenRead(path)) return Read(stream);
        }

        internal static ApplicationProfile Read(Stream stream)
        {
            // Bound the actual read, not a separate earlier file-size observation.
            // The extra byte distinguishes an exact-limit file from an oversized one.
            byte[] bytes = new byte[16385];
            int total = 0;
            while (total < bytes.Length)
            {
                int count = stream.Read(bytes, total, bytes.Length - total);
                if (count == 0) break;
                total += count;
            }
            if (total > 16384) throw new InvalidDataException("UI application profile is too large.");
            string text;
            using (var snapshot = new MemoryStream(bytes, 0, total))
            using (var reader = new StreamReader(snapshot, new UTF8Encoding(false, true), true)) text = reader.ReadToEnd();
            var json = new JavaScriptSerializer { MaxJsonLength = 16384, RecursionLimit = 16 };
            var config = json.DeserializeObject(text) as Dictionary<string, object>;
            if (config == null || !config.ContainsKey("schema") || !(config["schema"] is int) || (int)config["schema"] != 1 ||
                !config.ContainsKey("id") || !(config["id"] is string) || !View.ValidKey((string)config["id"]) ||
                !config.ContainsKey("program") || !(config["program"] is string) || (string)config["program"] != "application.exe")
                throw new InvalidDataException("Invalid private UI application profile.");
            string command = "ui-view";
            if (config.ContainsKey("view_command"))
            {
                command = config["view_command"] as string;
                if (command != "ui-view" && command != "ui-layout-view") throw new InvalidDataException("Invalid UI view command.");
            }
            return new ApplicationProfile((string)config["id"], command);
        }
    }
}
