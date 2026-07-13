use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use qrcode::QrCode;
use qrcode::types::Color;

pub const PAIR_CODE_LIFETIME_SECS: u64 = 300;
// ponytail: kept as const for documentation; not referenced outside tests
#[allow(dead_code)]
pub const PAIR_CODE_DIGITS: u32 = 6;

/// Map ValidationResult to a stable machine-readable protocol reason code.
pub fn validation_reason_code(result: &ValidationResult) -> &'static str {
    match result {
        ValidationResult::CodeMismatch => "code_mismatch",
        ValidationResult::Expired => "expired",
        ValidationResult::Cancelled => "cancelled",
        ValidationResult::AlreadyConsumed => "already_used",
        ValidationResult::NoSession => "no_session",
        ValidationResult::Accepted => "accepted",
    }
}

/// Snapshot of the active pairing session for GUI rendering.
/// Owned, cloneable, thread-safe projection of PairingManager state.
#[derive(Clone)]
#[allow(dead_code)]
pub struct PairingSessionSnapshot {
    pub otp: String,
    pub qr_matrix: Vec<Vec<bool>>,
    pub qr_size: usize,
    pub created_at: u64,
    pub expires_at: u64,
    pub device_id: String,
    pub hostname: String,
    pub port: u16,
    pub consumed: bool,
    pub cancelled: bool,
}

/// Owns pairing session lifecycle. Single source of truth.
/// Shared via Arc<PairingManager> between GUI thread and agent thread.
pub struct PairingManager {
    inner: Mutex<Option<InnerSession>>,
}

struct InnerSession {
    otp: String,
    qr_matrix: Vec<Vec<bool>>,
    qr_size: usize,
    created_at: u64,
    device_id: String,
    hostname: String,
    port: u16,
    consumed: bool,
    cancelled: bool,
}

impl PairingManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    /// Generate a fresh session with OTP and QR.
    pub fn generate_session(
        &self,
        device_id: &str,
        hostname: &str,
        port: u16,
    ) -> PairingSessionSnapshot {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let otp = format!("{:06}", now % 1_000_000);

        let pairing_url = format!(
            "amd://pair?otp={}&host={}&port={}&device_id={}",
            otp, hostname, port, device_id
        );

        let (qr_matrix, qr_size) = match QrCode::new(&pairing_url) {
            Ok(qr) => {
                let sz = qr.width();
                let mut mat = Vec::with_capacity(sz);
                for y in 0..sz {
                    let mut row = Vec::with_capacity(sz);
                    for x in 0..sz {
                        row.push(qr[(x, y)] == Color::Dark);
                    }
                    mat.push(row);
                }
                (mat, sz)
            }
            Err(_) => (Vec::new(), 0),
        };

        let snap = PairingSessionSnapshot {
            otp: otp.clone(),
            qr_matrix: qr_matrix.clone(),
            qr_size,
            created_at: now,
            expires_at: now + PAIR_CODE_LIFETIME_SECS,
            device_id: device_id.to_string(),
            hostname: hostname.to_string(),
            port,
            consumed: false,
            cancelled: false,
        };

        *self.inner.lock().unwrap() = Some(InnerSession {
            otp,
            qr_matrix,
            qr_size,
            created_at: now,
            device_id: device_id.to_string(),
            hostname: hostname.to_string(),
            port,
            consumed: false,
            cancelled: false,
        });

        snap
    }

    /// Read-only snapshot of current session state.
    pub fn snapshot(&self) -> Option<PairingSessionSnapshot> {
        self.inner
            .lock()
            .unwrap()
            .as_ref()
            .map(|s| PairingSessionSnapshot {
                otp: s.otp.clone(),
                qr_matrix: s.qr_matrix.clone(),
                qr_size: s.qr_size,
                created_at: s.created_at,
                expires_at: s.created_at + PAIR_CODE_LIFETIME_SECS,
                device_id: s.device_id.clone(),
                hostname: s.hostname.clone(),
                port: s.port,
                consumed: s.consumed,
                cancelled: s.cancelled,
            })
    }

    /// Validate a pairing code against the active session.
    /// On success: consumes session (one-time use).
    /// On failure: does not mutate session.
    pub fn validate_code(&self, code: &str) -> ValidationResult {
        let mut guard = self.inner.lock().unwrap();
        let session = match guard.as_ref() {
            Some(s) => s,
            None => return ValidationResult::NoSession,
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if session.consumed {
            return ValidationResult::AlreadyConsumed;
        }
        if session.cancelled {
            return ValidationResult::Cancelled;
        }
        if now - session.created_at > PAIR_CODE_LIFETIME_SECS {
            return ValidationResult::Expired;
        }
        if session.otp != code {
            return ValidationResult::CodeMismatch;
        }

        // Consume
        if let Some(ref mut s) = *guard {
            s.consumed = true;
        }
        ValidationResult::Accepted
    }

    /// Cancel the current session.
    pub fn cancel_session(&self) {
        if let Some(ref mut s) = *self.inner.lock().unwrap() {
            s.cancelled = true;
        }
    }

    /// Check if active session is expired (without consuming).
    #[allow(dead_code)]
    pub fn is_expired(&self) -> bool {
        let guard = self.inner.lock().unwrap();
        guard.as_ref().map_or(false, |s| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            now - s.created_at > PAIR_CODE_LIFETIME_SECS
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    Accepted,
    NoSession,
    AlreadyConsumed,
    Cancelled,
    Expired,
    CodeMismatch,
}

pub type SharedPairingManager = std::sync::Arc<PairingManager>;

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_mgr() -> PairingManager {
        let m = PairingManager::new();
        m.generate_session("test-device", "test-pc", 9742);
        m
    }

    #[test]
    fn generate_session_creates_active_session() {
        let m = PairingManager::new();
        assert!(m.snapshot().is_none());
        m.generate_session("d1", "h1", 9742);
        assert!(m.snapshot().is_some());
    }

    #[test]
    fn generated_code_is_six_digits() {
        let snap = PairingManager::new().generate_session("d1", "h1", 9742);
        assert_eq!(snap.otp.len(), 6);
        assert!(snap.otp.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn active_session_snapshot_exposes_current_session() {
        let m = PairingManager::new();
        m.generate_session("d1", "h1", 9742);
        let snap = m.snapshot().unwrap();
        assert_eq!(snap.device_id, "d1");
        assert_eq!(snap.hostname, "h1");
        assert_eq!(snap.port, 9742);
        assert!(!snap.consumed);
        assert!(!snap.cancelled);
    }

    #[test]
    fn invalid_code_is_rejected() {
        let m = fresh_mgr();
        assert_eq!(m.validate_code("000000"), ValidationResult::CodeMismatch);
    }

    #[test]
    fn valid_code_is_accepted() {
        let m = PairingManager::new();
        let snap = m.generate_session("d1", "h1", 9742);
        assert_eq!(m.validate_code(&snap.otp), ValidationResult::Accepted);
    }

    #[test]
    fn successful_validation_consumes_session() {
        let m = PairingManager::new();
        let snap = m.generate_session("d1", "h1", 9742);
        assert_eq!(m.validate_code(&snap.otp), ValidationResult::Accepted);
        // Second use fails
        assert_eq!(
            m.validate_code(&snap.otp),
            ValidationResult::AlreadyConsumed
        );
    }

    #[test]
    fn no_session_rejects() {
        let m = PairingManager::new();
        assert_eq!(m.validate_code("123456"), ValidationResult::NoSession);
    }

    #[test]
    fn cancelled_session_is_rejected() {
        let m = fresh_mgr();
        m.cancel_session();
        assert_eq!(m.validate_code("000000"), ValidationResult::Cancelled);
    }

    #[test]
    fn cancelled_session_rejects_valid_code() {
        let m = PairingManager::new();
        let snap = m.generate_session("d1", "h1", 9742);
        m.cancel_session();
        assert_eq!(m.validate_code(&snap.otp), ValidationResult::Cancelled);
    }

    #[test]
    fn snapshot_shows_expiry() {
        let m = PairingManager::new();
        let snap = m.generate_session("d1", "h1", 9742);
        assert!(snap.expires_at > snap.created_at);
        assert_eq!(snap.expires_at - snap.created_at, PAIR_CODE_LIFETIME_SECS);
    }

    /// ponytail: clock-based expiry test uses real wall clock.
    /// Fast enough in practice — add mock clock if proven flaky.
    #[test]
    fn expired_code_is_rejected() {
        let m = PairingManager::new();
        let snap = m.generate_session("d1", "h1", 9742);
        // Simulate expiry by bumping created_at backward
        {
            let mut guard = m.inner.lock().unwrap();
            if let Some(ref mut s) = *guard {
                s.created_at = 0;
            }
        }
        assert_eq!(m.validate_code(&snap.otp), ValidationResult::Expired);
    }

    #[test]
    fn validation_reason_code_maps_all_variants() {
        assert_eq!(
            validation_reason_code(&ValidationResult::Accepted),
            "accepted"
        );
        assert_eq!(
            validation_reason_code(&ValidationResult::NoSession),
            "no_session"
        );
        assert_eq!(
            validation_reason_code(&ValidationResult::AlreadyConsumed),
            "already_used"
        );
        assert_eq!(
            validation_reason_code(&ValidationResult::Cancelled),
            "cancelled"
        );
        assert_eq!(
            validation_reason_code(&ValidationResult::Expired),
            "expired"
        );
        assert_eq!(
            validation_reason_code(&ValidationResult::CodeMismatch),
            "code_mismatch"
        );
    }
}
