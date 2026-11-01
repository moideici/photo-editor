# Décodeur AV1 portable, compilé avec MSVC sans dépendance NASM.
set(AOM_TARGET_CPU generic CACHE STRING "Portable AV1 decoder" FORCE)
if(WIN32 AND NOT PERL_EXECUTABLE)
  find_program(PRISME_PERL perl PATHS "C:/Program Files/Git/usr/bin")
  if(PRISME_PERL)
    set(PERL_EXECUTABLE "${PRISME_PERL}" CACHE FILEPATH "Perl for libaom" FORCE)
  endif()
endif()
