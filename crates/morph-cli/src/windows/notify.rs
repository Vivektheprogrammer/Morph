use std::process::Command;

pub struct Notifier;

impl Notifier {
    /// Dispatches a native Windows Toast Notification without blocking the main worker.
    pub fn send_notification(title: &str, body: &str) {
        let title_escaped = title.replace('"', "`\"").replace('$', "`$");
        let body_escaped = body.replace('"', "`\"").replace('$', "`$");

        // PowerShell WinRT Toast Notification script (works out of the box on Windows 10/11)
        let ps_script = format!(
            r#"[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null;
$template = [Windows.UI.Notifications.ToastTemplateType]::ToastText02;
$xml = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent($template);
$textNodes = $xml.GetElementsByTagName('text');
$textNodes.Item(0).AppendChild($xml.CreateTextNode("{}")) | Out-Null;
$textNodes.Item(1).AppendChild($xml.CreateTextNode("{}")) | Out-Null;
$toast = [Windows.UI.Notifications.ToastNotification]::new($xml);
$notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Morph');
$notifier.Show($toast);
"#,
            title_escaped, body_escaped
        );

        #[cfg(windows)]
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut cmd = Command::new("powershell.exe");
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let _ = cmd
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-Command",
                &ps_script,
            ])
            .spawn();
    }
}
