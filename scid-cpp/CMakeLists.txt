cmake_minimum_required(VERSION 3.15)
project(scidtopgn LANGUAGES CXX C)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# SCID source files
set(SCID_SOURCES_DIR ${CMAKE_SOURCE_DIR}/scid)

# Include directories
include_directories(${SCID_SOURCES_DIR})
include_directories(${CMAKE_SOURCE_DIR}/src)

# Source files
set(SCID_CORE_SOURCES
    ${SCID_SOURCES_DIR}/bytebuf.cpp
    ${SCID_SOURCES_DIR}/date.cpp
    ${SCID_SOURCES_DIR}/dstring.cpp
    ${SCID_SOURCES_DIR}/game.cpp
    ${SCID_SOURCES_DIR}/gfile.cpp
    ${SCID_SOURCES_DIR}/index.cpp
    ${SCID_SOURCES_DIR}/matsig.cpp
    ${SCID_SOURCES_DIR}/mfile.cpp
    ${SCID_SOURCES_DIR}/misc.cpp
    ${SCID_SOURCES_DIR}/movelist.cpp
    ${SCID_SOURCES_DIR}/myassert.cpp
    ${SCID_SOURCES_DIR}/namebase.cpp
    ${SCID_SOURCES_DIR}/pgnparse.cpp
    ${SCID_SOURCES_DIR}/position.cpp
    ${SCID_SOURCES_DIR}/stralloc.cpp
    ${SCID_SOURCES_DIR}/textbuf.cpp
)

set(SCID_HEADERS
    ${SCID_SOURCES_DIR}/bytebuf.h
    ${SCID_SOURCES_DIR}/date.h
    ${SCID_SOURCES_DIR}/dstring.h
    ${SCID_SOURCES_DIR}/game.h
    ${SCID_SOURCES_DIR}/gfile.h
    ${SCID_SOURCES_DIR}/index.h
    ${SCID_SOURCES_DIR}/matsig.h
    ${SCID_SOURCES_DIR}/mfile.h
    ${SCID_SOURCES_DIR}/misc.h
    ${SCID_SOURCES_DIR}/movelist.h
    ${SCID_SOURCES_DIR}/myassert.h
    ${SCID_SOURCES_DIR}/namebase.h
    ${SCID_SOURCES_DIR}/pgnparse.h
    ${SCID_SOURCES_DIR}/position.h
    ${SCID_SOURCES_DIR}/stralloc.h
    ${SCID_SOURCES_DIR}/textbuf.h
    ${SCID_SOURCES_DIR}/common.h
    ${SCID_SOURCES_DIR}/error.h
    ${SCID_SOURCES_DIR}/hash.h
    ${SCID_SOURCES_DIR}/stored.h
    ${SCID_SOURCES_DIR}/tokens.h
    ${SCID_SOURCES_DIR}/attacks.h
    ${SCID_SOURCES_DIR}/sqlist.h
    ${SCID_SOURCES_DIR}/sqmove.h
    ${SCID_SOURCES_DIR}/sqset.h
    ${SCID_SOURCES_DIR}/strtree.h
)

# Create object library
add_library(scid_core STATIC ${SCID_CORE_SOURCES})

# Main executable
set(SCID_MAIN_SOURCES ${CMAKE_SOURCE_DIR}/src/main.cpp)
add_executable(scidtopgn ${SCID_MAIN_SOURCES})
target_link_libraries(scidtopgn PRIVATE scid_core ${CMAKE_DL_LIBS})

# Link zlib for compression
find_package(ZLIB REQUIRED)
if(ZLIB_FOUND)
    target_include_directories(scidtopgn PRIVATE ${ZLIB_INCLUDE_DIRS})
    target_link_libraries(scidtopgn PRIVATE ${ZLIB_LIBRARIES})
endif()

# Install target
install(TARGETS scidtopgn DESTINATION bin)

# Build type
set(CMAKE_BUILD_TYPE Release)

# Compiler flags
if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(scidtopgn PRIVATE -Wall -Wextra)
    target_compile_options(scidtopgn PRIVATE -Wno-register)
    # SCID's sources use `register`, which C++17 removed and Clang rejects.
    target_compile_options(scid_core PRIVATE -Wno-register)
endif()
