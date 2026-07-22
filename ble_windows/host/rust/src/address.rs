//! Pure-Rust Bluetooth address, name-matching, and device-identity helpers.
//!
//! Everything in this module is platform-neutral: Windows device-instance-id
//! parsing, 48-bit BLE address formatting, advertisement name matching, BTHPORT
//! registry name decoding, and PnP instance-id normalization.

/// Format a 48-bit Bluetooth address as twelve uppercase hex digits.
pub fn format_bluetooth_address(address: u64) -> String {
    format!("{address:012X}")
}

/// Parse a 48-bit Bluetooth address from a Windows device instance id.
///
/// Accepts the `DEV_`-suffixed BTHLE form (`..._DEV_A1B2C3D4E5F6`), any
/// trailing `_<hex>` marker, or any `\`/`/`/`#`/`_`/`-` separated segment that
/// is exactly twelve hex digits.
pub fn parse_bluetooth_address_from_device_id(device_id: &str) -> Option<u64> {
    let upper = device_id.to_ascii_uppercase();
    for marker in ["DEV_", "_"] {
        let Some((_, suffix)) = upper.rsplit_once(marker) else {
            continue;
        };
        let hex: String = suffix
            .chars()
            .take_while(|ch| ch.is_ascii_hexdigit())
            .collect();
        if hex.len() == 12 {
            return u64::from_str_radix(&hex, 16).ok();
        }
    }
    for segment in upper.rsplit(|ch: char| matches!(ch, '\\' | '/' | '#' | '_' | '-')) {
        if let Some(address) = parse_bluetooth_address_hex_exact(segment) {
            return Some(address);
        }
    }
    None
}

/// Parse a 48-bit Bluetooth address from free text (`A1B2C3D4E5F6`,
/// `A1:B2:C3:D4:E5:F6`, `A1-B2-C3-D4-E5-F6`).
pub fn parse_bluetooth_address_hex(value: &str) -> Option<u64> {
    parse_bluetooth_address_hex_exact(value)
}

fn parse_bluetooth_address_hex_exact(value: &str) -> Option<u64> {
    let hex: String = value.chars().filter(|ch| ch.is_ascii_hexdigit()).collect();
    if hex.len() != 12 {
        return None;
    }
    u64::from_str_radix(&hex, 16).ok()
}

/// Render bytes as uppercase hex, or `-` when empty.
pub fn hex_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "-".to_string();
    }
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join("")
}

/// Append `address` to `addresses` unless already present.
pub fn push_unique_address(addresses: &mut Vec<u64>, address: u64) {
    if !addresses.contains(&address) {
        addresses.push(address);
    }
}

/// Case-insensitive, whitespace-tolerant BLE display-name equality.
pub fn bluetooth_name_matches_expected(name: &str, expected_name: &str) -> bool {
    let trimmed = name.trim();
    !trimmed.is_empty() && trimmed.eq_ignore_ascii_case(expected_name.trim())
}

/// [`bluetooth_name_matches_expected`] against any of `target_names`.
pub fn bluetooth_name_matches_any(name: &str, target_names: &[String]) -> bool {
    target_names
        .iter()
        .any(|target| bluetooth_name_matches_expected(name, target))
}

/// Advertisement local-name equality (trimmed, ASCII case-insensitive).
pub fn ble_advertisement_name_matches(name: &str, expected_name: &str) -> bool {
    name.trim().eq_ignore_ascii_case(expected_name.trim())
}

/// Decode a Swift Pair (Microsoft company id 0x0006) manufacturer-data
/// display-name payload.
pub fn swift_pair_display_name_from_manufacturer_entry(
    company_id: u16,
    bytes: &[u8],
) -> Option<String> {
    if company_id != 0x0006 {
        return None;
    }

    let payload = if bytes.len() >= 5 && bytes[0] == 0x06 && bytes[1] == 0x00 {
        &bytes[2..]
    } else {
        bytes
    };
    if payload.len() <= 3 || payload[0] != 0x03 {
        return None;
    }

    let name = String::from_utf8_lossy(&payload[3..])
        .trim_matches(char::from(0))
        .trim()
        .to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Decode a BTHPORT registry `Name` value (UTF-16LE when it looks like it,
/// otherwise UTF-8), trimming NUL padding.
pub fn decode_bthport_device_name(bytes: &[u8]) -> String {
    let mut utf16_end = bytes.len();
    while utf16_end >= 2 && bytes[utf16_end - 1] == 0 && bytes[utf16_end - 2] == 0 {
        utf16_end -= 2;
    }
    let utf16_candidate = &bytes[..utf16_end];
    if utf16_candidate.len() >= 2 && utf16_candidate.len() % 2 == 0 {
        let zero_high_bytes = utf16_candidate
            .chunks_exact(2)
            .filter(|pair| pair[1] == 0)
            .count();
        if zero_high_bytes * 2 >= utf16_candidate.len() {
            let utf16: Vec<u16> = utf16_candidate
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .collect();
            return String::from_utf16_lossy(&utf16)
                .trim_matches('\0')
                .trim()
                .to_string();
        }
    }

    let end = bytes
        .iter()
        .rposition(|byte| *byte != 0)
        .map(|index| index + 1)
        .unwrap_or(0);
    let trimmed = &bytes[..end];
    String::from_utf8_lossy(trimmed)
        .trim_matches('\0')
        .trim()
        .to_string()
}

/// Normalize a raw PnP device instance id to the canonical
/// `BTHLE\...` / `BTHLEDEVICE\...` / `HID\...` form, stripping wrapper
/// prefixes, `#` separators, and trailing class GUIDs. Returns `None` for
/// ids outside the BLE/HID families.
pub fn normalize_pnp_device_instance_id(raw_id: &str) -> Option<String> {
    let trimmed = raw_id.trim().trim_matches('\0');
    if trimmed.is_empty() {
        return None;
    }
    let upper = trimmed.to_ascii_uppercase();
    let start = [
        "BTHLE\\",
        "BTHLE#",
        "BTHLEDEVICE\\",
        "BTHLEDEVICE#",
        "HID\\",
        "HID#",
    ]
    .iter()
    .filter_map(|marker| upper.find(marker))
    .min()?;
    let mut value = trimmed[start..].to_string();
    if let Some(guid_marker) = value.find("#{") {
        value.truncate(guid_marker);
    }
    if value.contains('#') {
        value = value.replace('#', "\\");
    }
    let normalized_upper = value.to_ascii_uppercase();
    if !normalized_upper.starts_with("BTHLE\\")
        && !normalized_upper.starts_with("BTHLEDEVICE\\")
        && !normalized_upper.starts_with("HID\\")
    {
        return None;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_address_as_twelve_uppercase_hex_digits() {
        assert_eq!(format_bluetooth_address(0xD41A50FBF35E), "D41A50FBF35E");
        assert_eq!(format_bluetooth_address(0x1), "000000000001");
    }

    #[test]
    fn parses_address_from_bthle_device_ids() {
        assert_eq!(
            parse_bluetooth_address_from_device_id(
                r"BTHLE\DEV_D41A50FBF35E\7&1234&0&00D41A50FBF35E"
            ),
            Some(0xD41A50FBF35E)
        );
        assert_eq!(
            parse_bluetooth_address_from_device_id(
                r"SWD\DEVICECONTROLQUEUE\BTHLEDEVICE#{00002a26-0000-1000-8000-00805f9b34fb}#8&1&2_D41A50FBF35E"
            ),
            Some(0xD41A50FBF35E)
        );
        assert_eq!(
            parse_bluetooth_address_from_device_id(r"USB\VID_0000"),
            None
        );
        assert_eq!(parse_bluetooth_address_from_device_id(""), None);
    }

    #[test]
    fn parses_free_text_addresses() {
        assert_eq!(
            parse_bluetooth_address_hex("D41A50FBF35E"),
            Some(0xD41A50FBF35E)
        );
        assert_eq!(
            parse_bluetooth_address_hex("D4:1A:50:FB:F3:5E"),
            Some(0xD41A50FBF35E)
        );
        assert_eq!(
            parse_bluetooth_address_hex("d4-1a-50-fb-f3-5e"),
            Some(0xD41A50FBF35E)
        );
        assert_eq!(parse_bluetooth_address_hex("D41A50FBF35"), None);
        assert_eq!(parse_bluetooth_address_hex(""), None);
    }

    #[test]
    fn hex_bytes_renders_uppercase_or_dash() {
        assert_eq!(hex_bytes(&[0x0A, 0xFF]), "0AFF");
        assert_eq!(hex_bytes(&[]), "-");
    }

    #[test]
    fn push_unique_address_deduplicates() {
        let mut addresses = vec![1u64, 2];
        push_unique_address(&mut addresses, 2);
        push_unique_address(&mut addresses, 3);
        assert_eq!(addresses, vec![1, 2, 3]);
    }

    #[test]
    fn name_matching_trims_and_ignores_ascii_case() {
        assert!(bluetooth_name_matches_expected(
            " OfficeType01 ",
            "officetype01"
        ));
        assert!(!bluetooth_name_matches_expected("", "x"));
        assert!(!bluetooth_name_matches_expected("x", ""));
        assert!(bluetooth_name_matches_any(
            "b",
            &["a".to_string(), "B".to_string()]
        ));
        assert!(ble_advertisement_name_matches(" Companion ", "companion"));
        assert!(!ble_advertisement_name_matches("Companion2", "companion"));
    }

    #[test]
    fn swift_pair_manufacturer_data_exposes_display_name() {
        assert_eq!(
            swift_pair_display_name_from_manufacturer_entry(
                0x0006,
                &[0x03, 0x00, 0x80, b'D', b'e', b'v', b'i', b'c', b'e']
            ),
            Some("Device".to_string())
        );
        assert_eq!(
            swift_pair_display_name_from_manufacturer_entry(
                0x0006,
                &[0x06, 0x00, 0x03, 0x00, 0x80, b'D', b'e', b'v', b'i', b'c', b'e']
            ),
            Some("Device".to_string())
        );
        assert_eq!(
            swift_pair_display_name_from_manufacturer_entry(
                0x004C,
                &[0x03, 0x00, 0x80, b'D', b'e', b'v', b'i', b'c', b'e']
            ),
            None
        );
        assert_eq!(
            swift_pair_display_name_from_manufacturer_entry(0x0006, b"\x03"),
            None
        );
    }

    #[test]
    fn decodes_bthport_names_from_utf16_and_utf8() {
        assert_eq!(decode_bthport_device_name(b"Device\0\0"), "Device");
        let utf16: Vec<u8> = "Device".encode_utf16().flat_map(u16::to_le_bytes).collect();
        assert_eq!(decode_bthport_device_name(&utf16), "Device");
        assert_eq!(decode_bthport_device_name(&[]), "");
    }

    #[test]
    fn normalizes_pnp_instance_ids() {
        assert_eq!(
            normalize_pnp_device_instance_id(
                r#"\\?\BTHLE#DEV_D41A50FBF35E#9&B465B9E&0&D41A50FBF35E#{0000180a-0000-1000-8000-00805f9b34fb}"#
            ),
            Some(r#"BTHLE\DEV_D41A50FBF35E\9&B465B9E&0&D41A50FBF35E"#.to_string())
        );
        assert_eq!(
            normalize_pnp_device_instance_id(
                r#"HID\{00001812-0000-1000-8000-00805F9B34FB}_DEV_VID&0216C0_PID&05DF_REV&0001_FD2F988DB40D&COL01\A&220D8BA7&0&0000"#
            ),
            Some(
                r#"HID\{00001812-0000-1000-8000-00805F9B34FB}_DEV_VID&0216C0_PID&05DF_REV&0001_FD2F988DB40D&COL01\A&220D8BA7&0&0000"#
                    .to_string()
            )
        );
        assert_eq!(
            normalize_pnp_device_instance_id(r"USB\VID_0000&PID_0000"),
            None
        );
        assert_eq!(normalize_pnp_device_instance_id(""), None);
    }
}
