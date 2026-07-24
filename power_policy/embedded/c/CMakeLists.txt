add_library(denzic_power_policy_v1 STATIC
    src/denzic_power_policy_v1.c
)
target_include_directories(denzic_power_policy_v1 PUBLIC include)

add_executable(test_denzic_power_policy_v1
    tests/test_denzic_power_policy_v1.c
)
target_link_libraries(test_denzic_power_policy_v1 PRIVATE denzic_power_policy_v1)
add_test(NAME denzic_power_policy_v1 COMMAND test_denzic_power_policy_v1)
