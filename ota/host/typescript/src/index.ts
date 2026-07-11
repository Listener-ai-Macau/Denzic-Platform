export * from './generated.ts';

import {
  DENZIC_OTA_V1_PROTOCOL_NAME,
  DENZIC_OTA_V1_PROTOCOL_VERSION,
} from './generated.ts';

export interface DenzicOtaV1Identity {
  protocolName: string;
  protocolVersion: number;
}

export function isDenzicOtaV1Identity(value: DenzicOtaV1Identity): boolean {
  return value.protocolName === DENZIC_OTA_V1_PROTOCOL_NAME
    && value.protocolVersion === DENZIC_OTA_V1_PROTOCOL_VERSION;
}

