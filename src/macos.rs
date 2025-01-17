use security_framework::os::macos::keychain::{SecKeychain, SecPreferencesDomain};
use security_framework::os::macos::passwords::find_generic_password;

use crate::credential::{MacCredential, MacKeychainDomain};
use crate::{Error as ErrorCode, Platform, PlatformCredential, Result};

pub fn platform() -> Platform {
    Platform::MacOs
}

pub use security_framework::base::Error;

fn get_keychain(map: &MacCredential) -> Result<SecKeychain> {
    let domain = match map.domain {
        MacKeychainDomain::User => SecPreferencesDomain::User,
        MacKeychainDomain::System => SecPreferencesDomain::System,
        MacKeychainDomain::Common => SecPreferencesDomain::Common,
        MacKeychainDomain::Dynamic => SecPreferencesDomain::Dynamic,
    };
    match SecKeychain::default_for_domain(domain) {
        Ok(keychain) => Ok(keychain),
        Err(err) => Err(decode_error(err)),
    }
}

pub fn set_password(map: &PlatformCredential, password: &str) -> Result<()> {
    if let PlatformCredential::Mac(map) = map {
        get_keychain(map)?
            .set_generic_password(&map.service, &map.account, password.as_bytes())
            .map_err(decode_error)?;
        Ok(())
    } else {
        Err(ErrorCode::WrongCredentialPlatform)
    }
}

pub fn get_password(map: &mut PlatformCredential) -> Result<String> {
    if let PlatformCredential::Mac(map) = map {
        let (password_bytes, _) =
            find_generic_password(Some(&[get_keychain(map)?]), &map.service, &map.account)
                .map_err(decode_error)?;
        decode_password(password_bytes.to_vec())
    } else {
        Err(ErrorCode::WrongCredentialPlatform)
    }
}

pub fn delete_password(map: &PlatformCredential) -> Result<()> {
    if let PlatformCredential::Mac(map) = map {
        let (_, item) =
            find_generic_password(Some(&[get_keychain(map)?]), &map.service, &map.account)
                .map_err(decode_error)?;
        item.delete();
        Ok(())
    } else {
        Err(ErrorCode::WrongCredentialPlatform)
    }
}

fn decode_password(bytes: Vec<u8>) -> Result<String> {
    // Mac keychain allows non-UTF8 values, passwords from 3rd parties may not be UTF-8.
    String::from_utf8(bytes.clone()).map_err(|_| ErrorCode::BadEncoding(bytes))
}

// The MacOS error codes used here are from:
// https://opensource.apple.com/source/libsecurity_keychain/libsecurity_keychain-78/lib/SecBase.h.auto.html
fn decode_error(err: Error) -> ErrorCode {
    match err.code() {
        -25291 => ErrorCode::NoStorageAccess(err), // errSecNotAvailable
        -25292 => ErrorCode::NoStorageAccess(err), // errSecReadOnly
        -25294 => ErrorCode::NoStorageAccess(err), // errSecNoSuchKeychain
        -25295 => ErrorCode::NoStorageAccess(err), // errSecInvalidKeychain
        -25300 => ErrorCode::NoEntry,              // errSecItemNotFound
        _ => ErrorCode::PlatformFailure(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bad_password() {
        // malformed sequences here taken from:
        // https://www.cl.cam.ac.uk/~mgk25/ucs/examples/UTF-8-test.txt
        for bytes in [b"\x80".to_vec(), b"\xbf".to_vec(), b"\xed\xa0\xa0".to_vec()] {
            match decode_password(bytes.clone()) {
                Err(ErrorCode::BadEncoding(str)) => assert_eq!(str, bytes),
                Err(other) => panic!(
                    "Bad password ({:?}) decode gave wrong error: {}",
                    bytes, other
                ),
                Ok(s) => panic!("Bad password ({:?}) decode gave results: {:?}", bytes, &s),
            }
        }
    }
}
