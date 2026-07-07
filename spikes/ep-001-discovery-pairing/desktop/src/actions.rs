use std::collections::HashMap;
use serde_json::{json, Value};

pub struct ActionError {
    pub message: String,
}

pub trait Action: Send + Sync {
    fn execute(&self, payload: &Value) -> Result<Value, ActionError>;
}

pub struct ActionRegistry {
    actions: HashMap<&'static str, Box<dyn Action>>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        let mut r = ActionRegistry { actions: HashMap::new() };
        r.register("launch", Box::new(LaunchAction));
        r.register("open_url", Box::new(OpenUrlAction));
        r.register("open_file", Box::new(OpenFileAction));
        r.register("lock", Box::new(LockAction));
        r.register("notify", Box::new(NotifyAction));
        r
    }

    pub fn register(&mut self, name: &'static str, action: Box<dyn Action>) {
        self.actions.insert(name, action);
    }

    pub fn execute(&self, name: &str, payload: &Value) -> Result<Value, ActionError> {
        match self.actions.get(name) {
            Some(a) => a.execute(payload),
            None => Err(ActionError { message: format!("Unknown action: {}", name) }),
        }
    }
}

// --- Actions ---

struct LaunchAction;
struct OpenUrlAction;
struct OpenFileAction;
struct LockAction;
struct NotifyAction;

impl Action for LaunchAction {
    fn execute(&self, payload: &Value) -> Result<Value, ActionError> {
        let app = payload
            .get("app")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ActionError { message: "Missing 'app' in payload".into() })?;

        let child = std::process::Command::new("cmd")
            .args(["/C", "start", "", app])
            .spawn()
            .map_err(|e| ActionError {
                message: format!("Failed to launch '{}': {}", app, e),
            })?;

        Ok(json!({"pid": child.id()}))
    }
}

impl Action for OpenUrlAction {
    fn execute(&self, payload: &Value) -> Result<Value, ActionError> {
        let url = payload
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ActionError { message: "Missing 'url' in payload".into() })?;

        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map_err(|e| ActionError {
                message: format!("Failed to open URL '{}': {}", url, e),
            })?;

        Ok(json!({"opened": url}))
    }
}

impl Action for OpenFileAction {
    fn execute(&self, payload: &Value) -> Result<Value, ActionError> {
        let path = payload
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ActionError { message: "Missing 'path' in payload".into() })?;

        std::process::Command::new("cmd")
            .args(["/C", "start", "", path])
            .spawn()
            .map_err(|e| ActionError {
                message: format!("Failed to open file '{}': {}", path, e),
            })?;

        Ok(json!({"opened": path}))
    }
}

impl Action for LockAction {
    fn execute(&self, _payload: &Value) -> Result<Value, ActionError> {
        #[cfg(windows)]
        {
            let result = unsafe { windows_sys::Win32::System::Shutdown::LockWorkStation() };
            if result == 0 {
                return Err(ActionError {
                    message: "LockWorkStation failed".into(),
                });
            }
        }
        #[cfg(not(windows))]
        {
            let _ = _payload;
        }
        Ok(json!({"locked": true}))
    }
}

impl Action for NotifyAction {
    fn execute(&self, payload: &Value) -> Result<Value, ActionError> {
        let title = payload
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("AutoMatDeck");
        let body = payload
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        #[cfg(windows)]
        {
            show_windows_toast(title, body)?;
        }
        #[cfg(not(windows))]
        {
            let _ = (title, body);
        }

        Ok(json!({"notified": true}))
    }
}

#[cfg(windows)]
fn show_windows_toast(title: &str, body: &str) -> Result<(), ActionError> {
    let escaped_title = title.replace('\'', "''");
    let escaped_body = body.replace('\'', "''");

    let script = format!(
        r#"
$null = [Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime]
$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml("<toast><visual><binding template='ToastText02'><text id='1'>{title}</text><text id='2'>{body}</text></binding></visual></toast>")
$toast = New-Object Windows.UI.Notifications.ToastNotification($xml)
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('AutoMatDeck').Show($toast)
"#,
        title = escaped_title,
        body = escaped_body,
    );

    match std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .spawn()
    {
        Ok(_) => Ok(()),
        Err(e) => Err(ActionError {
            message: format!("Failed to show notification: {}", e),
        }),
    }
}
