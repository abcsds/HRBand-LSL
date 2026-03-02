#----------------------------------------------------------------
# Generated CMake target import file for configuration "Release".
#----------------------------------------------------------------

# Commands may need to know the format version.
set(CMAKE_IMPORT_FILE_VERSION 1)

# Import target "LSL::lsl" for configuration "Release"
set_property(TARGET LSL::lsl APPEND PROPERTY IMPORTED_CONFIGURATIONS RELEASE)
set_target_properties(LSL::lsl PROPERTIES
  IMPORTED_LOCATION_RELEASE "${_IMPORT_PREFIX}/lib64/liblsl.so.1.13.1"
  IMPORTED_SONAME_RELEASE "liblsl.so.1.13.1"
  )

list(APPEND _cmake_import_check_targets LSL::lsl )
list(APPEND _cmake_import_check_files_for_LSL::lsl "${_IMPORT_PREFIX}/lib64/liblsl.so.1.13.1" )

# Import target "LSL::lslver" for configuration "Release"
set_property(TARGET LSL::lslver APPEND PROPERTY IMPORTED_CONFIGURATIONS RELEASE)
set_target_properties(LSL::lslver PROPERTIES
  IMPORTED_LOCATION_RELEASE "${_IMPORT_PREFIX}/bin/lslver"
  )

list(APPEND _cmake_import_check_targets LSL::lslver )
list(APPEND _cmake_import_check_files_for_LSL::lslver "${_IMPORT_PREFIX}/bin/lslver" )

# Import target "LSL::lsl-static" for configuration "Release"
set_property(TARGET LSL::lsl-static APPEND PROPERTY IMPORTED_CONFIGURATIONS RELEASE)
set_target_properties(LSL::lsl-static PROPERTIES
  IMPORTED_LINK_INTERFACE_LANGUAGES_RELEASE "CXX"
  IMPORTED_LOCATION_RELEASE "${_IMPORT_PREFIX}/lib64/liblsl-static.a"
  )

list(APPEND _cmake_import_check_targets LSL::lsl-static )
list(APPEND _cmake_import_check_files_for_LSL::lsl-static "${_IMPORT_PREFIX}/lib64/liblsl-static.a" )

# Commands beyond this point should not need to know the version.
set(CMAKE_IMPORT_FILE_VERSION)
