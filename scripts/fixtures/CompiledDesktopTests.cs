// Executed as a compiled process: PowerShell is not a test or application host.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Threading;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Input;
using System.Windows.Threading;
using Sovra.Desktop;

internal static class CompiledDesktopTests
{
    private static int checks;
    private static void Check(bool valid, string message) { if (!valid) throw new Exception(message); checks++; }
    private static void Reject(Action action)
    {
        bool rejected = false;
        try { action(); } catch (InvalidDataException) { rejected = true; }
        Check(rejected, "Invalid UI input was accepted.");
    }
    private static void PumpUntil(Func<bool> ready)
    {
        var watch = Stopwatch.StartNew();
        while (!ready())
        {
            if (watch.ElapsedMilliseconds > 15000) throw new TimeoutException("Desktop test did not become ready.");
            var frame = new DispatcherFrame();
            Dispatcher.CurrentDispatcher.BeginInvoke(DispatcherPriority.Background, new Action(delegate { frame.Continue = false; }));
            Dispatcher.PushFrame(frame); Thread.Sleep(10);
        }
    }
    private static T Control<T>(DependencyObject parent, string id) where T : DependencyObject
    {
        if (parent is T && AutomationProperties.GetAutomationId(parent) == id) return (T)parent;
        for (int index = 0; index < VisualTreeHelper.GetChildrenCount(parent); index++)
        {
            T found = Control<T>(VisualTreeHelper.GetChild(parent, index), id);
            if (found != null) return found;
        }
        return null;
    }
    private static void Click(DesktopWindow window, string action)
    {
        Button button = Control<Button>(window, action);
        Check(button != null, "Missing native action: " + action);
        button.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        Check(window.Busy, "Native button did not start a Sovra event.");
        Check(!window.StartEvent(action, "", false), "Concurrent event was accepted.");
        PumpUntil(delegate { return !window.Busy; });
    }
    private static void TaskWindow(string compiler, string source, string state, string command)
    {
        var bridge = new Bridge(compiler, state, source, command);
        bridge.Refresh(); Check(bridge.Nodes[0].Text == "Sovra Task Manager", "View did not come from Sovra.");
        Reject(delegate { bridge.Dispatch("not-offered", "", false); });
        Reject(delegate { bridge.Dispatch("add", "", false); });
        Check(!File.Exists(state), "Rejected event wrote application state.");
        var window = new DesktopWindow(bridge);
        try
        {
            window.Show(); window.UpdateLayout();
            TextBox input = Control<TextBox>(window, "input-add");
            Check(input != null && AutomationProperties.GetName(input) == "Task title", "Missing accessible native input.");
            string title = "Literal <UI> \"quotes\" & $(not code) caf\u00e9 trailing\\";
            input.Text = title; Click(window, "add");
            Check(window.LastOperationStatus == 0 && bridge.Notice.StartsWith("added "), "Sovra add failed.");
            Check(File.ReadAllText(state).Contains(title), "Sovra did not persist literal argument bytes.");
            Check(input.Text == "" && Object.ReferenceEquals(input, Control<TextBox>(window, "input-add")), "Successful input was not retained and cleared.");
            input.Text = "Keep unfinished draft"; input.Select(3, 7); input.Focus();
            Click(window, "complete-1");
            Check(window.LastOperationStatus == 0, "Sovra completion failed.");
            Check(input.Text == "Keep unfinished draft" && input.SelectionStart == 3 && input.SelectionLength == 7, "Unrelated event lost draft/selection.");
            Check(input.IsKeyboardFocused, "Unrelated event lost keyboard focus.");
            Click(window, "delete-1");
            Check(Control<Button>(window, "delete-1") == null, "Deleted task remains in the native view.");
            input.Text = "Bad\u001b[2J"; Click(window, "add");
            Check(window.LastOperationStatus != 0, "Invalid title unexpectedly succeeded.");
            Check(input.Text == "Bad\u001b[2J", "Rejected submission lost its draft.");
            Check(File.ReadAllText(state).StartsWith("SVR-TASKS-2\n"), "Sovra state format changed.");
            string beforeBusy = File.ReadAllText(state);
            string marker = state + ".svr-write-lock";
            File.WriteAllText(marker, "owned test marker");
            try
            {
                input.Text = "Retain blocked draft"; Click(window, "add");
                Check(window.LastOperationStatus == 1 && bridge.Notice.Contains("save busy"), "Busy writer did not produce a visible rejection.");
                Check(input.Text == "Retain blocked draft" && window.IsVisible, "Busy rejection lost the draft or closed the window.");
                Check(File.ReadAllText(state) == beforeBusy && File.ReadAllText(marker) == "owned test marker", "Busy rejection changed data or removed another writer's marker.");
            }
            finally { File.Delete(marker); }
        }
        finally { window.Close(); }
    }
    private static void ComponentWindow(string compiler, string source, string state)
    {
        var bridge = new Bridge(compiler, state, source, "ui-layout-view");
        var window = new DesktopWindow(bridge);
        try
        {
            window.Show(); window.UpdateLayout();
            Check(window.Title == "Sovra reusable component", "Renderer is application-specific.");
            TextBox input = Control<TextBox>(window, "input-save-message");
            input.Text = "Reusable compiled renderer"; Click(window, "save-message");
            Check(File.ReadAllText(state) == "Reusable compiled renderer", "Reusable Sovra component did not persist.");
            Click(window, "reset-message");
            Check(File.ReadAllText(state) == "Ready from Sovra", "Reusable reset did not execute Sovra.");
            input.Text = "";
            Check(window.StartEvent("save-message", input.Text, true), "Could not start failure-close test.");
            window.Close();
            Check(window.IsVisible && window.Busy, "Window closed during an in-flight event.");
            PumpUntil(delegate { return !window.Busy; });
            Check(window.IsVisible && window.LastOperationStatus != 0, "Failed event closed the window.");
            input.Text = "Saved before close";
            Check(window.StartEvent("save-message", input.Text, true), "Could not start success-close test.");
            window.Close();
            PumpUntil(delegate { return !window.Busy; });
            Check(!window.IsVisible && File.ReadAllText(state) == "Saved before close", "Close did not wait for committed Sovra work.");
        }
        finally { window.Close(); }
    }
    private static void KeyboardAndWorkflow(string compiler, string source, string state)
    {
        var bridge = new Bridge(compiler, state, source, "ui-layout-view");
        Check(bridge.Dispatch("add", "Keyboard task one", true) == 0, "Could not seed first task.");
        Check(bridge.Dispatch("add", "Keyboard task two", true) == 0, "Could not seed second task.");
        // Exercise the system-color palette without changing the user's OS settings.
        var window = new DesktopWindow(bridge, DesktopTheme.Create(true));
        try
        {
            window.Show(); window.Activate(); window.UpdateLayout();
            TextBox input = Control<TextBox>(window, "input-add");
            Button add = Control<Button>(window, "add");
            Check(Object.ReferenceEquals(window.Background, SystemColors.WindowBrush), "High-contrast window ignores system colors.");
            Check(Object.ReferenceEquals(input.Foreground, SystemColors.WindowTextBrush), "High-contrast input ignores system text colors.");
            Check(Object.ReferenceEquals(add.Foreground, SystemColors.ControlTextBrush), "High-contrast button ignores system text colors.");
            Check(AutomationProperties.GetName(Control<Button>(window, "complete-1")).Contains("Keyboard task one") &&
                AutomationProperties.GetName(Control<Button>(window, "complete-2")).Contains("Keyboard task two"), "Repeated actions have indistinguishable names.");
            input.Focus(); input.Text = "Discard local draft";
            var escape = new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(input), Environment.TickCount, Key.Escape);
            escape.RoutedEvent = Keyboard.PreviewKeyDownEvent; input.RaiseEvent(escape);
            Check(escape.Handled && input.Text == "" && !window.Busy, "Escape mutated application state or failed to clear a draft.");
            input.Text = "Submitted by Enter";
            var enter = new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(input), Environment.TickCount, Key.Return);
            enter.RoutedEvent = Keyboard.PreviewKeyDownEvent; input.RaiseEvent(enter);
            Check(enter.Handled && window.Busy, "Enter did not submit an event.");
            Check(!window.StartEvent("add", "Duplicate", true), "Busy window accepted a duplicate event.");
            PumpUntil(delegate { return !window.Busy; });
            Check(Control<Button>(window, "complete-3") != null && input.IsKeyboardFocused && input.Text == "", "Enter lost its result/focus.");
            input.MoveFocus(new TraversalRequest(FocusNavigationDirection.Next));
            Check(add.IsKeyboardFocused, "Tab does not reach the submit button.");
            add.MoveFocus(new TraversalRequest(FocusNavigationDirection.Previous));
            Check(input.IsKeyboardFocused, "Reverse Tab does not return to the input.");
            TextBlock status = Control<TextBlock>(window, "application-status");
            Check(AutomationProperties.GetLiveSetting(status) == AutomationLiveSetting.Polite &&
                AutomationProperties.GetName(status).Contains("added 3"), "Status is not exposed to assistive technology.");
            input.Text = "Unsubmitted add draft";
            TextBox rename = Control<TextBox>(window, "input-rename-1");
            rename.Text = "Edited | task"; Click(window, "rename-1");
            Check(rename.Text == "" && File.ReadAllText(state).Contains("1|0|Edited | task"), "Rename changed identity or lost literal text.");
            Click(window, "complete-1");
            rename.Text = "Completed edited task"; Click(window, "rename-1");
            Check(File.ReadAllText(state).Contains("1|1|Completed edited task"), "Rename lost completion state.");
            string saved = File.ReadAllText(state);
            rename.Text = "Removed field draft";
            Click(window, "filter-open");
            Check(Control<TextBox>(window, "input-rename-1") == null, "Open filter retained a completed task.");
            Click(window, "filter-completed");
            TextBox restored = Control<TextBox>(window, "input-rename-1");
            Check(restored != null && !Object.ReferenceEquals(rename, restored) && restored.Text == "", "Removed control inherited a discarded draft.");
            TextBox search = Control<TextBox>(window, "input-search");
            search.Text = "no matching title"; Click(window, "search");
            Check(Control<Button>(window, "delete-1") == null, "Search failed to narrow the view.");
            Click(window, "clear-search");
            Check(Control<Button>(window, "delete-1") != null && Control<Button>(window, "complete-2") == null, "Clear search lost the selected status filter.");
            Click(window, "filter-all");
            Check(File.ReadAllText(state) == saved, "View-only events mutated task data.");
            Check(Object.ReferenceEquals(input, Control<TextBox>(window, "input-add")) && input.Text == "Unsubmitted add draft", "Workflow lost an unrelated draft.");
        }
        finally { window.Close(); }
    }
    private static void MovingLayout(string compiler, string source, string state)
    {
        var bridge = new Bridge(compiler, state, source, "ui-layout-view");
        var window = new DesktopWindow(bridge);
        try
        {
            window.Show(); window.Activate(); window.UpdateLayout();
            TextBox input = Control<TextBox>(window, "input-value");
            input.Text = "Draft across groups"; input.Select(2, 5); input.Focus();
            Check(AutomationProperties.GetName(Control<Button>(window, "root-action")) == "Root action: Moving component", "Group context leaked into root accessibility names.");
            Check(AutomationProperties.GetName(Control<Button>(window, "right-action")) == "Right action: Right context", "Sibling group context is incorrect.");
            foreach (string expected in new string[] { "right", "horizontal" })
            {
                Click(window, "advance");
                Check(File.ReadAllText(state) == expected, "Fixture transition did not execute Sovra.");
                Check(Object.ReferenceEquals(input, Control<TextBox>(window, "input-value")) && input.Text == "Draft across groups" &&
                    input.SelectionStart == 2 && input.SelectionLength == 5 && input.IsKeyboardFocused, "Moving a control lost its draft, selection or focus.");
            }
            Border group = Control<Border>(window, "group-left");
            Check(group.Child is WrapPanel, "Orientation change did not rebuild the group panel.");
            Click(window, "advance");
            Check(Control<TextBox>(window, "input-value") == null && Control<Button>(window, "value") != null, "Kind change retained the old input.");
            Click(window, "advance");
            Check(Control<Button>(window, "value") == null, "Removed action remains in the tree.");
            Click(window, "advance");
            TextBox fresh = Control<TextBox>(window, "input-value");
            Check(fresh != null && !Object.ReferenceEquals(input, fresh) && fresh.Text == "", "Reintroduced control inherited a removed draft.");
        }
        finally { window.Close(); }
    }
    private static void Profiles()
    {
        string valid = "{\"schema\":1,\"id\":\"test-app\",\"program\":\"application.exe\"}";
        using (var input = new MemoryStream(Encoding.UTF8.GetBytes(valid)))
        {
            ApplicationProfile profile = ApplicationProfile.Read(input);
            Check(profile.Id == "test-app" && profile.ViewCommand == "ui-view", "Default profile changed.");
            Check(input.CanRead, "Profile reader closed a caller-owned stream.");
        }
        foreach (string bad in new string[] {
            valid.Replace("\"schema\":1", "\"schema\":\"1\""), valid.Replace("test-app", "../outside"),
            valid.Replace("application.exe", "other.exe"), valid.Replace("\"schema\":1", "\"schema\":2"),
            valid.Substring(0, valid.Length - 1) + ",\"view_command\":null}",
            valid.Substring(0, valid.Length - 1) + ",\"view_command\":\"unknown\"}"
        }) Reject(delegate { using (var input = new MemoryStream(Encoding.UTF8.GetBytes(bad))) ApplicationProfile.Read(input); });
        using (var input = new MemoryStream(Encoding.UTF8.GetBytes(valid.PadRight(16384))))
            Check(ApplicationProfile.Read(input).Id == "test-app", "Exact profile byte limit was rejected.");
        using (var input = new MemoryStream(Encoding.UTF8.GetBytes(valid.PadRight(16385))))
        {
            Reject(delegate { ApplicationProfile.Read(input); });
            Check(input.Position == 16385, "Profile reader did not enforce its actual read bound.");
        }
        bool invalid = false;
        try { using (var input = new MemoryStream(new byte[] { 255 })) ApplicationProfile.Read(input); }
        catch (DecoderFallbackException) { invalid = true; }
        Check(invalid, "Profile silently replaced invalid UTF-8.");
    }
    private static void FailureAndResponsiveness(string scratch)
    {
        string state = Path.Combine(scratch, "transport-state.txt");
        var bridge = new Bridge(Path.Combine(scratch, "ui-process-fixture.exe"), state, "", "ui-layout-view");
        var window = new DesktopWindow(bridge);
        var heartbeat = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(20) };
        int ticks = 0; heartbeat.Tick += delegate { ticks++; };
        try
        {
            window.Show(); window.UpdateLayout(); heartbeat.Start();
            Check(window.StartEvent("slow", "", false), "Slow event did not start.");
            PumpUntil(delegate { return !window.Busy; });
            Check(ticks >= 3 && window.LastOperationStatus == 0, "Slow child blocked the UI dispatcher.");
            Click(window, "fail");
            Check(window.LastOperationStatus == 7 && window.IsVisible, "Nonzero exit closed the window or lost status.");
            Check(bridge.Notice.Contains("exit 7"), "Application error is not visible.");
            bridge.TimeoutMilliseconds = 500;
            Click(window, "timeout");
            Check(window.LastOperationStatus == null && bridge.Notice.Contains("timed out"), "Timeout did not leave a visible host error.");
            Check(!File.ReadAllText(state + ".events").Contains("timeout"), "Timed-out child survived to write its event.");
            bridge.TimeoutMilliseconds = 30000;
            Click(window, "break-view");
            Check(window.LastOperationStatus == null && bridge.Notice.Contains("Unclosed"), "Malformed refreshed view was accepted.");
            string mutations = File.ReadAllText(state + ".events");
            Click(window, "slow");
            Check(File.ReadAllText(state + ".events") == mutations, "Invalid current view permitted a stale action mutation.");
            Check(window.IsVisible && ticks >= 3, "Host failed to recover its dispatcher after an error.");
        }
        finally { heartbeat.Stop(); window.Close(); }
    }
    private static void PaginationWindow(string compiler, string source, string state)
    {
        var stored = new StringBuilder("SVR-TASKS-2\n1001\n");
        for (int id = 1; id <= 1000; id++) stored.Append(id + "|0|Task " + id + "\n");
        File.WriteAllText(state, stored.ToString(), new UTF8Encoding(false));
        var bridge = new Bridge(compiler, state, source, "ui-layout-view");
        var window = new DesktopWindow(bridge);
        try
        {
            window.Show(); window.Activate(); window.UpdateLayout();
            Check(Control<Button>(window, "complete-25") != null && Control<Button>(window, "complete-26") == null,
                "Large task view did not open on its bounded first page.");
            TextBox input = Control<TextBox>(window, "input-add");
            input.Text = "Keep navigation draft"; input.Select(2, 4); input.Focus();
            TextBox removed = Control<TextBox>(window, "input-rename-1"); removed.Text = "Discard on leaving page";
            Click(window, "page-next");
            Check(Control<Button>(window, "complete-26") != null && Control<Button>(window, "complete-1") == null,
                "Native Next page did not execute Sovra pagination.");
            Check(Object.ReferenceEquals(input, Control<TextBox>(window, "input-add")) && input.Text == "Keep navigation draft" &&
                input.SelectionStart == 2 && input.SelectionLength == 4 && input.IsKeyboardFocused, "Navigation lost a stable input draft/focus.");
            Click(window, "page-previous");
            Check(Control<TextBox>(window, "input-rename-1").Text == "", "Returning to a page resurrected a discarded task draft.");
            Check(File.ReadAllText(state) == stored.ToString(), "Native navigation mutated task data.");
        }
        finally { window.Close(); }
    }
    private static void ExpectBusy(Action action)
    {
        bool busy = false;
        try { action(); } catch (IOException error) { busy = error.Message.Contains("already open"); }
        Check(busy, "Duplicate profile ownership was not rejected with an actionable error.");
    }
    private static void ProfileOwnership(string scratch)
    {
        string first = Path.Combine(scratch, "profile one");
        string second = Path.Combine(scratch, "profile two");
        Directory.CreateDirectory(first); Directory.CreateDirectory(second);
        string state = Path.Combine(first, "state.txt"); File.WriteAllText(state, "preserve application data");
        using (ProfileLease.Acquire(first))
        {
            ExpectBusy(delegate { using (ProfileLease.Acquire(first)) { } });
            using (ProfileLease.Acquire(second)) Check(true, "Independent profile ownership failed.");
            Check(File.ReadAllText(state) == "preserve application data", "Lease modified application state.");
        }
        var released = ProfileLease.Acquire(first); released.Dispose(); released.Dispose();
        using (ProfileLease.Acquire(first)) Check(true, "Clean close did not release profile ownership.");
        Check(File.Exists(Path.Combine(first, ".desktop.lock")), "Lease marker was deleted and could introduce an ownership race.");
        bool missing = false;
        try { using (ProfileLease.Acquire(Path.Combine(scratch, "missing profile"))) { } }
        catch (DirectoryNotFoundException) { missing = true; }
        Check(missing, "Non-lock I/O failure was misreported as another instance.");
        var start = new ProcessStartInfo(Path.Combine(scratch, "ui-process-fixture.exe"),
            "hold-profile " + Bridge.Argument(first));
        start.UseShellExecute = false; start.CreateNoWindow = true; start.RedirectStandardOutput = true;
        using (Process process = Process.Start(start))
        {
            try
            {
                var ready = process.StandardOutput.ReadLineAsync();
                PumpUntil(delegate { return ready.IsCompleted; });
                Check(ready.GetAwaiter().GetResult() == "lease-held", "Child did not acquire its profile lease.");
                ExpectBusy(delegate { using (ProfileLease.Acquire(first)) { } });
                using (ProfileLease.Acquire(second)) Check(true, "Another process blocked an independent profile.");
                process.Kill(); Check(process.WaitForExit(5000), "Owned lease fixture did not terminate.");
                using (ProfileLease.Acquire(first)) Check(true, "Process termination left a stale profile lock.");
            }
            finally { if (!process.HasExited) { process.Kill(); process.WaitForExit(5000); } }
        }
    }
    private static void Package(string directory, string id)
    {
        string owned = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Sovra", "Desktop", id);
        Check(View.ValidKey(id) && id.StartsWith("compiled-test-"), "Invalid owned test profile.");
        Check(!Directory.Exists(owned), "Test must not reuse existing user data.");
        Process process = null;
        try
        {
            for (int launch = 0; launch < 3; launch++)
            {
                if (launch == 2)
                {
                    var stored = new StringBuilder("SVR-TASKS-2\n1001\n1|0|Compiled host production path\n");
                    for (int idValue = 2; idValue <= 1000; idValue++) stored.Append(idValue + "|0|Task " + idValue + "\n");
                    File.WriteAllText(Path.Combine(owned, "state.txt"), stored.ToString(), new UTF8Encoding(false));
                }
                var start = new ProcessStartInfo(Path.Combine(directory, "Sovra UI.exe"));
                start.UseShellExecute = false; start.CreateNoWindow = true; start.WorkingDirectory = directory;
                string system = Environment.GetFolderPath(Environment.SpecialFolder.System);
                start.EnvironmentVariables["PATH"] = system + ";" + Path.GetDirectoryName(system);
                process = Process.Start(start);
                AutomationElement window = null;
                PumpUntil(delegate {
                    if (process.HasExited) throw new Exception("Packaged host exited before opening a window.");
                    process.Refresh();
                    if (process.MainWindowHandle == IntPtr.Zero) return false;
                    window = AutomationElement.FromHandle(process.MainWindowHandle);
                    if (window.Current.Name == "Sovra UI could not start")
                    {
                        var messages = new StringBuilder();
                        foreach (AutomationElement element in window.FindAll(TreeScope.Descendants, System.Windows.Automation.Condition.TrueCondition))
                            messages.AppendLine(element.Current.Name);
                        throw new Exception("Packaged startup error: " + messages);
                    }
                    return window.Current.Name == "Sovra Task Manager";
                });
                // The UI belongs to the compiled EXE itself, not a shell child.
                Check(window.Current.ProcessId == process.Id, "Desktop window is not owned by the compiled host.");
                var descendants = TreeScope.Descendants;
                if (launch == 0)
                {
                    AutomationElement input = window.FindFirst(descendants, new PropertyCondition(AutomationElement.AutomationIdProperty, "input-add"));
                    Check(input != null, "Packaged host is missing its native input.");
                    ((ValuePattern)input.GetCurrentPattern(ValuePattern.Pattern)).SetValue("Compiled host production path");
                    AutomationElement add = window.FindFirst(descendants, new PropertyCondition(AutomationElement.AutomationIdProperty, "add"));
                    ((InvokePattern)add.GetCurrentPattern(InvokePattern.Pattern)).Invoke();
                }
                PumpUntil(delegate {
                    return window.FindFirst(descendants, new PropertyCondition(AutomationElement.AutomationIdProperty, "complete-1")) != null;
                });
                Check(File.ReadAllText(Path.Combine(owned, "state.txt")).Contains("Compiled host production path"), "Packaged native Sovra app did not persist its event.");
                if (launch == 0)
                {
                    string beforeDuplicate = File.ReadAllText(Path.Combine(owned, "state.txt"));
                    StartupError(directory, "already open");
                    Check(!process.HasExited && File.ReadAllText(Path.Combine(owned, "state.txt")) == beforeDuplicate,
                        "Duplicate launch disturbed the original writer or its state.");
                    string marker = Path.Combine(owned, "state.txt.svr-write-lock");
                    File.WriteAllText(marker, "owned packaged test marker");
                    try
                    {
                        AutomationElement input = window.FindFirst(descendants, new PropertyCondition(AutomationElement.AutomationIdProperty, "input-add"));
                        ((ValuePattern)input.GetCurrentPattern(ValuePattern.Pattern)).SetValue("Retain blocked packaged draft");
                        AutomationElement add = window.FindFirst(descendants, new PropertyCondition(AutomationElement.AutomationIdProperty, "add"));
                        ((InvokePattern)add.GetCurrentPattern(InvokePattern.Pattern)).Invoke();
                        PumpUntil(delegate {
                            AutomationElement status = window.FindFirst(descendants, new PropertyCondition(AutomationElement.AutomationIdProperty, "application-status"));
                            return status != null && status.Current.Name.Contains("save busy");
                        });
                        Check(((ValuePattern)input.GetCurrentPattern(ValuePattern.Pattern)).Current.Value == "Retain blocked packaged draft", "Packaged busy rejection lost its draft.");
                        Check(!process.HasExited && File.ReadAllText(Path.Combine(owned, "state.txt")) == beforeDuplicate,
                            "Packaged busy rejection changed state or closed the host.");
                        Check(File.ReadAllText(marker) == "owned packaged test marker", "Packaged invocation removed another writer's marker.");
                    }
                    finally { File.Delete(marker); }
                }
                if (launch == 2)
                {
                    AutomationElement next = window.FindFirst(descendants, new PropertyCondition(AutomationElement.AutomationIdProperty, "page-next"));
                    Check(next != null, "Large native package view has no page controls.");
                    ((InvokePattern)next.GetCurrentPattern(InvokePattern.Pattern)).Invoke();
                    PumpUntil(delegate { return window.FindFirst(descendants, new PropertyCondition(AutomationElement.AutomationIdProperty, "complete-26")) != null; });
                    Check(window.FindFirst(descendants, new PropertyCondition(AutomationElement.AutomationIdProperty, "complete-1")) == null,
                        "Packaged page transition retained prior task controls.");
                }
                ((WindowPattern)window.GetCurrentPattern(WindowPattern.Pattern)).Close();
                Check(process.WaitForExit(10000) && process.ExitCode == 0, "Packaged desktop did not exit cleanly.");
                process.Dispose(); process = null;
            }
            string profilePath = Path.Combine(directory, "app.json");
            byte[] profileBytes = File.ReadAllBytes(profilePath);
            string stateBytes = File.ReadAllText(Path.Combine(owned, "state.txt"));
            try
            {
                File.WriteAllText(profilePath, "{\"schema\":2}");
                StartupError(directory, "Invalid private UI application profile");
            }
            finally { File.WriteAllBytes(profilePath, profileBytes); }
            string program = Path.Combine(directory, "application.exe");
            File.Move(program, program + ".missing");
            try { StartupError(directory, "Sovra program/source was not found"); }
            finally { File.Move(program + ".missing", program); }
            Check(File.ReadAllText(Path.Combine(owned, "state.txt")) == stateBytes, "Startup failure changed existing application state.");
        }
        finally
        {
            if (process != null) { if (!process.HasExited) { process.Kill(); process.WaitForExit(5000); } process.Dispose(); }
            // Only the fresh, validated profile created by this test may be removed.
            if (Directory.Exists(owned)) Directory.Delete(owned, true);
        }
    }
    private static void StartupError(string directory, string expected)
    {
        var start = new ProcessStartInfo(Path.Combine(directory, "Sovra UI.exe"));
        start.UseShellExecute = false; start.CreateNoWindow = true;
        using (Process process = Process.Start(start))
        {
            try
            {
                AutomationElement dialog = null;
                PumpUntil(delegate {
                    if (process.HasExited) throw new Exception("Startup failure exited without a visible diagnostic.");
                    process.Refresh();
                    if (process.MainWindowHandle == IntPtr.Zero) return false;
                    dialog = AutomationElement.FromHandle(process.MainWindowHandle);
                    if (dialog.Current.Name == "Sovra Task Manager") throw new Exception("Second desktop instance was allowed to open.");
                    return dialog.Current.Name == "Sovra UI could not start";
                });
                var messages = new StringBuilder();
                foreach (AutomationElement element in dialog.FindAll(TreeScope.Descendants, System.Windows.Automation.Condition.TrueCondition))
                    messages.AppendLine(element.Current.Name);
                Check(messages.ToString().Contains(expected), "Startup diagnostic lost its cause: " + messages);
                ((WindowPattern)dialog.GetCurrentPattern(WindowPattern.Pattern)).Close();
                Check(process.WaitForExit(10000) && process.ExitCode == 1, "Startup failure did not propagate exit status.");
            }
            finally { if (!process.HasExited) { process.Kill(); process.WaitForExit(5000); } }
        }
    }
    private static void ChildLifetime(string scratch)
    {
        foreach (bool crash in new bool[] { true, false }) ChildLifetime(scratch, crash);
    }
    private static void InvocationLifetime(string scratch)
    {
        foreach (string mode in new string[] { "timeout", "flood", "exit" })
        {
            string marker = Path.Combine(scratch, "invocation-" + mode + ".txt");
            var bridge = new Bridge(Path.Combine(scratch, "ui-process-fixture.exe"), marker, "", "ui-view");
            bridge.TimeoutMilliseconds = 1500;
            try
            {
                bool expected = false;
                try { expected = bridge.Invoke("invocation-tree", marker, mode).Status == 7 && mode == "exit"; }
                catch (TimeoutException) { expected = mode == "timeout"; }
                catch (InvalidDataException) { expected = mode == "flood"; }
                Check(expected, "Invocation did not preserve its failure/status: " + mode);
                Check(File.Exists(marker + ".pids"), "Invocation tree was not exercised: " + mode);
                foreach (string id in File.ReadAllText(marker + ".pids").Split(','))
                {
                    Process child = null;
                    try { child = Process.GetProcessById(Int32.Parse(id)); }
                    catch (ArgumentException) { }
                    using (child) Check(child == null || child.WaitForExit(1000), "Invocation left a live descendant: " + mode);
                }
                Check(File.ReadAllText(marker) == "started", "Cancelled invocation performed a late write.");
                Check(bridge.Invoke("ui-view", marker).Status == 0, "Host could not run another invocation after cleanup.");
            }
            finally
            {
                if (File.Exists(marker + ".pids")) foreach (string id in File.ReadAllText(marker + ".pids").Split(','))
                {
                    try { using (Process child = Process.GetProcessById(Int32.Parse(id))) { if (!child.HasExited) { child.Kill(); child.WaitForExit(5000); } } }
                    catch (ArgumentException) { }
                }
            }
        }
    }
    private static void ChildLifetime(string scratch, bool crash)
    {
        string marker = Path.Combine(scratch, "orphan-writer-" + crash + ".txt");
        string fixture = Path.Combine(scratch, "ui-process-fixture.exe");
        using (var parent = Process.Start(new ProcessStartInfo(fixture, "tree-parent " + Bridge.Argument(marker) + (crash ? "" : " normal")) {
            UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true
        }))
        {
            Process child = null;
            Process grandchild = null;
            try
            {
                var ready = parent.StandardOutput.ReadLineAsync();
                Check(ready.Wait(5000), "Child lifetime fixture did not start.");
                string[] ids = ready.Result.Split(',');
                child = Process.GetProcessById(Int32.Parse(ids[0]));
                grandchild = Process.GetProcessById(Int32.Parse(ids[1]));
                PumpUntil(delegate { return File.Exists(marker); });
                if (crash) parent.Kill();
                Check(parent.WaitForExit(5000), "Fixture parent did not terminate.");
                if (!crash) Check(parent.ExitCode == 0, "Lifetime guard changed normal exit status.");
                Check(child.WaitForExit(1500), "Desktop host termination left its writer child alive.");
                Check(grandchild.WaitForExit(1500), "Desktop host termination left its grandchild alive.");
                Check(File.ReadAllText(marker) == "started", "Orphan child wrote after host termination.");
            }
            finally
            {
                if (!parent.HasExited) { parent.Kill(); parent.WaitForExit(5000); }
                if (child != null) { if (!child.HasExited) { child.Kill(); child.WaitForExit(5000); } child.Dispose(); }
                if (grandchild != null) { if (!grandchild.HasExited) { grandchild.Kill(); grandchild.WaitForExit(5000); } grandchild.Dispose(); }
            }
        }
    }
    [STAThread]
    private static int Main(string[] args)
    {
        int workerStatus;
        if (InvocationWorker.TryRun(args, out workerStatus)) return workerStatus;
        if (args.Length == 2 && args[0] == "fixture")
        {
            if (args[1] == "flood") { Console.Write(new string('x', 65537)); return 0; }
            if (args[1] == "stderr") { Console.Error.Write(new string('x', 65537)); return 0; }
            if (args[1] == "timeout") { Thread.Sleep(5000); return 0; }
            if (args[1] == "utf8") { Console.OpenStandardOutput().WriteByte(255); return 0; }
        }
        try
        {
            string root = args[0], scratch = args[1];
            Profiles();
            ProfileOwnership(scratch);
            ChildLifetime(scratch);
            InvocationLifetime(scratch);
            foreach (string bad in new string[] {
                "SVR-UI-1", "SVR-UI-3\nheading|Bad\n", "SVR-UI-1\ntext|No heading\n",
                "SVR-UI-1\nheading|Bad\ttext\n", "SVR-UI-1\nheading|Title\ninput|add|No submit\n",
                "SVR-UI-1\nheading|Title\nbutton|Bad|No\n", "SVR-UI-1\nheading|Title\nscript|bad\n",
                "SVR-UI-1\nheading|Title\nbutton|same|One\nbutton|same|Two\n",
                "SVR-UI-2\nheading|Title\ngroup|card|vertical|card\nend|other\n",
                "SVR-UI-2\nheading|Title\ngroup|card|vertical|card\n",
                "SVR-UI-2\nheading|Title\ngroup|card|vertical|card\nend|card\ngroup|card|vertical|card\nend|card\n",
                "SVR-UI-1\nheading|" + new string('x', 65536) + "\n"
            }) Reject(delegate { View.Parse(bad); });
            var excess = new StringBuilder("SVR-UI-1\nheading|Title\n");
            for (int index = 0; index < 256; index++) excess.Append("text|extra\n");
            Reject(delegate { View.Parse(excess.ToString()); });
            var depth = new StringBuilder("SVR-UI-2\nheading|Title\n");
            for (int index = 0; index < 9; index++) depth.Append("group|g-" + index + "|vertical|plain\n");
            Reject(delegate { View.Parse(depth.ToString()); });
            Check(View.Parse("SVR-UI-1\nheading|Literal <text>\ninput|save|Name|Save\n").Count == 2, "Valid view rejected.");
            string self = System.Reflection.Assembly.GetExecutingAssembly().Location;
            var transport = new Bridge(self, Path.Combine(scratch, "unused"), "", "ui-view");
            Reject(delegate { transport.Invoke("fixture", "flood"); });
            Reject(delegate { transport.Invoke("fixture", "stderr"); });
            bool invalidUtf8 = false;
            try { transport.Invoke("fixture", "utf8"); } catch (DecoderFallbackException) { invalidUtf8 = true; }
            Check(invalidUtf8, "Malformed UTF-8 was accepted.");
            transport.TimeoutMilliseconds = 100;
            bool timeout = false;
            try { transport.Invoke("fixture", "timeout"); } catch (TimeoutException) { timeout = true; }
            Check(timeout, "Unbounded application execution.");
            string compiler = Path.Combine(root, "target", "debug", "svr.exe");
            string task = Path.Combine(root, "dogfood", "01-task-manager", "main.svr");
            TaskWindow(compiler, task, Path.Combine(scratch, "flat state !.txt"), "ui-view");
            TaskWindow(compiler, task, Path.Combine(scratch, "grouped state !.txt"), "ui-layout-view");
            ComponentWindow(compiler, Path.Combine(root, "examples", "ui-desktop", "component.svr"), Path.Combine(scratch, "message.txt"));
            KeyboardAndWorkflow(compiler, task, Path.Combine(scratch, "workflow.txt"));
            MovingLayout(compiler, Path.Combine(root, "scripts", "fixtures", "ui-moving.svr"), Path.Combine(scratch, "moving.txt"));
            FailureAndResponsiveness(scratch);
            PaginationWindow(compiler, task, Path.Combine(scratch, "large-window.txt"));
            if (args.Length == 4) Package(args[2], args[3]);
            Console.WriteLine("PASS compiled desktop: " + checks + " checks; bounded transport, invalid views, real Sovra events, flat/grouped WPF and reusable component.");
            return 0;
        }
        catch (Exception error) { Console.Error.WriteLine(error); return 1; }
    }
}
