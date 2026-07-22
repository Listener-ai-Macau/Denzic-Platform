#include <assert.h>
#include <string.h>

#include "denzic_ble_windows_v1_generated.h"

int main(void)
{
    assert(strcmp(DENZIC_BLE_WINDOWS_V1_CONTRACT_NAME, "denzic_ble_windows_v1") == 0);
    assert(DENZIC_BLE_WINDOWS_V1_CONTRACT_VERSION == 1u);
    assert(DENZIC_BLE_WINDOWS_V1_DEFAULT_DISCOVERY_TIMEOUT_MS == 15000u);
    assert(DENZIC_BLE_WINDOWS_V1_OPTIONAL_READ_TIMEOUT_MS == 2000u);
    assert(DENZIC_BLE_WINDOWS_V1_CCCD_ENABLE_TIMEOUT_MS == 8000u);
    assert(DENZIC_BLE_WINDOWS_V1_ASYNC_POLL_INTERVAL_MS == 25u);
    assert(DENZIC_BLE_WINDOWS_V1_CREATE_NO_WINDOW_FLAG == 0x08000000u);

    assert(DENZIC_BLE_WINDOWS_V1_CCCD_ENABLE_RETRY_DELAYS_COUNT == 3u);
    assert(denzic_ble_windows_v1_cccd_enable_retry_delays_ms[0] == 250u);
    assert(denzic_ble_windows_v1_cccd_enable_retry_delays_ms[2] == 1500u);

    assert(DENZIC_BLE_WINDOWS_V1_FAILURE_KIND_DEVICE_MISSING == 0);
    assert(DENZIC_BLE_WINDOWS_V1_FAILURE_KIND_BACKGROUND_CONTENTION == 8);
    assert(DENZIC_BLE_WINDOWS_V1_FAILURE_KIND_UNKNOWN == 13);
    return 0;
}
