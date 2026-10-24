//! Test synchronization utilities for environment variable modifications.

#[cfg(test)]
pub static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
