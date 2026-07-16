#include "denzic_observability_v1_generated.h"

#include <assert.h>

int main(void)
{
    denzic_observability_v1_event_t event;
    denzic_observability_v1_event_init(
        &event,
        0x0102030405060708ull,
        7u,
        125u,
        DENZIC_OBSERVABILITY_V1_EVENT_SOURCE_FIRMWARE,
        DENZIC_OBSERVABILITY_V1_CAPABILITY_BLE);

    assert(event.contract_version == DENZIC_OBSERVABILITY_V1_CONTRACT_VERSION);
    assert(event.correlation_id == 0x0102030405060708ull);
    assert(event.event_sequence == 7u);
    assert(event.monotonic_ms == 125u);
    assert(event.ble_lifecycle_state == DENZIC_OBSERVABILITY_V1_BLE_LIFECYCLE_STATE_UNKNOWN);
    assert(event.error_category == DENZIC_OBSERVABILITY_V1_ERROR_CATEGORY_NONE);
    assert(event.timing_metric == DENZIC_OBSERVABILITY_V1_TIMING_METRIC_NONE);
    assert(event.timing_value_ms == 0u);
    return 0;
}
