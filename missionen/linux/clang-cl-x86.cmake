# Cross-Toolchain: clang-cl (MSVC-ABI) -> 32-Bit-Windows-DLL unter Linux. CRT und Windows-SDK aus "xwin splat"
# (Standard: /opt/xwin, sonst -DXWIN_DIR=<Pfad>). Die Import-Bibliotheken des SDK (altes MSVC-Format) bricht lld-link
# ("section larger than 4 GiB"), deshalb werden sie aus den .def-Dateien hier neu erzeugt (llvm-dlltool).
set(CMAKE_SYSTEM_NAME Windows)
set(CMAKE_SYSTEM_VERSION 10.0)
set(CMAKE_SYSTEM_PROCESSOR X86)
if(NOT XWIN_DIR)
  set(XWIN_DIR "/opt/xwin")
endif()

find_program(CLANG_CL NAMES clang-cl clang-cl-19 REQUIRED)
find_program(LLD_LINK NAMES lld-link REQUIRED)
find_program(LLVM_RC NAMES llvm-rc llvm-rc-19 REQUIRED)
find_program(LLVM_LIB NAMES llvm-lib llvm-lib-19 REQUIRED)
find_program(LLVM_DLLTOOL NAMES llvm-dlltool llvm-dlltool-19 REQUIRED)
set(CMAKE_C_COMPILER   ${CLANG_CL})
set(CMAKE_CXX_COMPILER ${CLANG_CL})
set(CMAKE_LINKER       ${LLD_LINK})
set(CMAKE_RC_COMPILER  ${LLVM_RC})
set(CMAKE_AR           ${LLVM_LIB})
set(CMAKE_C_COMPILER_TARGET   i686-pc-windows-msvc)
set(CMAKE_CXX_COMPILER_TARGET i686-pc-windows-msvc)

set(_inc "/imsvc ${XWIN_DIR}/crt/include /imsvc ${XWIN_DIR}/sdk/include/ucrt /imsvc ${XWIN_DIR}/sdk/include/um /imsvc ${XWIN_DIR}/sdk/include/shared")
set(CMAKE_C_FLAGS_INIT   "--target=i686-pc-windows-msvc -fms-compatibility-version=19.40 ${_inc}")
set(CMAKE_CXX_FLAGS_INIT "--target=i686-pc-windows-msvc -fms-compatibility-version=19.40 ${_inc} /EHsc")
set(CMAKE_RC_FLAGS_INIT  "/I ${XWIN_DIR}/sdk/include/um /I ${XWIN_DIR}/sdk/include/shared /I ${XWIN_DIR}/crt/include /I ${XWIN_DIR}/sdk/include/ucrt")
# Outpost2.lib ist nicht SAFESEH-faehig
set(_lib "/libpath:${XWIN_DIR}/crt/lib/x86 /libpath:${XWIN_DIR}/sdk/lib/um/x86 /libpath:${XWIN_DIR}/sdk/lib/ucrt/x86 /SAFESEH:NO")
set(CMAKE_SHARED_LINKER_FLAGS_INIT "${_lib}")
set(CMAKE_EXE_LINKER_FLAGS_INIT    "${_lib}")
set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
# xwin liefert keine Debug-CRT -> Testprogramme im Release-Modus
set(CMAKE_TRY_COMPILE_CONFIGURATION Release)

set(_libdir "${CMAKE_BINARY_DIR}/importlib")
file(MAKE_DIRECTORY "${_libdir}")
execute_process(COMMAND ${LLVM_DLLTOOL} -m i386 -d "${CMAKE_CURRENT_LIST_DIR}/Outpost2.def" -l "${_libdir}/Outpost2.lib" COMMAND_ERROR_IS_FATAL ANY)
set(OUTPOST2_LIB "${_libdir}/Outpost2.lib")
