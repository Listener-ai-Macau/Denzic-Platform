add_library(denzic_ota_embedded STATIC
    src/denzic_ota_v1.c
)
target_include_directories(denzic_ota_embedded PUBLIC include)
target_compile_features(denzic_ota_embedded PUBLIC c_std_99)

add_executable(denzic_ota_embedded_test
    tests/test_denzic_ota_v1.c
)
target_link_libraries(denzic_ota_embedded_test PRIVATE denzic_ota_embedded)
add_test(NAME denzic_ota_embedded_test COMMAND denzic_ota_embedded_test)

