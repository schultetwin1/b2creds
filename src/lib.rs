#![crate_name = "b2creds"]

//! This crate contains the logic to read B2 credentials following the same logic
//! used by the B2 CLI.
//!
//! ```no_run
//! let creds = b2creds::Credentials::locate().unwrap();
//! println!("Key ID: {} Key: {}", creds.application_key_id, creds.application_key);
//!```
//!
//! Look at the [`Credentials::locate`], [`Credentials::from_env`]. and
//! [`Credentials::from_file`] to understand how to parse B2 credentials.

mod credentials;
pub use credentials::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
