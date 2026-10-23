// Reusable WPF renderer: no application command, storage format or task-specific controls.
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Threading;

namespace Sovra.Desktop
{
    public sealed class DesktopWindow : Window
    {
        private sealed class Row
        {
            public string Kind;
            public StackPanel Element;
            public TextBox Input;
            public Label Label;
            public Button Button;
        }
        private sealed class Group
        {
            public string Orientation;
            public Border Element;
            public Panel Panel;
        }
        private sealed class Plan
        {
            public Panel Panel;
            public List<UIElement> Children = new List<UIElement>();
        }
        private readonly Bridge bridge;
        private readonly DesktopTheme fixedTheme;
        private DesktopTheme theme;
        private readonly StackPanel panel = new StackPanel();
        private readonly TextBlock status = new TextBlock();
        private readonly ScrollViewer scroll = new ScrollViewer();
        private Dictionary<string, Row> rows = new Dictionary<string, Row>(StringComparer.Ordinal);
        private Dictionary<string, Group> groups = new Dictionary<string, Group>(StringComparer.Ordinal);
        private bool busy, closeRequested;
        public bool Busy { get { return busy; } }
        public int? LastOperationStatus { get; private set; }

        public DesktopWindow(Bridge application) : this(application, null) { }

        internal DesktopWindow(Bridge application, DesktopTheme presentation)
        {
            bridge = application;
            fixedTheme = presentation;
            theme = fixedTheme ?? DesktopTheme.Create(SystemParameters.HighContrast);
            Width = 780; Height = 640; MinWidth = 420; MinHeight = 320;
            WindowStartupLocation = WindowStartupLocation.CenterScreen;
            UseLayoutRounding = true; SnapsToDevicePixels = true;
            KeyboardNavigation.SetTabNavigation(this, KeyboardNavigationMode.Cycle);
            var root = new DockPanel { Margin = new Thickness(24) };
            status.TextWrapping = TextWrapping.Wrap; status.Margin = new Thickness(0, 0, 0, 16);
            AutomationProperties.SetAutomationId(status, "application-status");
            DockPanel.SetDock(status, Dock.Top); root.Children.Add(status);
            scroll.VerticalScrollBarVisibility = ScrollBarVisibility.Auto;
            scroll.IsTabStop = false; scroll.Content = panel; root.Children.Add(scroll);
            Content = root;
            Closing += delegate(object sender, CancelEventArgs args) {
                if (!busy) return;
                args.Cancel = true; closeRequested = true;
                SetStatus("Finishing the current operation before closing...");
            };
            bridge.Refresh(); Render();
            if (fixedTheme == null) SystemParameters.StaticPropertyChanged += SettingsChanged;
            Closed += delegate { SystemParameters.StaticPropertyChanged -= SettingsChanged; };
        }

        private void SettingsChanged(object sender, PropertyChangedEventArgs args)
        {
            if (args.PropertyName == "HighContrast")
                Dispatcher.BeginInvoke(new Action(delegate {
                    theme = DesktopTheme.Create(SystemParameters.HighContrast);
                    ApplyTheme();
                }));
        }

        private void ApplyTheme()
        {
            Background = theme.Background; Foreground = theme.Foreground; status.Foreground = theme.Foreground;
            foreach (Row row in rows.Values)
            {
                theme.Apply(row.Button);
                if (row.Input != null) { theme.Apply(row.Input); row.Label.Foreground = theme.Foreground; }
            }
            foreach (Group group in groups.Values)
            {
                group.Element.BorderBrush = theme.Border;
                group.Element.Background = group.Element.BorderThickness.Left > 0 ? theme.Surface : Brushes.Transparent;
            }
        }
        private void SetStatus(string text)
        {
            status.Text = text;
            AutomationProperties.SetName(status, "Application status: " + text);
            AutomationProperties.SetLiveSetting(status, AutomationLiveSetting.Polite);
            var peer = System.Windows.Automation.Peers.UIElementAutomationPeer.FromElement(status);
            if (peer != null) peer.RaiseAutomationEvent(System.Windows.Automation.Peers.AutomationEvents.LiveRegionChanged);
        }

        public bool StartEvent(string action, string value, bool hasValue)
        {
            if (busy) return false;
            var priorFocus = Keyboard.FocusedElement as TextBox;
            int priorStart = priorFocus == null ? 0 : priorFocus.SelectionStart;
            int priorLength = priorFocus == null ? 0 : priorFocus.SelectionLength;
            double priorOffset = scroll.VerticalOffset;
            busy = true; LastOperationStatus = null; SetStatus("Working...");
            foreach (Row row in rows.Values) { row.Button.IsEnabled = false; if (row.Input != null) row.Input.IsEnabled = false; }
            var worker = new BackgroundWorker();
            worker.DoWork += delegate(object sender, DoWorkEventArgs args) { args.Result = bridge.Dispatch(action, value, hasValue); };
            worker.RunWorkerCompleted += delegate(object sender, RunWorkerCompletedEventArgs args) {
                Dispatcher.BeginInvoke(new Action(delegate {
                    bool success = args.Error == null && (int)args.Result == 0;
                    busy = false;
                    LastOperationStatus = args.Error == null ? (int?)((int)args.Result) : null;
                    if (success && hasValue && rows.ContainsKey(action) && rows[action].Input != null) rows[action].Input.Clear();
                    // No automatic replay: a timeout may follow a committed application write.
                    bridge.Notice = args.Error == null ? bridge.Notice : args.Error.Message;
                    Render();
                    scroll.ScrollToVerticalOffset(priorOffset);
                    if (priorFocus != null)
                        foreach (Row row in rows.Values)
                            if (Object.ReferenceEquals(row.Input, priorFocus))
                            {
                                priorFocus.Focus();
                                priorFocus.Select(priorStart, priorLength);
                                break;
                            }
                    if (closeRequested && success) Close();
                    else closeRequested = false;
                    worker.Dispose();
                }));
            };
            worker.RunWorkerAsync();
            return true;
        }

        private void Render()
        {
            var focused = Keyboard.FocusedElement as TextBox;
            int selectionStart = focused == null ? 0 : focused.SelectionStart;
            int selectionLength = focused == null ? 0 : focused.SelectionLength;
            double offset = scroll.VerticalOffset;
            Title = bridge.Nodes[0].Text; SetStatus(bridge.Notice);
            var desiredRows = new Dictionary<string, Row>(StringComparer.Ordinal);
            var desiredGroups = new Dictionary<string, Group>(StringComparer.Ordinal);
            var plans = new List<Plan>();
            var current = new Plan { Panel = panel }; plans.Add(current);
            var parents = new Stack<Plan>();
            var contexts = new Stack<string>();
            string context = "";
            int tab = 0;
            foreach (Node node in bridge.Nodes)
            {
                if (node.Kind == "group")
                {
                    contexts.Push(context); context = "";
                    Group group;
                    if (!groups.TryGetValue(node.Key, out group) || group.Orientation != node.Orientation)
                    {
                        Panel children = node.Orientation == "horizontal" ? (Panel)new WrapPanel() : new StackPanel();
                        group = new Group { Orientation = node.Orientation, Panel = children, Element = new Border { Child = children } };
                    }
                    group.Element.Margin = new Thickness(0, 0, 0, 12);
                    bool card = node.Presentation == "card";
                    group.Element.Padding = new Thickness(card ? 16 : 0);
                    group.Element.BorderThickness = new Thickness(card ? 1 : 0);
                    group.Element.BorderBrush = theme.Border;
                    group.Element.CornerRadius = new CornerRadius(card ? 10 : 0);
                    group.Element.Background = card ? theme.Surface : Brushes.Transparent;
                    AutomationProperties.SetAutomationId(group.Element, "group-" + node.Key);
                    desiredGroups.Add(node.Key, group); current.Children.Add(group.Element); parents.Push(current);
                    current = new Plan { Panel = group.Panel }; plans.Add(current);
                    continue;
                }
                if (node.Kind == "end") { current = parents.Pop(); context = contexts.Pop(); continue; }
                if (node.Kind == "heading" || node.Kind == "text")
                {
                    context = node.Text;
                    var text = new TextBlock { Text = node.Text, TextWrapping = TextWrapping.Wrap, Margin = new Thickness(0, 0, 0, 12) };
                    if (current.Panel is WrapPanel) { text.MaxWidth = 420; text.Margin = new Thickness(0, 10, 16, 8); }
                    if (node.Kind == "heading") { text.FontSize = 28; text.FontWeight = FontWeights.Bold; }
                    current.Children.Add(text); continue;
                }
                Row row;
                if (!rows.TryGetValue(node.Action, out row) || row.Kind != node.Kind) row = CreateRow(node);
                ((TextBlock)row.Button.Content).Text = node.Kind == "input" ? node.Submit : node.Label;
                row.Button.IsEnabled = !busy; row.Button.TabIndex = tab++;
                string actionContext = context;
                foreach (string ancestor in contexts) { if (actionContext != "") break; actionContext = ancestor; }
                AutomationProperties.SetName(row.Button, node.Kind == "input" ? node.Submit + ": " + node.Label :
                    node.Label + (actionContext == "" ? "" : ": " + actionContext));
                if (row.Input != null)
                {
                    ((TextBlock)row.Label.Content).Text = node.Label;
                    row.Input.IsEnabled = !busy; row.Input.TabIndex = row.Button.TabIndex; row.Button.TabIndex = tab++;
                    AutomationProperties.SetName(row.Input, node.Label);
                }
                row.Element.Margin = current.Panel is WrapPanel ? new Thickness(0, 0, 8, 0) : new Thickness(0, 0, 0, 12);
                desiredRows.Add(node.Action, row); current.Children.Add(row.Element);
            }
            // Detach removed/moved controls before attaching them to another group.
            foreach (Plan plan in plans)
                for (int index = plan.Panel.Children.Count - 1; index >= 0; index--)
                    if (!plan.Children.Contains(plan.Panel.Children[index])) plan.Panel.Children.RemoveAt(index);
            foreach (Plan plan in plans)
                for (int index = 0; index < plan.Children.Count; index++)
                {
                    UIElement element = plan.Children[index];
                    if (index < plan.Panel.Children.Count && Object.ReferenceEquals(plan.Panel.Children[index], element)) continue;
                    var parent = LogicalTreeHelper.GetParent(element) as Panel;
                    if (parent != null) parent.Children.Remove(element);
                    plan.Panel.Children.Insert(index, element);
                }
            rows = desiredRows; groups = desiredGroups;
            ApplyTheme();
            UpdateLayout(); scroll.ScrollToVerticalOffset(offset);
            if (focused != null)
                foreach (Row row in rows.Values)
                    if (Object.ReferenceEquals(row.Input, focused)) { focused.Focus(); focused.Select(selectionStart, selectionLength); break; }
        }

        private Row CreateRow(Node node)
        {
            var row = new Row { Kind = node.Kind, Element = new StackPanel() };
            string action = node.Action;
            bool hasValue = node.Kind == "input";
            if (hasValue)
            {
                row.Input = new TextBox { MinHeight = 32, FontSize = 16, Padding = new Thickness(8, 4, 8, 4), MaxLength = 65536 };
                row.Label = new Label { Content = new TextBlock(), Target = row.Input, IsTabStop = false };
                AutomationProperties.SetAutomationId(row.Input, "input-" + action);
                row.Input.PreviewKeyDown += delegate(object sender, KeyEventArgs args) {
                    if (Keyboard.Modifiers != ModifierKeys.None) return;
                    if (args.Key == Key.Escape) { args.Handled = true; row.Input.Clear(); }
                    else if (args.Key == Key.Return) { args.Handled = true; if (!args.IsRepeat) StartEvent(action, row.Input.Text, true); }
                };
                row.Element.Children.Add(row.Label); row.Element.Children.Add(row.Input);
            }
            row.Button = new Button { Content = new TextBlock(), HorizontalAlignment = HorizontalAlignment.Left,
                Margin = new Thickness(0, 6, 0, 0), Padding = new Thickness(16, 7, 16, 7),
                Background = theme.Action, Foreground = theme.ActionText };
            AutomationProperties.SetAutomationId(row.Button, action);
            row.Button.Click += delegate { StartEvent(action, hasValue ? row.Input.Text : "", hasValue); };
            row.Element.Children.Add(row.Button);
            return row;
        }
    }
}
