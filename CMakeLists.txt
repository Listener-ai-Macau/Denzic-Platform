cmake_minimum_required(VERSION 3.20)
project(denzic_platform C)

enable_testing()
add_subdirectory(audio/embedded/c)
add_subdirectory(ble_windows/embedded/c)
add_subdirectory(device_control/embedded/c)
add_subdirectory(ota/embedded/c)
add_subdirectory(observability/embedded/c)
