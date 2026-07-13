use crate::model::ContextSnapshot;

#[derive(Debug)]
pub(crate) enum ContextObserverError {
    ProcessOpenFailed,
    ProcessNameQueryFailed,
    InvalidProcessName,
    PlatformNotSupported,
}

pub(crate) struct ForegroundObserver;

#[cfg(windows)]
impl ForegroundObserver {
    pub(crate) fn current_context() -> Result<Option<ContextSnapshot>, ContextObserverError> {
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, HWND};
        use windows_sys::Win32::System::ProcessStatus::GetModuleBaseNameW;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId,
        };

        unsafe {
            let hwnd: HWND = GetForegroundWindow();
            if hwnd.is_null() {
                return Ok(None);
            }

            let mut pid: u32 = 0;
            let tid = GetWindowThreadProcessId(hwnd, &mut pid);
            if tid == 0 || pid == 0 {
                return Err(ContextObserverError::ProcessOpenFailed);
            }

            let handle: HANDLE = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid);
            if handle.is_null() {
                return Err(ContextObserverError::ProcessOpenFailed);
            }

            let mut buf = [0u16; 260];
            let chars = GetModuleBaseNameW(handle, std::ptr::null_mut(), buf.as_mut_ptr(), 260);
            CloseHandle(handle);

            if chars == 0 {
                return Err(ContextObserverError::ProcessNameQueryFailed);
            }

            let len = chars as usize;
            let basename = String::from_utf16_lossy(&buf[..len]);

            if basename.is_empty() {
                return Err(ContextObserverError::InvalidProcessName);
            }

            Ok(Some(ContextSnapshot {
                foreground_process: basename,
            }))
        }
    }
}

#[cfg(not(windows))]
impl ForegroundObserver {
    pub(crate) fn current_context() -> Result<Option<ContextSnapshot>, ContextObserverError> {
        Err(ContextObserverError::PlatformNotSupported)
    }
}

/// Pure policy boundary: maps an observation result to an actionable snapshot.
/// `Ok(snapshot)` → `Some(snapshot)` — caller should apply observation.
/// `Err(_)` → `None` — caller must not mutate runtime state.
pub(crate) fn successful_observation(
    observation: Result<Option<ContextSnapshot>, ContextObserverError>,
) -> Option<Option<ContextSnapshot>> {
    match observation {
        Ok(snapshot) => Some(snapshot),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_observation_ok_some() {
        let snap = ContextSnapshot {
            foreground_process: "Code.exe".into(),
        };
        assert_eq!(
            successful_observation(Ok(Some(snap.clone()))),
            Some(Some(snap))
        );
    }

    #[test]
    fn successful_observation_ok_none() {
        assert_eq!(successful_observation(Ok(None)), Some(None));
    }

    #[test]
    fn successful_observation_err_process_open_failed() {
        assert_eq!(
            successful_observation(Err(ContextObserverError::ProcessOpenFailed)),
            None
        );
    }

    #[test]
    fn successful_observation_err_process_name_query_failed() {
        assert_eq!(
            successful_observation(Err(ContextObserverError::ProcessNameQueryFailed)),
            None
        );
    }

    #[test]
    fn successful_observation_err_invalid_process_name() {
        assert_eq!(
            successful_observation(Err(ContextObserverError::InvalidProcessName)),
            None
        );
    }
}
