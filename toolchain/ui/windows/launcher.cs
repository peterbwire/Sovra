// Private Windows desktop host launcher. Application behavior remains in Sovra.
using System;
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Windows.Forms;

internal static class SovraDesktopLauncher
{
    private static string Argument(string value)
    {
        var result = new StringBuilder("\"");
        int slashes = 0;
        foreach (char character in value)
        {
            if (character == '\\') { slashes++; continue; }
            result.Append('\\', character == '"' ? 2 * slashes + 1 : slashes);
            result.Append(character);
            slashes = 0;
        }
        result.Append('\\', 2 * slashes);
        return result.Append('"').ToString();
    }

    [STAThread]
    private static int Main()
    {
        try
        {
            string root = AppDomain.CurrentDomain.BaseDirectory;
            string host = Path.Combine(root, "ui", "host.ps1");
            string profile = Path.Combine(root, "app.json");
            string shell = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.System),
                "WindowsPowerShell", "v1.0", "powershell.exe");
            if (!File.Exists(host) || !File.Exists(profile) || !File.Exists(shell))
                throw new IOException("The Sovra desktop package or Windows PowerShell is missing.");
            var start = new ProcessStartInfo(shell,
                "-NoProfile -STA -ExecutionPolicy Bypass -File " + Argument(host) + " -ApplicationProfile " + Argument(profile));
            start.UseShellExecute = false;
            start.CreateNoWindow = true;
            start.WorkingDirectory = root;
            using (var process = Process.Start(start))
            {
                if (process == null) throw new IOException("Could not start the Sovra desktop renderer.");
                process.WaitForExit();
                if (process.ExitCode != 0 && process.ExitCode != 10)
                    throw new IOException("The Sovra desktop host could not start (exit " + process.ExitCode +
                        "). Check the extracted package and Windows application policy.");
                return process.ExitCode;
            }
        }
        catch (Exception error)
        {
            MessageBox.Show(error.Message, "Sovra UI could not start", MessageBoxButtons.OK, MessageBoxIcon.Error);
            return 1;
        }
    }
}
