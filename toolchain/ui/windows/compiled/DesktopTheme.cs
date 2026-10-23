// Platform presentation only. Theme changes never invoke an application event.
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;

namespace Sovra.Desktop
{
    internal sealed class DesktopTheme
    {
        internal Brush Background, Surface, Foreground, Border, Action, ActionText, Input;

        private static Brush Color(string value)
        {
            var brush = (Brush)new BrushConverter().ConvertFromString(value);
            brush.Freeze();
            return brush;
        }

        internal static DesktopTheme Create(bool highContrast)
        {
            if (highContrast)
                return new DesktopTheme {
                    Background = SystemColors.WindowBrush, Surface = SystemColors.WindowBrush,
                    Foreground = SystemColors.WindowTextBrush, Border = SystemColors.WindowTextBrush,
                    Action = SystemColors.ControlBrush, ActionText = SystemColors.ControlTextBrush,
                    Input = SystemColors.WindowBrush
                };
            return new DesktopTheme {
                Background = Color("#f2f5f6"), Surface = Brushes.White, Foreground = Color("#183247"),
                Border = Color("#dbe3e8"), Action = Color("#075b72"), ActionText = Brushes.White,
                Input = Brushes.White
            };
        }

        internal void Apply(Button button)
        {
            button.Background = Action; button.Foreground = ActionText; button.BorderBrush = Border;
        }

        internal void Apply(TextBox input)
        {
            input.Background = Input; input.Foreground = Foreground; input.BorderBrush = Border;
        }
    }
}
