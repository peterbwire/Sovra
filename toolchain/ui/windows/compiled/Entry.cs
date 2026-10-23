using System;
using System.IO;
using System.Windows;

namespace Sovra.Desktop
{
    internal static class Entry
    {
        [STAThread]
        private static int Main()
        {
            try
            {
                string root = AppDomain.CurrentDomain.BaseDirectory;
                ApplicationProfile profile = ApplicationProfile.Load(Path.Combine(root, "app.json"));
                string data = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
                if (String.IsNullOrEmpty(data)) throw new IOException("Windows local application data is unavailable.");
                string directory = Path.Combine(data, "Sovra", "Desktop", profile.Id);
                Directory.CreateDirectory(directory);
                var bridge = new Bridge(Path.Combine(root, "application.exe"), Path.Combine(directory, "state.txt"), "", profile.ViewCommand);
                var application = new Application();
                application.Run(new DesktopWindow(bridge));
                return 0;
            }
            catch (Exception error)
            {
                MessageBox.Show(error.Message, "Sovra UI could not start", MessageBoxButton.OK, MessageBoxImage.Error);
                return 1;
            }
        }
    }
}
