use serde_json::{Value, json};
use std::collections::HashMap;

#[cfg(windows)]
use winrt_notification::{Duration, Sound, Toast};

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn shell_execute(verb: &str, file: &str) -> Result<(), ActionError> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    unsafe {
        let wide_verb = to_wide(verb);
        let wide_file = to_wide(file);
        let result = ShellExecuteW(
            std::ptr::null_mut(),
            wide_verb.as_ptr(),
            wide_file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
        if (result as isize) <= 32 {
            return Err(ActionError {
                message: format!("ShellExecuteW failed for '{}'", file),
            });
        }
    }
    Ok(())
}

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
        let mut r = ActionRegistry {
            actions: HashMap::new(),
        };
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
            None => Err(ActionError {
                message: format!("Unknown action: {}", name),
            }),
        }
    }
}

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
            .ok_or_else(|| ActionError {
                message: "Missing 'app' in payload".into(),
            })?;

        #[cfg(windows)]
        shell_execute("open", app)?;

        Ok(json!({"launched": app}))
    }
}

impl Action for OpenUrlAction {
    fn execute(&self, payload: &Value) -> Result<Value, ActionError> {
        let url = payload
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ActionError {
                message: "Missing 'url' in payload".into(),
            })?;

        #[cfg(windows)]
        shell_execute("open", url)?;

        Ok(json!({"opened": url}))
    }
}

impl Action for OpenFileAction {
    fn execute(&self, payload: &Value) -> Result<Value, ActionError> {
        let path = payload
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ActionError {
                message: "Missing 'path' in payload".into(),
            })?;

        #[cfg(windows)]
        shell_execute("open", path)?;

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
        Ok(json!({"locked": true}))
    }
}

impl Action for NotifyAction {
    fn execute(&self, payload: &Value) -> Result<Value, ActionError> {
        let title = payload
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("AutoMatDeck");
        let body = payload.get("body").and_then(|v| v.as_str()).unwrap_or("");

        #[cfg(windows)]
        show_windows_toast(title, body)?;

        Ok(json!({"notified": true}))
    }
}

#[cfg(windows)]
fn show_windows_toast(title: &str, body: &str) -> Result<(), ActionError> {
    Toast::new(Toast::POWERSHELL_APP_ID)
        .title(title)
        .text1(body)
        .sound(Some(Sound::SMS))
        .duration(Duration::Short)
        .show()
        .map_err(|e| ActionError {
            message: format!("Failed to show toast: {:?}", e),
        })
}
