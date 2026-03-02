# Install script for directory: /home/beto/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/lsl-sys-0.1.1/liblsl

# Set the install prefix
if(NOT DEFINED CMAKE_INSTALL_PREFIX)
  set(CMAKE_INSTALL_PREFIX "/home/beto/code/HRBand-LSL/target/release/build/lsl-sys-b9d8c70acdf76a9f/out")
endif()
string(REGEX REPLACE "/$" "" CMAKE_INSTALL_PREFIX "${CMAKE_INSTALL_PREFIX}")

# Set the install configuration name.
if(NOT DEFINED CMAKE_INSTALL_CONFIG_NAME)
  if(BUILD_TYPE)
    string(REGEX REPLACE "^[^A-Za-z0-9_]+" ""
           CMAKE_INSTALL_CONFIG_NAME "${BUILD_TYPE}")
  else()
    set(CMAKE_INSTALL_CONFIG_NAME "Release")
  endif()
  message(STATUS "Install configuration: \"${CMAKE_INSTALL_CONFIG_NAME}\"")
endif()

# Set the component getting installed.
if(NOT CMAKE_INSTALL_COMPONENT)
  if(COMPONENT)
    message(STATUS "Install component: \"${COMPONENT}\"")
    set(CMAKE_INSTALL_COMPONENT "${COMPONENT}")
  else()
    set(CMAKE_INSTALL_COMPONENT)
  endif()
endif()

# Install shared libraries without execute permission?
if(NOT DEFINED CMAKE_INSTALL_SO_NO_EXE)
  set(CMAKE_INSTALL_SO_NO_EXE "0")
endif()

# Is this installation the result of a crosscompile?
if(NOT DEFINED CMAKE_CROSSCOMPILING)
  set(CMAKE_CROSSCOMPILING "FALSE")
endif()

# Set path to fallback-tool for dependency-resolution.
if(NOT DEFINED CMAKE_OBJDUMP)
  set(CMAKE_OBJDUMP "/nix/store/9qqhx0a9270lq4n64a8k46vjlnvd7301-gcc-wrapper-13.4.0/bin/objdump")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "liblsl" OR NOT CMAKE_INSTALL_COMPONENT)
  if(EXISTS "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/lib64/liblsl.so.1.13.1" AND
     NOT IS_SYMLINK "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/lib64/liblsl.so.1.13.1")
    file(RPATH_CHECK
         FILE "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/lib64/liblsl.so.1.13.1"
         RPATH "")
  endif()
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib64" TYPE SHARED_LIBRARY FILES "/home/beto/code/HRBand-LSL/target/release/build/lsl-sys-b9d8c70acdf76a9f/out/build/liblsl.so.1.13.1")
  if(EXISTS "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/lib64/liblsl.so.1.13.1" AND
     NOT IS_SYMLINK "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/lib64/liblsl.so.1.13.1")
    if(CMAKE_INSTALL_DO_STRIP)
      execute_process(COMMAND "/nix/store/9qqhx0a9270lq4n64a8k46vjlnvd7301-gcc-wrapper-13.4.0/bin/strip" "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/lib64/liblsl.so.1.13.1")
    endif()
  endif()
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "liblsl" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib64" TYPE SHARED_LIBRARY FILES "/home/beto/code/HRBand-LSL/target/release/build/lsl-sys-b9d8c70acdf76a9f/out/build/liblsl.so")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "liblsl" OR NOT CMAKE_INSTALL_COMPONENT)
  if(EXISTS "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/lslver" AND
     NOT IS_SYMLINK "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/lslver")
    file(RPATH_CHECK
         FILE "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/lslver"
         RPATH "")
  endif()
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE EXECUTABLE FILES "/home/beto/code/HRBand-LSL/target/release/build/lsl-sys-b9d8c70acdf76a9f/out/build/lslver")
  if(EXISTS "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/lslver" AND
     NOT IS_SYMLINK "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/lslver")
    file(RPATH_CHANGE
         FILE "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/lslver"
         OLD_RPATH "/home/beto/code/HRBand-LSL/target/release/build/lsl-sys-b9d8c70acdf76a9f/out/build:"
         NEW_RPATH "")
    if(CMAKE_INSTALL_DO_STRIP)
      execute_process(COMMAND "/nix/store/9qqhx0a9270lq4n64a8k46vjlnvd7301-gcc-wrapper-13.4.0/bin/strip" "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/lslver")
    endif()
  endif()
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "liblsl" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib64" TYPE STATIC_LIBRARY FILES "/home/beto/code/HRBand-LSL/target/release/build/lsl-sys-b9d8c70acdf76a9f/out/build/liblsl-static.a")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "liblsl" OR NOT CMAKE_INSTALL_COMPONENT)
  if(EXISTS "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/share/LSL/LSLTargets.cmake")
    file(DIFFERENT _cmake_export_file_changed FILES
         "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/share/LSL/LSLTargets.cmake"
         "/home/beto/code/HRBand-LSL/target/release/build/lsl-sys-b9d8c70acdf76a9f/out/build/CMakeFiles/Export/c260596966d4a118b3f2428645d3d835/LSLTargets.cmake")
    if(_cmake_export_file_changed)
      file(GLOB _cmake_old_config_files "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/share/LSL/LSLTargets-*.cmake")
      if(_cmake_old_config_files)
        string(REPLACE ";" ", " _cmake_old_config_files_text "${_cmake_old_config_files}")
        message(STATUS "Old export file \"$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/share/LSL/LSLTargets.cmake\" will be replaced.  Removing files [${_cmake_old_config_files_text}].")
        unset(_cmake_old_config_files_text)
        file(REMOVE ${_cmake_old_config_files})
      endif()
      unset(_cmake_old_config_files)
    endif()
    unset(_cmake_export_file_changed)
  endif()
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/share/LSL" TYPE FILE FILES "/home/beto/code/HRBand-LSL/target/release/build/lsl-sys-b9d8c70acdf76a9f/out/build/CMakeFiles/Export/c260596966d4a118b3f2428645d3d835/LSLTargets.cmake")
  if(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Rr][Ee][Ll][Ee][Aa][Ss][Ee])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/share/LSL" TYPE FILE FILES "/home/beto/code/HRBand-LSL/target/release/build/lsl-sys-b9d8c70acdf76a9f/out/build/CMakeFiles/Export/c260596966d4a118b3f2428645d3d835/LSLTargets-release.cmake")
  endif()
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "liblsl" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/include" TYPE DIRECTORY FILES "/home/beto/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/lsl-sys-0.1.1/liblsl/include/")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "liblsl" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/share/LSL" TYPE FILE FILES
    "/home/beto/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/lsl-sys-0.1.1/liblsl/cmake/LSLCMake.cmake"
    "/home/beto/code/HRBand-LSL/target/release/build/lsl-sys-b9d8c70acdf76a9f/out/build/LSLConfig.cmake"
    "/home/beto/code/HRBand-LSL/target/release/build/lsl-sys-b9d8c70acdf76a9f/out/build/LSLConfigVersion.cmake"
    )
endif()

string(REPLACE ";" "\n" CMAKE_INSTALL_MANIFEST_CONTENT
       "${CMAKE_INSTALL_MANIFEST_FILES}")
if(CMAKE_INSTALL_LOCAL_ONLY)
  file(WRITE "/home/beto/code/HRBand-LSL/target/release/build/lsl-sys-b9d8c70acdf76a9f/out/build/install_local_manifest.txt"
     "${CMAKE_INSTALL_MANIFEST_CONTENT}")
endif()
if(CMAKE_INSTALL_COMPONENT)
  if(CMAKE_INSTALL_COMPONENT MATCHES "^[a-zA-Z0-9_.+-]+$")
    set(CMAKE_INSTALL_MANIFEST "install_manifest_${CMAKE_INSTALL_COMPONENT}.txt")
  else()
    string(MD5 CMAKE_INST_COMP_HASH "${CMAKE_INSTALL_COMPONENT}")
    set(CMAKE_INSTALL_MANIFEST "install_manifest_${CMAKE_INST_COMP_HASH}.txt")
    unset(CMAKE_INST_COMP_HASH)
  endif()
else()
  set(CMAKE_INSTALL_MANIFEST "install_manifest.txt")
endif()

if(NOT CMAKE_INSTALL_LOCAL_ONLY)
  file(WRITE "/home/beto/code/HRBand-LSL/target/release/build/lsl-sys-b9d8c70acdf76a9f/out/build/${CMAKE_INSTALL_MANIFEST}"
     "${CMAKE_INSTALL_MANIFEST_CONTENT}")
endif()
