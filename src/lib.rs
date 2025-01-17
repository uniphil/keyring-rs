//! # Keyring library
//!
//! Allows for setting and getting passwords on Linux, OSX, and Windows

pub mod credential;
pub mod error;

use credential::{Platform, PlatformCredential};
pub use error::{Error, Result};

// compile-time Platform known at runtime
pub fn platform() -> Platform {
    platform::platform()
}

// Platform-specific implementations
#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "windows", path = "windows.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
mod platform;

#[derive(Debug)]
pub struct Entry {
    target: PlatformCredential,
}

impl Entry {
    // Create an entry for the given service and username.
    // This maps to a target credential in the default keychain.
    pub fn new(service: &str, username: &str) -> Entry {
        Entry {
            target: credential::default_target(&platform(), None, service, username),
        }
    }

    // Create an entry for the given target, service, and username.
    // On Linux and Mac, the target is interpreted as naming the collection/keychain
    // to store the credential.  On Windows, the target is used directly as
    // the _target name_ of the credential.
    pub fn new_with_target(target: &str, service: &str, username: &str) -> Entry {
        Entry {
            target: credential::default_target(&platform(), Some(target), service, username),
        }
    }

    // Create an entry that uses the given credential for storage.  Callers can use
    // their own algorithm to produce a platform-specific credential spec for the
    // given service and username and then call this entry with that value.
    pub fn new_with_credential(target: &PlatformCredential) -> Result<Entry> {
        if target.matches_platform(&platform()) {
            Ok(Entry {
                target: target.clone(),
            })
        } else {
            Err(Error::WrongCredentialPlatform)
        }
    }

    // Set the password for this item.  Any other platform-specific
    // annotations are determined by the mapper that was used
    // to create the credential.
    pub fn set_password(&self, password: &str) -> Result<()> {
        platform::set_password(&self.target, password)
    }

    // Retrieve the password saved for this item.
    // Returns a `NoEntry` error is there isn't one.
    pub fn get_password(&self) -> Result<String> {
        let mut map = self.target.clone();
        platform::get_password(&mut map)
    }

    // Retrieve the password and all the other fields
    // set in the platform-specific credential.  This
    // allows retrieving metadata on the credential that
    // were saved by external applications.
    pub fn get_password_and_credential(&self) -> Result<(String, PlatformCredential)> {
        let mut map = self.target.clone();
        let password = platform::get_password(&mut map)?;
        Ok((password, map))
    }

    // Delete the password for this item.  (Although the item
    // itself follows the Rust structure lifecycle, deleting
    // the password deletes the platform credential from secure storage.)
    pub fn delete_password(&self) -> Result<()> {
        platform::delete_password(&self.target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credential::default_target;

    #[test]
    fn test_default_initial_and_retrieved_map() {
        let name = generate_random_string();
        let expected_target = default_target(&platform(), None, &name, &name);
        let entry = Entry::new(&name, &name);
        assert_eq!(entry.target, expected_target);
        entry.set_password("ignored").unwrap();
        let (_, target) = entry.get_password_and_credential().unwrap();
        assert_eq!(target, expected_target);
        // don't leave password around.
        entry.delete_password().unwrap();
    }

    #[test]
    fn test_targeted_initial_and_retrieved_map() {
        let name = generate_random_string();
        let expected_target = default_target(&platform(), Some(&name), &name, &name);
        let entry = Entry::new_with_target(&name, &name, &name);
        assert_eq!(entry.target, expected_target);
        // can only test targeted credentials on Windows
        if matches!(platform(), Platform::Windows) {
            entry.set_password("ignored").unwrap();
            let (_, target) = entry.get_password_and_credential().unwrap();
            assert_eq!(target, expected_target);
            // don't leave password around.
            entry.delete_password().unwrap();
        }
    }

    fn generate_random_string() -> String {
        // from the Rust Cookbook:
        // https://rust-lang-nursery.github.io/rust-cookbook/algorithms/randomness.html
        use rand::{distributions::Alphanumeric, thread_rng, Rng};
        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect()
    }
}
