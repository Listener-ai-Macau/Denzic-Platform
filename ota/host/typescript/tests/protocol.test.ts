import assert from 'node:assert/strict';
import {
  DENZIC_OTA_V1_CONTROL_BYTES,
  DENZIC_OTA_V1_DATA_HEADER_BYTES,
  DENZIC_OTA_V1_GATT_CONTROL_UUID,
  DENZIC_OTA_V1_GATT_DATA_UUID,
  DENZIC_OTA_V1_GATT_SERVICE_UUID,
  DENZIC_OTA_V1_GATT_STATUS_UUID,
  DENZIC_OTA_V1_MAGIC,
  DENZIC_OTA_V1_PROTOCOL_NAME,
  DENZIC_OTA_V1_PROTOCOL_VERSION,
  DENZIC_OTA_V1_STATUS_BYTES,
  isDenzicOtaV1Identity,
} from '../src/index.ts';

assert.equal(DENZIC_OTA_V1_PROTOCOL_NAME, 'denzic_ota_v1');
assert.equal(DENZIC_OTA_V1_PROTOCOL_VERSION, 1);
assert.equal(DENZIC_OTA_V1_MAGIC, 'DOV1');
assert.equal(DENZIC_OTA_V1_CONTROL_BYTES, 20);
assert.equal(DENZIC_OTA_V1_DATA_HEADER_BYTES, 4);
assert.equal(DENZIC_OTA_V1_STATUS_BYTES, 24);
assert.equal(DENZIC_OTA_V1_GATT_SERVICE_UUID, '1b55f597-f09c-4c7f-9529-adfa64983b06');
assert.equal(DENZIC_OTA_V1_GATT_CONTROL_UUID, '206c5e29-c64d-4392-8180-66463788533c');
assert.equal(DENZIC_OTA_V1_GATT_DATA_UUID, 'fbc4b0fb-6102-4bd1-abe4-e5e90a9a7e12');
assert.equal(DENZIC_OTA_V1_GATT_STATUS_UUID, 'e571544a-7c41-4650-b0d6-ccebfe1db489');
assert.equal(isDenzicOtaV1Identity({
  protocolName: 'denzic_ota_v1',
  protocolVersion: 1,
}), true);
assert.equal(isDenzicOtaV1Identity({
  protocolName: 'obsolete_protocol',
  protocolVersion: 1,
}), false);
