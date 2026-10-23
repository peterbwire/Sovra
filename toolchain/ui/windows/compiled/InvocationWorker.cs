using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;

namespace Sovra.Desktop
{
    // Private re-entry into the same executable, never a public Sovra command.
    // The worker owns a job before starting application code. Killing just this
    // worker closes its sole job handle and terminates that invocation's tree.
    internal static class InvocationWorker
    {
        internal const string Flag = "--svr-private-ui-invocation";

        internal static bool TryRun(string[] arguments, out int status)
        {
            status = 1;
            if (arguments.Length == 0 || arguments[0] != Flag) return false;
            try
            {
                if (arguments.Length < 2) throw new ArgumentException("Missing desktop invocation program.");
                ProcessLifetime.ProtectChildren();
                var encoded = new List<string>();
                for (int index = 2; index < arguments.Length; index++) encoded.Add(Bridge.Argument(arguments[index]));
                var start = new ProcessStartInfo(Path.GetFullPath(arguments[1]), String.Join(" ", encoded.ToArray()));
                start.UseShellExecute = false; start.CreateNoWindow = true;
                // Setting stdin redirection also makes .NET pass the worker's
                // inherited stdout/stderr handles directly to the application.
                // No text decoding or unbounded intermediate buffers occur here.
                start.RedirectStandardInput = true;
                using (var process = new Process { StartInfo = start })
                {
                    if (!process.Start()) throw new IOException("Could not start Sovra application.");
                    process.StandardInput.Close();
                    process.WaitForExit();
                    status = process.ExitCode;
                }
            }
            catch (Exception error)
            {
                Console.Error.WriteLine("Desktop invocation failed: " + error.Message);
            }
            return true;
        }
    }
}
