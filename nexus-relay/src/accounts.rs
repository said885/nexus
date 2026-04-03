#![allow(missing_docs, dead_code)]

//! User account management with 2FA and recovery

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;

/// User account
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct UserAccount {
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String, // Argon2id(2^18, 3, 4GB)
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub is_verified: bool,
    pub verification_token: Option<String>,
    pub verification_expires_at: Option<DateTime<Utc>>,
    
    // 2FA
    pub two_fa_enabled: bool,
    pub two_fa_method: Option<TwoFAMethod>,
    pub two_fa_secret: Option<String>, // TOTP secret or phone number
    pub backup_codes: Vec<String>,    // Encrypted
    
    // Recovery
    pub recovery_email: Option<String>,
    pub recovery_phone: Option<String>,
    pub account_recovery_tokens: Vec<RecoveryToken>,
    
    // Security
    pub password_changed_at: DateTime<Utc>,
    pub failed_login_attempts: u32,
    pub locked_until: Option<DateTime<Utc>>,
    pub ip_whitelist: Vec<String>,
    pub session_timeout_seconds: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum TwoFAMethod {
    TOTP,           // Time-based One-Time Password
    SMS,            // SMS text message
    Email,          // Email code
    Authenticator,  // Authenticator app
}

/// Recovery token for account recovery without 2FA
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RecoveryToken {
    pub token_id: String,
    pub token_hash: String,  // SHA-256 hash
    pub created_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub recovery_method: String, // "email", "phone", "backup_code"
}

/// Login session
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LoginSession {
    pub session_id: String,
    pub user_id: String,
    pub device_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub ip_address: String,
    pub user_agent: String,
    pub is_active: bool,
    pub mfa_verified: bool,
    pub session_token_hash: String, // Token is never stored plaintext
}

impl UserAccount {
    /// Create new user account
    pub(crate) fn new(
        user_id: String,
        username: String,
        email: String,
        password_hash: String,
    ) -> Self {
        Self {
            user_id,
            username,
            email,
            password_hash,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
            is_active: true,
            is_verified: false,
            verification_token: Some(Self::generate_token()),
            verification_expires_at: Some(Utc::now() + Duration::hours(24)),
            two_fa_enabled: false,
            two_fa_method: None,
            two_fa_secret: None,
            backup_codes: Vec::new(),
            recovery_email: None,
            recovery_phone: None,
            account_recovery_tokens: Vec::new(),
            password_changed_at: Utc::now(),
            failed_login_attempts: 0,
            locked_until: None,
            ip_whitelist: Vec::new(),
            session_timeout_seconds: 3600, // 1 hour
        }
    }

    /// Verify email with token
    pub(crate) fn verify_email(&mut self, token: &str) -> Result<(), String> {
        let vtoken = self.verification_token.as_ref()
            .ok_or("No verification token")?;

        if vtoken != token {
            return Err("Invalid verification token".to_string());
        }

        let expires = self.verification_expires_at
            .ok_or("Verification expired")?;

        if Utc::now() > expires {
            return Err("Verification token expired".to_string());
        }

        self.is_verified = true;
        self.verification_token = None;
        self.verification_expires_at = None;
        self.updated_at = Utc::now();

        Ok(())
    }

    /// Enable 2FA
    pub(crate) fn enable_2fa(&mut self, method: TwoFAMethod, secret: String) -> Result<Vec<String>, String> {
        if method != TwoFAMethod::TOTP && method != TwoFAMethod::SMS && method != TwoFAMethod::Email {
            return Err("Invalid 2FA method".to_string());
        }

        self.two_fa_enabled = true;
        self.two_fa_method = Some(method);
        self.two_fa_secret = Some(secret);

        // Generate 8 backup codes
        let backup_codes: Vec<String> = (0..8)
            .map(|_| Self::generate_token())
            .collect();

        // Store encrypted
        self.backup_codes = backup_codes.clone();
        self.updated_at = Utc::now();

        Ok(backup_codes)
    }

    /// Disable 2FA
    pub(crate) fn disable_2fa(&mut self) -> Result<(), String> {
        self.two_fa_enabled = false;
        self.two_fa_method = None;
        self.two_fa_secret = None;
        self.backup_codes.clear();
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Create recovery token
    pub(crate) fn create_recovery_token(&mut self, method: String) -> Result<String, String> {
        let token = Self::generate_token();
        let token_hash = format!("{:x}", md5::compute(token.as_bytes()));

        let recovery_token = RecoveryToken {
            token_id: uuid::Uuid::new_v4().to_string(),
            token_hash,
            created_at: Utc::now(),
            used_at: None,
            expires_at: Utc::now() + Duration::days(30),
            recovery_method: method,
        };

        self.account_recovery_tokens.push(recovery_token);

        Ok(token)
    }

    /// Use recovery token
    pub(crate) fn use_recovery_token(&mut self, token: &str) -> Result<(), String> {
        let token_hash = format!("{:x}", md5::compute(token.as_bytes()));

        for recovery_token in self.account_recovery_tokens.iter_mut() {
            if recovery_token.token_hash == token_hash && recovery_token.used_at.is_none() {
                if Utc::now() > recovery_token.expires_at {
                    return Err("Recovery token expired".to_string());
                }

                recovery_token.used_at = Some(Utc::now());
                self.updated_at = Utc::now();
                return Ok(());
            }
        }

        Err("Invalid recovery token".to_string())
    }

    /// Record failed login attempt
    pub(crate) fn record_failed_login(&mut self) -> Result<(), String> {
        self.failed_login_attempts += 1;

        // Lock account after 5 failed attempts
        if self.failed_login_attempts >= 5 {
            self.locked_until = Some(Utc::now() + Duration::minutes(30));
            return Err("Account locked for 30 minutes".to_string());
        }

        Ok(())
    }

    /// Clear failed login attempts
    pub(crate) fn clear_failed_logins(&mut self) {
        self.failed_login_attempts = 0;
        self.locked_until = None;
    }

    /// Check if account is locked
    pub(crate) fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            Utc::now() < locked_until
        } else {
            false
        }
    }

    /// Change password
    pub(crate) fn change_password(&mut self, new_password_hash: String) -> Result<(), String> {
        self.password_hash = new_password_hash;
        self.password_changed_at = Utc::now();
        self.updated_at = Utc::now();
        self.clear_failed_logins();
        Ok(())
    }

    /// Generate random token
    fn generate_token() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut rng = rand::thread_rng();

        (0..32)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
}

impl LoginSession {
    /// Create new login session
    pub(crate) fn new(
        user_id: String,
        device_id: String,
        ip_address: String,
        user_agent: String,
        session_timeout_seconds: u32,
    ) -> Self {
        let now = Utc::now();

        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            user_id,
            device_id,
            created_at: now,
            expires_at: now + Duration::seconds(session_timeout_seconds as i64),
            last_activity: now,
            ip_address,
            user_agent,
            is_active: true,
            mfa_verified: false,
            session_token_hash: String::new(),
        }
    }

    /// Check if session is valid
    pub(crate) fn is_valid(&self) -> bool {
        self.is_active && Utc::now() < self.expires_at
    }

    /// Extend session
    pub(crate) fn extend(&mut self, session_timeout_seconds: u32) {
        self.expires_at = Utc::now() + Duration::seconds(session_timeout_seconds as i64);
        self.last_activity = Utc::now();
    }

    /// Invalidate session
    pub(crate) fn invalidate(&mut self) {
        self.is_active = false;
    }
}

/// Account manager
pub(crate) struct AccountManager {
    pub accounts: HashMap<String, UserAccount>,
    pub sessions: HashMap<String, LoginSession>,
    pub email_to_user: HashMap<String, String>, // email -> user_id
}

impl AccountManager {
    pub(crate) fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            sessions: HashMap::new(),
            email_to_user: HashMap::new(),
        }
    }

    /// Create or get account
    pub(crate) fn create_account(
        &mut self,
        user_id: String,
        username: String,
        email: String,
        password_hash: String,
    ) -> Result<UserAccount, String> {
        if self.email_to_user.contains_key(&email) {
            return Err("Email already registered".to_string());
        }

        let account = UserAccount::new(user_id.clone(), username, email.clone(), password_hash);

        self.email_to_user.insert(email, user_id.clone());
        self.accounts.insert(user_id, account.clone());

        Ok(account)
    }

    /// Get account
    pub(crate) fn get_account(&self, user_id: &str) -> Option<&UserAccount> {
        self.accounts.get(user_id)
    }

    /// Get mutable account
    pub(crate) fn get_account_mut(&mut self, user_id: &str) -> Option<&mut UserAccount> {
        self.accounts.get_mut(user_id)
    }

    /// Create session
    pub(crate) fn create_session(
        &mut self,
        user_id: String,
        device_id: String,
        ip_address: String,
        user_agent: String,
        session_timeout_seconds: u32,
    ) -> LoginSession {
        let session = LoginSession::new(user_id, device_id, ip_address, user_agent, session_timeout_seconds);
        self.sessions.insert(session.session_id.clone(), session.clone());
        session
    }

    /// Get session
    pub(crate) fn get_session(&self, session_id: &str) -> Option<&LoginSession> {
        self.sessions.get(session_id)
    }

    /// Invalidate session
    pub(crate) fn invalidate_session(&mut self, session_id: &str) -> Result<(), String> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.invalidate();
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }
}

impl Default for AccountManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_creation() {
        let account = UserAccount::new(
            "user1".to_string(),
            "alice".to_string(),
            "alice@example.com".to_string(),
            "hash123".to_string(),
        );

        assert_eq!(account.username, "alice");
        assert!(!account.is_verified);
        assert!(!account.two_fa_enabled);
    }

    #[test]
    fn test_email_verification() {
        let mut account = UserAccount::new(
            "user1".to_string(),
            "alice".to_string(),
            "alice@example.com".to_string(),
            "hash123".to_string(),
        );

        let token = account.verification_token.clone().unwrap();
        assert!(account.verify_email(&token).is_ok());
        assert!(account.is_verified);
    }

    #[test]
    fn test_2fa_setup() {
        let mut account = UserAccount::new(
            "user1".to_string(),
            "alice".to_string(),
            "alice@example.com".to_string(),
            "hash123".to_string(),
        );

        let codes = account.enable_2fa(TwoFAMethod::TOTP, "secret123".to_string());
        assert!(codes.is_ok());
        assert!(account.two_fa_enabled);
        assert_eq!(account.backup_codes.len(), 8);
    }

    #[test]
    fn test_account_lockout() {
        let mut account = UserAccount::new(
            "user1".to_string(),
            "alice".to_string(),
            "alice@example.com".to_string(),
            "hash123".to_string(),
        );

        for _ in 0..5 {
            let _ = account.record_failed_login();
        }

        assert!(account.is_locked());
    }

    #[test]
    fn test_login_session() {
        let session = LoginSession::new(
            "user1".to_string(),
            "device1".to_string(),
            "192.168.1.1".to_string(),
            "Mozilla/5.0".to_string(),
            3600,
        );

        assert!(session.is_valid());
    }
}
