# Bauen

Beides sind 32-Bit-Windows-DLLs (Outpost 2 ist ein 32-Bit-Spiel).

## op2ki.dll (Rust)

**Windows** (Rust mit MSVC-Toolchain):

```
rustup target add i686-pc-windows-msvc
cargo build --release -p op2ki-dll --target i686-pc-windows-msvc
```

Ergebnis: `target/i686-pc-windows-msvc/release/op2ki.dll`.

**Linux** (Cross-Build mit lld-link und dem Windows-SDK aus [xwin](https://github.com/Jake-Shadle/xwin)):

```
cargo install xwin
xwin --accept-license --arch x86 splat --output /opt/xwin
rustup target add i686-pc-windows-msvc
```

Dazu eine `.cargo/config.toml` (im Repo oder darüber):

```toml
[target.i686-pc-windows-msvc]
linker = "lld-link"
rustflags = ["-Lnative=/opt/xwin/crt/lib/x86", "-Lnative=/opt/xwin/sdk/lib/um/x86",
             "-Lnative=/opt/xwin/sdk/lib/ucrt/x86", "-Ctarget-feature=+crt-static"]
```

Dann wie unter Windows `cargo build --release -p op2ki-dll --target i686-pc-windows-msvc`.

`op2ki.dll` ist statisch gebaut und braucht nur Windows-Systembibliotheken.

## Missionen (C++)

Die Missionen bauen gegen das [OP2MissionSDK](https://github.com/OutpostUniverse/OP2MissionSDK):

```
git clone https://github.com/OutpostUniverse/OP2MissionSDK.git
```

**Windows** (Visual Studio mit x86-Werkzeugen):

```
cmake -S missionen -B build -A Win32 -DOP2SDK_DIR=<Pfad>/OP2MissionSDK
cmake --build build --config Release
```

**Linux** (clang-cl, lld-link, llvm-rc, llvm-dlltool und xwin wie oben):

```
cmake -S missionen -B build -G Ninja -DCMAKE_TOOLCHAIN_FILE=missionen/linux/clang-cl-x86.cmake \
      -DOP2SDK_DIR=<Pfad>/OP2MissionSDK
cmake --build build
```

Die Import-Bibliothek `Outpost2.lib` des SDK liegt in einem alten MSVC-Format, das lld-link nicht liest. Die
Toolchain erzeugt sie deshalb aus `missionen/linux/Outpost2.def` neu.

Ergebnis: `cDuGegenKI.dll`, `cDuGegenKI211.dll` und `cKIKRR<karte>.dll` in `build/`.

## Brücke (erzeugt)

`dll/src/extern_welt.rs` (Rust) und `missionen/gemeinsam/op2ki_bruecke.hpp` (C++) werden aus dem Trait `Welt`
(`kern/src/cpp/welt.rs`) erzeugt, nicht von Hand geändert:

```
python3 werkzeug/bruecke.py
```

Jede Methode hat eine feste Rufnummer, damit ältere Missions-DLLs mit einer neueren `op2ki.dll` laufen. Eine neue
Methode im Trait wird in `werkzeug/bruecke.py` am Ende von `CODES` eingetragen und bekommt dort ihren C++-Ausdruck.

## Aufbau

```
kern/        die KI (Rust-Bibliothek op2ki); kern/src/cpp/welt.rs ist ihre Sicht auf das Spiel
dll/         op2ki.dll: C-Schnittstelle ki2_neu / ki2_parameter / ki2_start / ki2_schritt
missionen/   Missions-DLLs: du_gegen_ki/ (mp2_08, mp2_11), zuschauen/ (KI gegen KI je Karte),
             gemeinsam/ (op2ki.hpp: KI laden und aufrufen, op2ki_bruecke.hpp: erzeugte Brücke, duell.hpp,
             aufzeichnung.hpp, Log, Absturzbericht), linux/ (Cross-Toolchain)
werkzeug/    bruecke.py: erzeugt die Brücke aus dem Trait Welt
fertig/      gebaute DLLs
```
