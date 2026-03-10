//! Account alias resolution.
//!
//! Aliases are written with an `@` sigil (e.g. `@alice`) and map to the
//! address stored in `data/keys/<name>.json`.
//!
//! Valid alias names: `^[a-zA-Z][a-zA-Z0-9_-]*$`
//! Address inputs:    `0x<40 hex chars>`

use anyhow::{bail, Context, Result};
use minichain_core::{Address, Keypair};
use std::fs;
use std::path::Path;

/// Validate that `name` (without the leading `@`) matches `^[a-zA-Z][a-zA-Z0-9_-]*$`.
pub fn validate_alias_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Alias name cannot be empty");
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() {
        bail!(
            "Invalid alias '@{}': must start with a letter (a–z, A–Z)",
            name
        );
    }
    for ch in chars {
        if !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-' {
            bail!(
                "Invalid alias '@{}': only letters, digits, '_' and '-' are allowed",
                name
            );
        }
    }
    Ok(())
}

/// Resolve `input` to an [`Address`].
///
/// Accepted forms:
/// - `@<alias>` — looks up `data/keys/<alias>.json`
/// - `0x<40hex>` — parsed directly as a hex address
///
/// Anything else produces a clear error message.
pub fn resolve_address(data_dir: &Path, input: &str) -> Result<Address> {
    if let Some(name) = input.strip_prefix('@') {
        validate_alias_name(name)?;
        let key_file = data_dir.join("keys").join(format!("{}.json", name));
        if !key_file.exists() {
            bail!(
                "Unknown alias '{}': key file not found at {}",
                input,
                key_file.display()
            );
        }
        let contents = fs::read_to_string(&key_file)
            .with_context(|| format!("Failed to read key file: {}", key_file.display()))?;
        let json: serde_json::Value = serde_json::from_str(&contents)
            .with_context(|| format!("Invalid JSON in key file: {}", key_file.display()))?;
        let addr_str = json
            .get("address")
            .and_then(|v| v.as_str())
            .with_context(|| format!("Missing 'address' field in {}", key_file.display()))?;
        Address::from_hex(addr_str)
            .with_context(|| format!("Invalid address in {}", key_file.display()))
    } else if input.starts_with("0x") || input.starts_with("0X") {
        Address::from_hex(input).with_context(|| format!("Invalid address format: {}", input))
    } else {
        bail!(
            "Expected an address (0x…) or an alias (@name), got: '{}'\n\
             Hint: use @{} if '{}' is a local account alias",
            input,
            input,
            input
        )
    }
}

/// Load a [`Keypair`] from `input`.
///
/// Accepted forms for `--from` style arguments:
/// - `@<alias>` — loads `data/keys/<alias>.json`
/// - `<name>` (no sigil, no `0x`) — same; retained for backward compatibility
///
/// Passing a raw `0x…` address produces a clear error since you need a
/// private key to sign.
pub fn load_keypair_by_ref(data_dir: &Path, input: &str) -> Result<Keypair> {
    let name = if let Some(n) = input.strip_prefix('@') {
        validate_alias_name(n)?;
        n
    } else if input.starts_with("0x") || input.starts_with("0X") {
        bail!(
            "Cannot sign with a raw address '{}'; use a keypair alias (e.g. @alice)",
            input
        );
    } else {
        bail!(
            "Expected a keypair alias starting with '@' (e.g. @alice), got: '{}'",
            input
        );
    };

    let keys_dir = data_dir.join("keys");
    let key_file = keys_dir.join(format!("{}.json", name));
    if !key_file.exists() {
        bail!(
            "Keypair file not found for '{}': expected at {}\n\
             Use 'minichain account new --name {}' to create one.",
            input,
            key_file.display(),
            name
        );
    }

    let contents = fs::read_to_string(&key_file)
        .with_context(|| format!("Failed to read key file: {}", key_file.display()))?;
    let json: serde_json::Value = serde_json::from_str(&contents)
        .with_context(|| format!("Invalid JSON in key file: {}", key_file.display()))?;

    let private_key_hex = json
        .get("private_key")
        .and_then(|v| v.as_str())
        .with_context(|| format!("Missing 'private_key' in {}", key_file.display()))?;

    let private_key_bytes = hex::decode(private_key_hex).context("Invalid private key hex")?;

    if private_key_bytes.len() != 32 {
        bail!(
            "Invalid private key length in {}: expected 32 bytes, got {}",
            key_file.display(),
            private_key_bytes.len()
        );
    }

    let mut private_key = [0u8; 32];
    private_key.copy_from_slice(&private_key_bytes);

    Keypair::from_private_key(&private_key)
        .with_context(|| format!("Failed to create keypair from {}", key_file.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use minichain_core::Keypair;
    use std::fs;
    use tempfile::TempDir;

    // ── helpers ────────────────────────────────────────────────────────────────

    /// Write a well-formed key file for `name` under `<dir>/keys/`.
    fn write_key_file(dir: &TempDir, name: &str, keypair: &Keypair) {
        let keys_dir = dir.path().join("keys");
        fs::create_dir_all(&keys_dir).unwrap();
        let addr = keypair.address();
        let json = serde_json::json!({
            "address": addr.to_hex(),
            "public_key": hex::encode(keypair.public_key.as_bytes()),
            "private_key": hex::encode(keypair.private_key()),
        });
        fs::write(
            keys_dir.join(format!("{}.json", name)),
            serde_json::to_string_pretty(&json).unwrap(),
        )
        .unwrap();
    }

    // ── validate_alias_name ────────────────────────────────────────────────────

    #[test]
    fn valid_alias_names() {
        for name in &["alice", "Bob", "my-account", "acc123", "A", "x1-y2"] {
            assert!(
                validate_alias_name(name).is_ok(),
                "'{}' should be valid",
                name
            );
        }
    }

    #[test]
    fn alias_empty_is_invalid() {
        assert!(validate_alias_name("").is_err());
    }

    #[test]
    fn alias_starting_with_digit_is_invalid() {
        let err = validate_alias_name("0alice").unwrap_err();
        assert!(err.to_string().contains("must start with a letter"));
    }

    #[test]
    fn alias_starting_with_underscore_is_invalid() {
        let err = validate_alias_name("_alice").unwrap_err();
        assert!(err.to_string().contains("must start with a letter"));
    }

    #[test]
    fn alias_with_whitespace_is_invalid() {
        let err = validate_alias_name("my account").unwrap_err();
        assert!(err.to_string().contains("only letters"));
    }

    #[test]
    fn alias_with_at_sign_is_invalid() {
        // '@' should never reach validate_alias_name (it's stripped), but
        // if it somehow does it must be rejected.
        assert!(validate_alias_name("@alice").is_err());
    }

    #[test]
    fn alias_starting_with_0x_prefix_is_invalid() {
        // '0' is a digit — should be caught by the first-char check.
        let err = validate_alias_name("0xdeadbeef").unwrap_err();
        assert!(err.to_string().contains("must start with a letter"));
    }

    // ── resolve_address ────────────────────────────────────────────────────────

    #[test]
    fn resolve_hex_address() {
        let dir = TempDir::new().unwrap();
        let addr_hex = "0x0102030405060708090a0b0c0d0e0f1011121314";
        let addr = resolve_address(dir.path(), addr_hex).unwrap();
        assert_eq!(addr.to_hex(), addr_hex);
    }

    #[test]
    fn resolve_hex_address_without_0x_prefix_is_rejected() {
        let dir = TempDir::new().unwrap();
        // bare hex without 0x must not be silently accepted as an alias
        let err =
            resolve_address(dir.path(), "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef").unwrap_err();
        assert!(err.to_string().contains("Expected an address"));
    }

    #[test]
    fn resolve_alias_returns_correct_address() {
        let dir = TempDir::new().unwrap();
        let kp = Keypair::generate();
        let expected = kp.address();
        write_key_file(&dir, "alice", &kp);

        let resolved = resolve_address(dir.path(), "@alice").unwrap();
        assert_eq!(resolved, expected);
    }

    #[test]
    fn resolve_alias_unknown_errors_clearly() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("keys")).unwrap();
        let err = resolve_address(dir.path(), "@nobody").unwrap_err();
        assert!(err.to_string().contains("Unknown alias '@nobody'"));
    }

    #[test]
    fn resolve_alias_invalid_grammar_errors() {
        let dir = TempDir::new().unwrap();
        let err = resolve_address(dir.path(), "@0bad").unwrap_err();
        assert!(err.to_string().contains("must start with a letter"));
    }

    #[test]
    fn resolve_bare_text_without_sigil_is_rejected() {
        let dir = TempDir::new().unwrap();
        let err = resolve_address(dir.path(), "alice").unwrap_err();
        assert!(err.to_string().contains("Expected an address"));
    }

    // ── load_keypair_by_ref ────────────────────────────────────────────────────

    #[test]
    fn load_keypair_with_at_sigil() {
        let dir = TempDir::new().unwrap();
        let kp = Keypair::generate();
        let expected_addr = kp.address();
        write_key_file(&dir, "alice", &kp);

        let loaded = load_keypair_by_ref(dir.path(), "@alice").unwrap();
        assert_eq!(loaded.address(), expected_addr);
    }

    #[test]
    fn load_keypair_bare_name_without_sigil_is_rejected() {
        let dir = TempDir::new().unwrap();
        let kp = Keypair::generate();
        write_key_file(&dir, "alice", &kp);

        let err = load_keypair_by_ref(dir.path(), "alice").unwrap_err();
        assert!(
            err.to_string().contains("Expected a keypair alias starting with '@'"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn load_keypair_with_raw_address_is_rejected() {
        let dir = TempDir::new().unwrap();
        let err = load_keypair_by_ref(dir.path(), "0x0102030405060708090a0b0c0d0e0f1011121314")
            .unwrap_err();
        assert!(err.to_string().contains("Cannot sign with a raw address"));
    }

    #[test]
    fn load_keypair_unknown_alias_errors_clearly() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("keys")).unwrap();
        let err = load_keypair_by_ref(dir.path(), "@nobody").unwrap_err();
        assert!(err.to_string().contains("Keypair file not found"));
    }

    #[test]
    fn load_keypair_invalid_grammar_after_at_errors() {
        let dir = TempDir::new().unwrap();
        let err = load_keypair_by_ref(dir.path(), "@0bad").unwrap_err();
        assert!(err.to_string().contains("must start with a letter"));
    }
}
