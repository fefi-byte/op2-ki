# Missions-DLLs fuer Outpost 2: Du gegen die KI und KI gegen KI zum Zuschauen. Jede Mission laedt zur Laufzeit
# op2ki.dll (die KI in Rust) aus ihrem Ordner; fehlt sie, spielt die eingebaute C++-KI (gemeinsam/op2_ki.hpp).
#
# Windows (MSVC, 32 Bit):
#     cmake -S missionen -B build -A Win32 -DOP2SDK_DIR=<Pfad zu OP2MissionSDK>
#     cmake --build build --config Release
# Linux (clang-cl + xwin, siehe BAUEN.md):
#     cmake -S missionen -B build -G Ninja -DCMAKE_TOOLCHAIN_FILE=missionen/linux/clang-cl-x86.cmake -DOP2SDK_DIR=<...>
#     cmake --build build
cmake_minimum_required(VERSION 3.21)
if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
  set(CMAKE_BUILD_TYPE Release CACHE STRING "Build-Typ" FORCE)
endif()
project(op2ki_missionen VERSION 1.0.0 LANGUAGES CXX RC)

if(NOT MSVC)
  message(FATAL_ERROR "Die Missionen brauchen MSVC oder clang-cl (Outpost2.lib nutzt die MSVC-Namensdekoration).")
endif()
if(NOT CMAKE_SIZEOF_VOID_P EQUAL 4)
  message(FATAL_ERROR "Nur 32 Bit (x86): mit -A Win32 bzw. der Linux-Toolchain konfigurieren.")
endif()

find_path(OP2SDK_DIR NAMES Outpost2DLL/Outpost2DLL.h
  HINTS "${CMAKE_CURRENT_SOURCE_DIR}/../OP2MissionSDK"
  DOC "OP2MissionSDK (https://github.com/OutpostUniverse/OP2MissionSDK) mit Outpost2DLL/, OP2Helper/, HFL/")
if(NOT OP2SDK_DIR)
  message(FATAL_ERROR "OP2MissionSDK nicht gefunden: -DOP2SDK_DIR=<Pfad> angeben.")
endif()
if(NOT OUTPOST2_LIB)
  set(OUTPOST2_LIB "${OP2SDK_DIR}/Outpost2DLL/Lib/Outpost2.lib")
endif()

# OP2Helper + HFL werden als Quellcode in jede Mission kompiliert.
file(GLOB OP2HELPER_SOURCES "${OP2SDK_DIR}/OP2Helper/*.cpp")
file(GLOB HFL_SOURCES "${OP2SDK_DIR}/HFL/Source/*.cpp")

function(ki_mission name quelle)
  set(RC_OUTPUT_NAME "${name}")
  configure_file(gemeinsam/version.rc.in "${CMAKE_CURRENT_BINARY_DIR}/${name}.rc" @ONLY)
  add_library(${name} SHARED ${quelle} "${CMAKE_CURRENT_BINARY_DIR}/${name}.rc" ${OP2HELPER_SOURCES} ${HFL_SOURCES})
  target_include_directories(${name} PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/gemeinsam" "${OP2SDK_DIR}")
  target_link_libraries(${name} PRIVATE "${OUTPOST2_LIB}")
  target_compile_definitions(${name} PRIVATE WIN32_LEAN_AND_MEAN NOMINMAX _CRT_SECURE_NO_WARNINGS OP2LOG_NAME="${name}")
  # /std:c++17 ohne /permissive-: die SDK-APIs binden Temporaries an non-const Referenzen (LOCATION&, MAP_RECT&).
  target_compile_options(${name} PRIVATE /std:c++17)
  if(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    # Lokale Variablen mit Null vorbelegen und SSE2-Gleitkomma: sonst haengt das Verhalten der C++-KI vom Build ab.
    target_compile_options(${name} PRIVATE /clang:-ftrivial-auto-var-init=zero /arch:SSE2)
  endif()
  set_property(TARGET ${name} PROPERTY MSVC_RUNTIME_LIBRARY "MultiThreaded")
  set_target_properties(${name} PROPERTIES OUTPUT_NAME "${name}" PREFIX "")
endfunction()

# Du gegen die KI: du Eden (Spieler 0), die KI Plymouth (Spieler 1); mit Aufzeichnung deines Spiels in OPU\logs.
ki_mission(cDuGegenKI    du_gegen_ki/mp2_08.cpp)
ki_mission(cDuGegenKI211 du_gegen_ki/mp2_11.cpp)

# KI gegen KI zum Zuschauen, eine Mission je Karte (cKIKRR<karte>.dll).
foreach(karte mp2_05 mp2_08 mp2_11 mp4_05 mp4_08 mp4_12 on4_03 on6_01 on6_03)
  string(REPLACE "_" "" kurz "${karte}")
  ki_mission(cKIKRR${kurz} zuschauen/${karte}.cpp)
endforeach()

# Zuschauen mit festem Seed (Startplaetze wie im Simulator --seed): eine Partie, die man nachspielen will
ki_mission(cKIKRRmp211s1 zuschauen/mp2_11.cpp)
target_compile_definitions(cKIKRRmp211s1 PRIVATE OP2KI_FEST_SEED=1)
