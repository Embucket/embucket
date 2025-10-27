cmake_minimum_required(VERSION 3.20)
project(velox_shim C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# Build mode: stub (default) or native linking to Velox
option(VELOX_SHIM_STUB "Build shim in stub mode (no Velox linkage)" ON)

add_library(velox_shim SHARED
    src/velox_shim.cpp
)

target_include_directories(velox_shim PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)

# Propagate stub definition to sources
if(VELOX_SHIM_STUB)
  target_compile_definitions(velox_shim PRIVATE VELOX_SHIM_STUB=1)
endif()

if(NOT VELOX_SHIM_STUB)
  # Hints: pass CMAKE_PREFIX_PATH or VELOX_BUILD_DIR when invoking cmake from build.rs
  # Example: -DCMAKE_PREFIX_PATH=$VELOX_BUILD_DIR
  find_package(Velox REQUIRED)
  # Arrow C Data Interface is header-only for ABI; link to Arrow if available via Velox
  # Link a minimal set of Velox libs; exact targets may vary by build
  # Consumers can adjust through CMAKE_PREFIX_PATH to find the same targets
  if(TARGET velox_exec)
    target_link_libraries(velox_shim PRIVATE velox_exec)
  endif()
  if(TARGET velox_substrait)
    target_link_libraries(velox_shim PRIVATE velox_substrait)
  endif()
  if(TARGET velox_functions_prestosql)
    target_link_libraries(velox_shim PRIVATE velox_functions_prestosql)
  endif()
  if(TARGET velox_aggregates)
    target_link_libraries(velox_shim PRIVATE velox_aggregates)
  endif()
endif()


