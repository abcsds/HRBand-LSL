file(REMOVE_RECURSE
  "liblsl-static.a"
  "liblsl-static.pdb"
)

# Per-language clean rules from dependency scanning.
foreach(lang CXX)
  include(CMakeFiles/lsl-static.dir/cmake_clean_${lang}.cmake OPTIONAL)
endforeach()
