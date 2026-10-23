// Controlled host I/O failures, not a Sovra application or compiler shortcut.
using System;
using System.IO;
using System.Threading;

internal static class UiProcessFixture
{
    private static int Main(string[] args)
    {
        string state = args[1];
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
