// Controlled host I/O failures, not a Sovra application or compiler shortcut.
using System;
using System.IO;
using System.Threading;
using System.Diagnostics;
using Sovra.Desktop;

internal static class UiProcessFixture
{
    private static int Main(string[] args)
    {
        string state = args[1];
        if (args[0] == "invocation-tree")
        {
            string self = System.Reflection.Assembly.GetExecutingAssembly().Location;
            using (Process child = Process.Start(new ProcessStartInfo(self, "tree-middle \"" + state + "\"") { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true }))
            {
                string grandchild = child.StandardOutput.ReadLine();
                File.WriteAllText(state + ".pids", Process.GetCurrentProcess().Id + "," + child.Id + "," + grandchild);
                var ready = Stopwatch.StartNew();
                while (!File.Exists(state) && ready.ElapsedMilliseconds < 1000) Thread.Sleep(10);
                if (!File.Exists(state)) throw new IOException("Writer fixture did not start.");
                if (args[2] == "exit") return 7;
                if (args[2] == "flood") { Console.Write(new string('x', 65537)); Console.Out.Flush(); }
                Thread.Sleep(30000);
            }
            return 0;
        }
        if (args[0] == "tree-parent")
        {
            ProcessLifetime.ProtectChildren();
            ProcessLifetime.ProtectChildren(); // Repeated initialization is harmless.
            string self = System.Reflection.Assembly.GetExecutingAssembly().Location;
            using (Process child = Process.Start(new ProcessStartInfo(self, "tree-middle \"" + state + "\"") { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true }))
            {
                string grandchild = child.StandardOutput.ReadLine();
                Console.WriteLine(child.Id + "," + grandchild); Console.Out.Flush();
                Thread.Sleep(args.Length > 2 ? 500 : 30000);
            }
            return 0;
        }
        if (args[0] == "tree-middle")
        {
            string self = System.Reflection.Assembly.GetExecutingAssembly().Location;
            using (Process child = Process.Start(new ProcessStartInfo(self, "tree-writer \"" + state + "\"") { UseShellExecute = false, CreateNoWindow = true }))
            {
                Console.WriteLine(child.Id); Console.Out.Flush();
                Thread.Sleep(30000);
            }
            return 0;
        }
        if (args[0] == "tree-writer")
        {
            File.WriteAllText(state, "started");
            Thread.Sleep(3000);
            File.AppendAllText(state, "late-write");
            Thread.Sleep(30000);
            return 0;
        }
        if (args[0] == "hold-profile")
        {
            using (ProfileLease.Acquire(state))
            {
                Console.WriteLine("lease-held"); Console.Out.Flush();
                Thread.Sleep(30000);
            }
            return 0;
        }
        if (args[0] == "ui-event")
        {
            string action = args[2];
            if (action == "slow") Thread.Sleep(600);
            if (action == "timeout") Thread.Sleep(5000);
            File.AppendAllText(state + ".events", action + "\n");
            if (action == "break-view") File.WriteAllText(state, "broken");
            Console.WriteLine("fixture result");
            return action == "fail" ? 7 : 0;
        }
        Console.WriteLine("SVR-UI-2");
        Console.WriteLine("heading|Host transport fixture");
        Console.WriteLine("group|actions|vertical|plain");
        foreach (string action in new string[] { "slow", "fail", "timeout", "break-view" })
            Console.WriteLine("button|" + action + "|" + action);
        if (!File.Exists(state) || File.ReadAllText(state) != "broken") Console.WriteLine("end|actions");
        return 0;
    }
}
