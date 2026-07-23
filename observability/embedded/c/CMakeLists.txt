add_library(denzic_observability_embedded STATIC
    src/denzic_diag_log_gatt_v1.c
    src/denzic_diag_log_store.c
)
target_include_directories(denzic_observability_embedded PUBLIC include)
target_compile_features(denzic_observability_embedded PUBLIC c_std_99)

add_executable(denzic_observability_embedded_test
    tests/test_denzic_observability_v1.c
)
target_link_libraries(denzic_observability_embedded_test PRIVATE denzic_observability_embedded)
add_test(NAME denzic_observability_embedded_test COMMAND denzic_observability_embedded_test)

add_executable(denzic_diag_log_gatt_v1_test
    tests/test_denzic_diag_log_gatt_v1.c
)
target_link_libraries(denzic_diag_log_gatt_v1_test PRIVATE denzic_observability_embedded)
add_test(NAME denzic_diag_log_gatt_v1_test COMMAND denzic_diag_log_gatt_v1_test)

add_executable(denzic_diag_log_store_test
    tests/test_denzic_diag_log_store.c
)
target_link_libraries(denzic_diag_log_store_test PRIVATE denzic_observability_embedded)
add_test(NAME denzic_diag_log_store_test COMMAND denzic_diag_log_store_test)
