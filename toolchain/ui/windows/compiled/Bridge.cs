// Private desktop transport. Sovra owns views, events, validation and persistence.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading;
using System.Threading.Tasks;

namespace Sovra.Desktop
{
    public sealed class Node
    {
        public string Kind, Text = "", Action = "", Label = "", Submit = "";
        public string Key = "", Orientation = "", Presentation = "";
    }

    public static class View
    {
        private static readonly Regex Key = new Regex(@"\A[a-z][a-z0-9-]{0,63}\z");
        public static bool ValidKey(string value) { return value != null && Key.IsMatch(value); }
        private static bool ValidText(string value)
        {
            if (String.IsNullOrEmpty(value)) return false;
            foreach (char character in value)
                if (character < 32 || (character >= 127 && character <= 159)) return false;
            return true;
        }

        public static List<Node> Parse(string text)
        {
            if (new UTF8Encoding(false, true).GetByteCount(text) > 65536)
                throw new InvalidDataException("UI view exceeds 65536 bytes.");
            if (!text.EndsWith("\n", StringComparison.Ordinal)) throw new InvalidDataException("Incomplete UI view.");
            string[] lines = text.Substring(0, text.Length - 1).Split('\n');
            bool grouped = lines[0] == "SVR-UI-2";
            if (!grouped && lines[0] != "SVR-UI-1") throw new InvalidDataException("Unsupported UI view version.");
            if (lines.Length < 2 || lines.Length > (grouped ? 769 : 257))
                throw new InvalidDataException("UI view exceeds its node limit.");
            var nodes = new List<Node>();
            var groups = new HashSet<string>(StringComparer.Ordinal);
            var actions = new HashSet<string>(StringComparer.Ordinal);
            var stack = new Stack<string>();
            int leaves = 0;
            for (int index = 1; index < lines.Length; index++)
            {
                int split = lines[index].IndexOf('|');
                if (split < 0) throw new InvalidDataException("Invalid UI node.");
                var node = new Node { Kind = lines[index].Substring(0, split) };
                string value = lines[index].Substring(split + 1);
                if (grouped && node.Kind == "group")
                {
                    string[] parts = value.Split('|');
                    if (parts.Length != 3 || !ValidKey(parts[0]) ||
                        (parts[1] != "vertical" && parts[1] != "horizontal") ||
                        (parts[2] != "card" && parts[2] != "plain"))
                        throw new InvalidDataException("Invalid UI group.");
                    if (groups.Count >= 256 || stack.Count >= 8 || !groups.Add(parts[0]))
                        throw new InvalidDataException("UI group identity/depth limit exceeded.");
                    node.Key = parts[0]; node.Orientation = parts[1]; node.Presentation = parts[2];
                    stack.Push(node.Key);
                }
                else if (grouped && node.Kind == "end")
                {
                    if (stack.Count == 0 || stack.Pop() != value) throw new InvalidDataException("Unbalanced UI group.");
                    node.Key = value;
                }
                else
                {
                    if (++leaves > 256) throw new InvalidDataException("UI view exceeds 256 leaf nodes.");
                    if (node.Kind == "heading" || node.Kind == "text")
                    {
                        if (!ValidText(value)) throw new InvalidDataException("Invalid UI text.");
                        node.Text = value;
                    }
                    else if (node.Kind == "button" || node.Kind == "input")
                    {
                        int first = value.IndexOf('|');
                        if (first < 0) throw new InvalidDataException("Invalid UI action.");
                        node.Action = value.Substring(0, first);
                        if (!ValidKey(node.Action) || !actions.Add(node.Action))
                            throw new InvalidDataException("Invalid or duplicate UI action key.");
                        if (node.Kind == "button") node.Label = value.Substring(first + 1);
                        else
                        {
                            int last = value.LastIndexOf('|');
                            if (last == first) throw new InvalidDataException("Invalid UI input.");
                            node.Label = value.Substring(first + 1, last - first - 1);
                            node.Submit = value.Substring(last + 1);
                            if (!ValidText(node.Submit)) throw new InvalidDataException("Invalid UI submit label.");
                        }
                        if (!ValidText(node.Label)) throw new InvalidDataException("Invalid UI action label.");
                    }
                    else throw new InvalidDataException("Unsupported UI node.");
                }
                nodes.Add(node);
            }
            if (nodes[0].Kind != "heading") throw new InvalidDataException("Missing UI heading.");
            if (stack.Count != 0) throw new InvalidDataException("Unclosed UI group.");
            return nodes;
        }
    }

    public sealed class ProgramResult
    {
        public int Status;
        public string Output, Error;
    }

    public sealed class Bridge
    {
        public readonly string Program, State, Source, ViewCommand;
        public int TimeoutMilliseconds = 30000;
        public List<Node> Nodes = new List<Node>();
        public string Notice = "";

        public Bridge(string program, string state, string source, string viewCommand)
        {
            Program = Path.GetFullPath(program); State = Path.GetFullPath(state);
            Source = String.IsNullOrEmpty(source) ? "" : Path.GetFullPath(source);
            if (!File.Exists(Program) || (Source != "" && !File.Exists(Source)))
                throw new FileNotFoundException("Sovra program/source was not found.");
            if (viewCommand != "ui-view" && viewCommand != "ui-layout-view")
                throw new InvalidDataException("Invalid UI view command.");
            ViewCommand = viewCommand;
        }

        public static string Argument(string value)
        {
            if (value.IndexOf('\0') >= 0) throw new ArgumentException("A process argument contains NUL.");
            var result = new StringBuilder("\"");
            int slashes = 0;
            foreach (char character in value)
            {
                if (character == '\\') { slashes++; continue; }
                result.Append('\\', character == '"' ? 2 * slashes + 1 : slashes);
                result.Append(character); slashes = 0;
            }
            result.Append('\\', 2 * slashes);
            return result.Append('"').ToString();
        }

        public ProgramResult Invoke(params string[] arguments)
        {
            var argv = new List<string>();
            if (Source != "") argv.AddRange(new string[] { "run", Source, "--" });
            argv.AddRange(arguments);
            var encoded = new List<string> { Argument(InvocationWorker.Flag), Argument(Program) };
            foreach (string argument in argv) encoded.Add(Argument(argument));
            var start = new ProcessStartInfo(System.Reflection.Assembly.GetExecutingAssembly().Location,
                String.Join(" ", encoded.ToArray()));
            start.UseShellExecute = false; start.CreateNoWindow = true;
            start.RedirectStandardInput = true; start.RedirectStandardOutput = true; start.RedirectStandardError = true;
            using (var process = new Process { StartInfo = start })
            using (var output = new MemoryStream())
            using (var error = new MemoryStream())
            {
                bool started = false;
                try
                {
                    if (!process.Start()) throw new IOException("Could not start Sovra application.");
                    started = true; process.StandardInput.Close();
                    Stream[] streams = { process.StandardOutput.BaseStream, process.StandardError.BaseStream };
                    MemoryStream[] captured = { output, error };
                    byte[][] buffers = { new byte[4096], new byte[4096] };
                    Task<int>[] pending = { streams[0].ReadAsync(buffers[0], 0, 4096), streams[1].ReadAsync(buffers[1], 0, 4096) };
                    bool[] done = { false, false };
                    var watch = Stopwatch.StartNew();
                    while (!done[0] || !done[1] || !process.HasExited)
                    {
                        if (watch.ElapsedMilliseconds > TimeoutMilliseconds) throw new TimeoutException("Sovra UI operation timed out.");
                        for (int index = 0; index < 2; index++)
                        {
                            if (done[index] || !pending[index].IsCompleted) continue;
                            int count = pending[index].GetAwaiter().GetResult();
                            if (count == 0) { done[index] = true; continue; }
                            if (captured[index].Length + count > 65536) throw new InvalidDataException("Sovra UI output exceeds 65536 bytes.");
                            captured[index].Write(buffers[index], 0, count);
                            pending[index] = streams[index].ReadAsync(buffers[index], 0, 4096);
                        }
                        if (!done[0] || !done[1] || !process.HasExited) Thread.Sleep(10);
                    }
                    var utf8 = new UTF8Encoding(false, true);
                    return new ProgramResult { Status = process.ExitCode,
                        Output = utf8.GetString(output.ToArray()).Replace("\r\n", "\n"),
                        Error = utf8.GetString(error.ToArray()).Replace("\r\n", "\n") };
                }
                finally
                {
                    if (started && !process.HasExited) { process.Kill(); process.WaitForExit(5000); }
                }
            }
        }

        public void Refresh()
        {
            ProgramResult result = Invoke(ViewCommand, State);
            if (result.Status != 0) throw new IOException("Cannot load Sovra view: " + result.Output + result.Error);
            Nodes = View.Parse(result.Output);
        }

        public int Dispatch(string action, string value, bool hasValue)
        {
            Refresh(); // Revalidate actions against the latest application state.
            Node selected = Nodes.Find(delegate(Node node) { return node.Action == action && (node.Kind == "button" || node.Kind == "input"); });
            if (selected == null) throw new InvalidDataException("UI action is unavailable.");
            if ((selected.Kind == "input") != hasValue) throw new InvalidDataException("UI event value does not match its control.");
            if (new UTF8Encoding(false, true).GetByteCount(value) > 65536) throw new InvalidDataException("UI event value exceeds 65536 bytes.");
            ProgramResult result = hasValue ? Invoke("ui-event", State, action, value) : Invoke("ui-event", State, action);
            Notice = (result.Output + result.Error).Trim();
            if (result.Status != 0) Notice = "Operation failed (exit " + result.Status + "). " + Notice;
            Refresh();
            return result.Status;
        }
    }
}
