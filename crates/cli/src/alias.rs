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
        Address::from_hex(input)
            .with_context(|| format!("Invalid address format: {}", input))
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
            "Cannot sign with a raw address '{}'; use a keypair alias (e.g. @alice or alice)",
            input
        );
    } else {
        // backward-compat: bare name without sigil
        input
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

    let private_key_bytes =
        hex::decode(private_key_hex).context("Invalid private key hex")?;

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
