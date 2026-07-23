// Generated from ota/protocol/ota_manifest_v2.json. Do not edit.
pub const MANIFEST_NAME: &str = "denzic_ota_manifest_v2";
pub const MANIFEST_SCHEMA_VERSION: u64 = 2;
pub const MANIFEST_FILE_NAME: &str = "ota_manifest.json";
pub const MANIFEST_PACKAGE_FILE_NAME: &str = "firmware_ota.bin";
pub const MANIFEST_FIRMWARE_VERSION_MAX_CHARS: usize = 31;
pub const MANIFEST_SHA256_HEX_CHARS: usize = 64;
pub const MANIFEST_DEFAULT_GATT_CHUNK_BYTES: u64 = 500;
pub const MANIFEST_CHANNELS: [&str; 2] = ["stable", "development"];
pub const MANIFEST_ROLLBACK_METHODS: [&str; 1] = ["esp_idf_bootloader_rollback"];
