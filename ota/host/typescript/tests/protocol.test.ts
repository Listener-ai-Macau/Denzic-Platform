import assert from 'node:assert/strict';
import {
  DENZIC_OTA_V1_CONTROL_BYTES,
  DENZIC_OTA_V1_DATA_HEADER_BYTES,
  DENZIC_OTA_V1_MAGIC,
  DENZIC_OTA_V1_PROTOCOL_NAME,
  DENZIC_OTA_V1_PROTOCOL_VERSION,
  DENZIC_OTA_V1_STATUS_BYTES,
  isDenzicOtaV1Identity,
} from '../src/index.ts';

assert.equal(DENZIC_OTA_V1_PROTOCOL_NAME, 'denzic_ota_v1');
assert.equal(DENZIC_OTA_V1_PROTOCOL_VERSION, 1);
assert.equal(DENZIC_OTA_V1_MAGIC, 'DOV1');
assert.equal(DENZIC_OTA_V1_CONTROL_BYTES, 16);
assert.equal(DENZIC_OTA_V1_DATA_HEADER_BYTES, 4);
assert.equal(DENZIC_OTA_V1_STATUS_BYTES, 24);
assert.equal(isDenzicOtaV1Identity({
  protocolName: 'denzic_ota_v1',
  protocolVersion: 1,
}), true);
assert.equal(isDenzicOtaV1Identity({
  protocolName: 'obsolete_protocol',
  protocolVersion: 1,
}), false);
