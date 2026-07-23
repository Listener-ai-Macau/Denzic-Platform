//! OTA package manifest (schema_version 2) parsing and validation.
//!
//! The schema constants are generated from `ota/protocol/ota_manifest_v2.json`
//! (see `generated_manifest.rs`). The parser accepts both snake_case and
//! camelCase field spellings, mirroring the behavior product hosts relied on.
//! Product policy (package type, project name, message labels, chunk size) is
//! injected through [`OtaManifestPolicy`] so error semantics stay aligned with
//! the product adapters that used to own this logic.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    GATT_CONTROL_UUID, GATT_DATA_UUID, GATT_SERVICE_UUID, GATT_STATUS_UUID, MANIFEST_CHANNELS,
    MANIFEST_DEFAULT_GATT_CHUNK_BYTES, MANIFEST_ROLLBACK_METHODS, MANIFEST_SCHEMA_VERSION,
    MANIFEST_SHA256_HEX_CHARS, PROTOCOL_NAME, PROTOCOL_VERSION,
};

/// Product policy applied to a normalized manifest. Pure schema facts (field
/// presence, types, channel and rollback-method enums, SHA-256 shape) are
/// checked by the platform; this struct carries only the product-specific
/// expectations and the label prefixes used in error messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtaManifestPolicy {
    pub package_type: String,
    pub project: String,
    /// Label used for protocol-version errors, e.g. "Listener OTA".
    pub protocol_label: String,
    /// Label used for package-level errors, e.g. "Listener OTA v1".
    pub package_label: String,
    pub firmware_capability: String,
    pub gatt_chunk_bytes: u64,
}

impl OtaManifestPolicy {
    /// Builds a policy bound to the `denzic_ota_v1` protocol constants; the
    /// product supplies only its package identity and message labels.
    pub fn denzic_ota_v1(
        package_type: &str,
        project: &str,
        protocol_label: &str,
        package_label: &str,
    ) -> Self {
        Self {
            package_type: package_type.to_string(),
            project: project.to_string(),
            protocol_label: protocol_label.to_string(),
            package_label: package_label.to_string(),
            firmware_capability: PROTOCOL_NAME.to_string(),
            gatt_chunk_bytes: MANIFEST_DEFAULT_GATT_CHUNK_BYTES,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OtaManifest {
    pub schema_version: u64,
    pub package_type: String,
    pub project: String,
    pub version: String,
    pub protocol_name: String,
    pub protocol_version: u64,
    pub hardware_revision: String,
    pub min_desktop_version: String,
    pub channel: String,
    pub file_name: String,
    pub file_size_bytes: u64,
    pub file_sha256: String,
    pub firmware_capability: String,
    pub gatt_service_uuid: String,
    pub gatt_control_uuid: String,
    pub gatt_data_uuid: String,
    pub gatt_confirm_uuid: Option<String>,
    #[serde(default)]
    pub gatt_status_uuid: Option<String>,
    pub gatt_chunk_bytes: u64,
    pub rollback_instructions: Vec<String>,
    pub recovery_instructions: Vec<String>,
}

impl OtaManifest {
    pub fn is_denzic_ota_v1(&self) -> bool {
        self.protocol_name == PROTOCOL_NAME
    }
}

pub fn parse_manifest(value: &Value, policy: &OtaManifestPolicy) -> Result<OtaManifest, String> {
    let schema_version = require_u64(
        value
            .get("schema_version")
            .or_else(|| value.get("schemaVersion")),
        "schema_version",
    )?;
    if schema_version == MANIFEST_SCHEMA_VERSION {
        parse_manifest_v2(value, schema_version, policy)
    } else {
        Err(format!(
            "Unsupported OTA manifest schema_version {schema_version}."
        ))
    }
}

fn parse_manifest_v2(
    value: &Value,
    schema_version: u64,
    policy: &OtaManifestPolicy,
) -> Result<OtaManifest, String> {
    let firmware = require_object(value.get("firmware"), "firmware")?;
    let requirements = require_object(value.get("requirements"), "requirements")?;
    let protocol = require_object(value.get("protocol"), "protocol")?;
    let gatt = require_object(protocol.get("gatt"), "protocol.gatt")?;
    let ble_identity = require_object(
        value
            .get("ble_identity")
            .or_else(|| value.get("bleIdentity")),
        "ble_identity",
    )?;
    let dis = require_object(ble_identity.get("dis"), "ble_identity.dis")?;
    let rollback = require_object(value.get("rollback"), "rollback")?;
    let recovery = require_object(value.get("recovery"), "recovery")?;

    require_string(
        value
            .get("created_at_utc")
            .or_else(|| value.get("createdAtUtc")),
        "created_at_utc",
    )?;
    require_string(
        firmware
            .get("git_commit")
            .or_else(|| firmware.get("gitCommit")),
        "firmware.git_commit",
    )?;
    require_bool(
        firmware
            .get("git_dirty")
            .or_else(|| firmware.get("gitDirty")),
        "firmware.git_dirty",
    )?;
    require_string(firmware.get("target"), "firmware.target")?;
    require_string(ble_identity.get("name"), "ble_identity.name")?;
    require_string(ble_identity.get("appearance"), "ble_identity.appearance")?;
    require_string(dis.get("model"), "ble_identity.dis.model")?;
    require_string(
        dis.get("hardware_revision")
            .or_else(|| dis.get("hardwareRevision")),
        "ble_identity.dis.hardware_revision",
    )?;
    require_string(
        dis.get("firmware_revision")
            .or_else(|| dis.get("firmwareRevision")),
        "ble_identity.dis.firmware_revision",
    )?;

    if !require_bool(rollback.get("supported"), "rollback.supported")? {
        return Err("rollback.supported must be true.".to_string());
    }
    let rollback_method = require_string(rollback.get("method"), "rollback.method")?;
    if !MANIFEST_ROLLBACK_METHODS.contains(&rollback_method.as_str()) {
        return Err(format!("Unsupported rollback.method {rollback_method}."));
    }
    let factory_reflash = require_string(
        recovery
            .get("factory_reflash")
            .or_else(|| recovery.get("factoryReflash")),
        "recovery.factory_reflash",
    )?;
    let serial_commands = require_string(
        recovery
            .get("serial_commands")
            .or_else(|| recovery.get("serialCommands")),
        "recovery.serial_commands",
    )?;

    let manifest = OtaManifest {
        schema_version,
        package_type: policy.package_type.clone(),
        project: require_string(firmware.get("project"), "firmware.project")?,
        version: require_string(firmware.get("version"), "firmware.version")?,
        protocol_name: require_string(protocol.get("name"), "protocol.name")?,
        protocol_version: require_u64(protocol.get("version"), "protocol.version")?,
        hardware_revision: require_string(
            requirements
                .get("hardware_revision")
                .or_else(|| requirements.get("hardwareRevision")),
            "requirements.hardware_revision",
        )?,
        min_desktop_version: require_string(
            requirements
                .get("min_desktop_version")
                .or_else(|| requirements.get("minDesktopVersion")),
            "requirements.min_desktop_version",
        )?,
        channel: require_channel(value.get("channel"))?,
        file_name: require_string(firmware.get("file"), "firmware.file")?,
        file_size_bytes: require_u64(
            firmware
                .get("size_bytes")
                .or_else(|| firmware.get("sizeBytes")),
            "firmware.size_bytes",
        )?,
        file_sha256: require_string(firmware.get("sha256"), "firmware.sha256")?
            .to_ascii_lowercase(),
        firmware_capability: require_string(
            protocol
                .get("firmware_capability")
                .or_else(|| protocol.get("firmwareCapability")),
            "protocol.firmware_capability",
        )?,
        gatt_service_uuid: optional_gatt_string(
            Some(gatt),
            "service_uuid",
            "serviceUuid",
            GATT_SERVICE_UUID,
        )?,
        gatt_control_uuid: optional_gatt_string(
            Some(gatt),
            "control_uuid",
            "controlUuid",
            GATT_CONTROL_UUID,
        )?,
        gatt_data_uuid: optional_gatt_string(Some(gatt), "data_uuid", "dataUuid", GATT_DATA_UUID)?,
        gatt_confirm_uuid: optional_gatt_optional_string(
            Some(gatt),
            "confirm_uuid",
            "confirmUuid",
        )?,
        gatt_status_uuid: optional_gatt_optional_string(Some(gatt), "status_uuid", "statusUuid")?,
        gatt_chunk_bytes: optional_gatt_u64(
            Some(gatt),
            "chunk_bytes",
            "chunkBytes",
            policy.gatt_chunk_bytes,
        )?,
        rollback_instructions: require_instructions(
            rollback.get("instructions"),
            "rollback.instructions",
        )?,
        recovery_instructions: vec![factory_reflash, serial_commands],
    };
    validate_manifest(manifest, policy)
}

pub fn validate_manifest(
    manifest: OtaManifest,
    policy: &OtaManifestPolicy,
) -> Result<OtaManifest, String> {
    if !manifest.is_denzic_ota_v1() {
        return Err(format!(
            "Unsupported OTA protocol {}.",
            manifest.protocol_name
        ));
    }
    if manifest.package_type != policy.package_type {
        return Err(format!(
            "ota_manifest.json package_type must be {}.",
            policy.package_type
        ));
    }
    if manifest.protocol_version != u64::from(PROTOCOL_VERSION) {
        return Err(format!(
            "{} protocol.version must be {}, got {}.",
            policy.protocol_label, PROTOCOL_VERSION, manifest.protocol_version
        ));
    }
    if manifest.project != policy.project {
        return Err(format!(
            "{} project must be {}.",
            policy.package_label, policy.project
        ));
    }
    if manifest.firmware_capability != policy.firmware_capability {
        return Err(format!(
            "{} package requires unsupported firmware capability.",
            policy.package_label
        ));
    }
    if !uuid_eq(&manifest.gatt_service_uuid, GATT_SERVICE_UUID)
        || !uuid_eq(&manifest.gatt_control_uuid, GATT_CONTROL_UUID)
        || !uuid_eq(&manifest.gatt_data_uuid, GATT_DATA_UUID)
        || manifest
            .gatt_status_uuid
            .as_deref()
            .map_or(true, |value| !uuid_eq(value, GATT_STATUS_UUID))
    {
        return Err(format!(
            "{} package uses an unsupported GATT boundary.",
            policy.package_label
        ));
    }
    if manifest.gatt_confirm_uuid.is_some() {
        return Err(format!(
            "{} must use status_uuid, not confirm_uuid.",
            policy.package_label
        ));
    }
    if manifest.gatt_chunk_bytes != policy.gatt_chunk_bytes {
        return Err(format!(
            "{} chunk size must be {} bytes, got {}.",
            policy.package_label, policy.gatt_chunk_bytes, manifest.gatt_chunk_bytes
        ));
    }
    if manifest.file_size_bytes == 0 {
        return Err("file.size_bytes must be greater than zero.".to_string());
    }
    if !is_lower_sha256(&manifest.file_sha256) {
        return Err("file.sha256 must be lowercase SHA256 hex.".to_string());
    }
    Ok(manifest)
}

fn require_object<'a>(
    value: Option<&'a Value>,
    field: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{field} must be an object."))
}

fn require_string(value: Option<&Value>, field: &str) -> Result<String, String> {
    let value = value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{field} must be a non-empty string."))?;
    Ok(value.to_string())
}

fn require_u64(value: Option<&Value>, field: &str) -> Result<u64, String> {
    value
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{field} must be a number."))
}

fn require_bool(value: Option<&Value>, field: &str) -> Result<bool, String> {
    value
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("{field} must be a boolean."))
}

fn require_channel(value: Option<&Value>) -> Result<String, String> {
    let channel = require_string(value, "channel")?;
    if MANIFEST_CHANNELS.contains(&channel.as_str()) {
        Ok(channel)
    } else {
        Err(format!(
            "channel must be {}.",
            MANIFEST_CHANNELS.join(" or ")
        ))
    }
}

fn uuid_eq(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn require_instructions(value: Option<&Value>, field: &str) -> Result<Vec<String>, String> {
    if let Some(text) = value.and_then(Value::as_str) {
        let text = text.trim();
        if !text.is_empty() {
            return Ok(vec![text.to_string()]);
        }
    }
    let Some(items) = value.and_then(Value::as_array) else {
        return Err(format!("{field} must be an array."));
    };
    let strings: Vec<String> = items
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect();
    if strings.is_empty() {
        return Err(format!("{field} must contain at least one instruction."));
    }
    Ok(strings)
}

fn optional_gatt_string(
    gatt: Option<&serde_json::Map<String, Value>>,
    snake: &str,
    camel: &str,
    default_value: &str,
) -> Result<String, String> {
    match gatt {
        Some(gatt) => require_string(
            gatt.get(snake).or_else(|| gatt.get(camel)),
            &format!("protocol.gatt.{snake}"),
        ),
        None => Ok(default_value.to_string()),
    }
}

fn optional_gatt_optional_string(
    gatt: Option<&serde_json::Map<String, Value>>,
    snake: &str,
    camel: &str,
) -> Result<Option<String>, String> {
    let Some(gatt) = gatt else {
        return Ok(None);
    };
    let value = gatt.get(snake).or_else(|| gatt.get(camel));
    match value {
        Some(_) => require_string(value, &format!("protocol.gatt.{snake}")).map(Some),
        None => Ok(None),
    }
}

fn optional_gatt_u64(
    gatt: Option<&serde_json::Map<String, Value>>,
    snake: &str,
    camel: &str,
    default_value: u64,
) -> Result<u64, String> {
    match gatt {
        Some(gatt) => require_u64(
            gatt.get(snake).or_else(|| gatt.get(camel)),
            &format!("protocol.gatt.{snake}"),
        ),
        None => Ok(default_value),
    }
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == MANIFEST_SHA256_HEX_CHARS
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIRMWARE_SHA256: &str =
        "6d3841935f58db1c3efa67022f2d770184be6fdef93c087bca10c30e70157e84";

    fn policy() -> OtaManifestPolicy {
        OtaManifestPolicy::denzic_ota_v1(
            "listener-firmware-ota",
            "voice-keyboard-firmware",
            "Listener OTA",
            "Listener OTA v1",
        )
    }

    fn manifest_v2(extra: &str) -> String {
        format!(
            r#"{{
  "schema_version": 2,
  "created_at_utc": "2026-05-26T00:00:00Z",
  "channel": "development",
  "firmware": {{
    "project": "voice-keyboard-firmware",
    "version": "1.2.0",
    "git_commit": "{git_commit}",
    "git_dirty": false,
    "target": "esp32s3",
    "file": "firmware_ota.bin",
    "size_bytes": 6,
    "sha256": "{FIRMWARE_SHA256}"
  }},
  "requirements": {{
    "hardware_revision": "keyboard-v1",
    "protocol_version": 1,
    "min_desktop_version": "1.0.0"
  }},
  "protocol": {{
    "name": "{PROTOCOL_NAME}",
    "version": 1,
    "firmware_capability": "{PROTOCOL_NAME}",
    "gatt": {{
      "service_uuid": "{GATT_SERVICE_UUID}",
      "control_uuid": "{GATT_CONTROL_UUID}",
      "data_uuid": "{GATT_DATA_UUID}",
      "status_uuid": "{GATT_STATUS_UUID}",
      "chunk_bytes": 500
    }}
  }},
  "ble_identity": {{
    "name": "listener",
    "appearance": "0x03C1",
    "dis": {{
      "model": "keyboard-v1",
      "hardware_revision": "esp32s3-devkit",
      "firmware_revision": "1.2.0"
    }}
  }},
  "rollback": {{
    "supported": true,
    "method": "esp_idf_bootloader_rollback",
    "instructions": "Rollback on failed pending verify."
  }},
  "recovery": {{
    "factory_reflash": "Use USB factory package.",
    "serial_commands": "~OTA:STATUS"
  }}
  {extra}
}}"#,
            git_commit = "a".repeat(40)
        )
    }

    fn parse(text: &str) -> Result<OtaManifest, String> {
        let value = serde_json::from_str::<Value>(text).expect("fixture is valid JSON");
        parse_manifest(&value, &policy())
    }

    #[test]
    fn parses_valid_schema_v2_manifest() {
        let manifest = parse(&manifest_v2("")).expect("valid manifest parses");

        assert_eq!(manifest.schema_version, MANIFEST_SCHEMA_VERSION);
        assert_eq!(manifest.version, "1.2.0");
        assert_eq!(manifest.gatt_chunk_bytes, MANIFEST_DEFAULT_GATT_CHUNK_BYTES);
        assert_eq!(manifest.file_sha256, FIRMWARE_SHA256);
        assert_eq!(
            manifest.recovery_instructions,
            vec![
                "Use USB factory package.".to_string(),
                "~OTA:STATUS".to_string()
            ]
        );
        assert!(manifest.is_denzic_ota_v1());
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let text = manifest_v2("").replace("\"schema_version\": 2", "\"schema_version\": 1");
        let error = parse(&text).expect_err("schema_version 1 is retired");

        assert_eq!(error, "Unsupported OTA manifest schema_version 1.");
    }

    #[test]
    fn rejects_retired_protocol_name() {
        let text = manifest_v2("").replace(PROTOCOL_NAME, "listener_ble_ota_v2");
        let error = parse(&text).expect_err("retired protocol is rejected");

        assert_eq!(error, "Unsupported OTA protocol listener_ble_ota_v2.");
    }

    #[test]
    fn rejects_chunk_size_mismatch() {
        let text = manifest_v2("").replace("\"chunk_bytes\": 500", "\"chunk_bytes\": 499");
        let error = parse(&text).expect_err("unexpected chunk size is rejected");

        assert_eq!(
            error,
            "Listener OTA v1 chunk size must be 500 bytes, got 499."
        );
    }

    #[test]
    fn rejects_missing_recovery_serial_commands() {
        let error = parse(&manifest_v2(
            r#","recovery":{"factory_reflash":"Use USB factory package."}"#,
        ))
        .expect_err("missing recovery.serial_commands is rejected");

        assert_eq!(
            error,
            "recovery.serial_commands must be a non-empty string."
        );
    }

    #[test]
    fn rejects_invalid_channel() {
        let text = manifest_v2("").replace("\"channel\": \"development\"", "\"channel\": \"beta\"");
        let error = parse(&text).expect_err("unknown channel is rejected");

        assert_eq!(error, "channel must be stable or development.");
    }

    #[test]
    fn rejects_unsupported_rollback_method() {
        let text = manifest_v2("").replace(
            "\"method\": \"esp_idf_bootloader_rollback\"",
            "\"method\": \"manual_reflash\"",
        );
        let error = parse(&text).expect_err("unknown rollback method is rejected");

        assert_eq!(error, "Unsupported rollback.method manual_reflash.");
    }

    #[test]
    fn normalizes_uppercase_sha256() {
        let text = manifest_v2("").replace(FIRMWARE_SHA256, &FIRMWARE_SHA256.to_ascii_uppercase());
        let manifest = parse(&text).expect("uppercase sha256 normalizes to lowercase");

        assert_eq!(manifest.file_sha256, FIRMWARE_SHA256);
    }

    #[test]
    fn rejects_zero_file_size() {
        let text = manifest_v2("").replace("\"size_bytes\": 6", "\"size_bytes\": 0");
        let error = parse(&text).expect_err("zero size is rejected");

        assert_eq!(error, "file.size_bytes must be greater than zero.");
    }

    #[test]
    fn rejects_foreign_gatt_boundary() {
        let text =
            manifest_v2("").replace(GATT_STATUS_UUID, "00000000-0000-4000-8000-000000000000");
        let error = parse(&text).expect_err("foreign GATT boundary is rejected");

        assert_eq!(
            error,
            "Listener OTA v1 package uses an unsupported GATT boundary."
        );
    }

    #[test]
    fn rejects_confirm_uuid() {
        let text = manifest_v2("").replace(
            "\"status_uuid\":",
            "\"confirm_uuid\": \"e571544a-7c41-4650-b0d6-ccebfe1db489\", \"status_uuid\":",
        );
        let error = parse(&text).expect_err("confirm_uuid is rejected");

        assert_eq!(
            error,
            "Listener OTA v1 must use status_uuid, not confirm_uuid."
        );
    }

    #[test]
    fn accepts_camel_case_field_spellings() {
        let text = manifest_v2("")
            .replace("\"schema_version\":", "\"schemaVersion\":")
            .replace("\"ble_identity\":", "\"bleIdentity\":")
            .replace("\"git_commit\":", "\"gitCommit\":")
            .replace(
                "\"hardware_revision\": \"keyboard-v1\"",
                "\"hardwareRevision\": \"keyboard-v1\"",
            );
        let manifest = parse(&text).expect("camelCase spellings parse");

        assert_eq!(manifest.hardware_revision, "keyboard-v1");
    }
}
