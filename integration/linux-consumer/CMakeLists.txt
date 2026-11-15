cmake_minimum_required(VERSION 3.16)
project(titan_trit16_external_c_consumer C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)

get_filename_component(
    REPO_ROOT "${CMAKE_CURRENT_LIST_DIR}/../.." ABSOLUTE
)

set(
    TITAN_TRIT16_INCLUDE_DIR
    "${REPO_ROOT}/ffi/include"
    CACHE PATH "Directory containing titan_trit16.h"
)

set(
    TITAN_TRIT16_LIB_DIR
    "" CACHE PATH "Directory containing libtitan_trit16_ffi.so"
)

if(NOT EXISTS "${TITAN_TRIT16_INCLUDE_DIR}/titan_trit16.h")
    message(FATAL_ERROR "Public TRIT16 header not found")
endif()

if(NOT TITAN_TRIT16_LIB_DIR)
    message(FATAL_ERROR "Set TITAN_TRIT16_LIB_DIR")
endif()

find_library(
    TITAN_TRIT16_LIBRARY
    NAMES titan_trit16_ffi
    PATHS "${TITAN_TRIT16_LIB_DIR}"
    NO_DEFAULT_PATH
    REQUIRED
)

add_executable(trit16_external_c_consumer main.c)

target_include_directories(
    trit16_external_c_consumer
    PRIVATE "${TITAN_TRIT16_INCLUDE_DIR}"
)

target_link_libraries(
    trit16_external_c_consumer
    PRIVATE "${TITAN_TRIT16_LIBRARY}"
)

set_target_properties(
    trit16_external_c_consumer
    PROPERTIES BUILD_RPATH "${TITAN_TRIT16_LIB_DIR}"
)

enable_testing()

add_test(
    NAME trit16_external_c_consumer
    COMMAND trit16_external_c_consumer
)
