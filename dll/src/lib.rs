//! C-Schnittstelle der KI fuer die Missions-DLL (LoadLibrary "op2ki.dll", cdecl). Die Mission gibt die OP2-API als
//! Funktionszeiger (extern_welt::Api), setzt die Stellschrauben je Spieler, ruft einmal ki2_start und dann jeden Tick
//! ki2_schritt.
//! C interface of the AI for the mission DLL.
mod extern_welt;
use op2ki::cpp::{self, zustand::Zustand};

/// Version der C-Schnittstelle (die Mission schreibt sie ins Log).
#[no_mangle]
pub extern "C" fn ki_abi() -> i32 { op2ki::ABI }

/// Eine KI-Instanz: die OP2-API der Mission und je Spieler ein Zustand.
pub struct Ki2 { welt: extern_welt::ExternWelt, z: Vec<(i32, Box<Zustand>)> }

#[no_mangle]
pub extern "C" fn ki2_neu(api: extern_welt::Api) -> *mut Ki2 {
    cpp::globale::zuruecksetzen();
    Box::into_raw(Box::new(Ki2 { welt: extern_welt::ExternWelt::neu(api), z: Vec::new() }))
}

/// Eingebaute Stellschrauben der Mission (ki::parameterText[spieler], "name=wert" je Zeile).
/// # Safety
/// `text` zeigt auf `n` Bytes.
#[no_mangle]
pub unsafe extern "C" fn ki2_parameter(_h: *mut Ki2, spieler: i32, text: *const u8, n: i32) {
    if text.is_null() || n < 0 || !(0..8).contains(&spieler) { return; }
    let t = String::from_utf8_lossy(std::slice::from_raw_parts(text, n as usize)).into_owned();
    cpp::globale::parameterText.with(|p| p.borrow_mut()[spieler as usize] = Some(t));
}

/// ki::start fuer einen Spieler.
/// # Safety
/// `h` stammt aus `ki2_neu`.
#[no_mangle]
pub unsafe extern "C" fn ki2_start(h: *mut Ki2, spieler: i32, ux: i32, uy: i32, breite: i32, hoehe: i32, x: i32, y: i32,
    ausruestung: i32, ziel: i32, start_stufe: i32, ziel_wert: i32) {
    let Some(k) = h.as_mut() else { return };
    let mut z = Zustand::null();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        cpp::start(&mut k.welt, &mut z, spieler, cpp::welt::Loc::new(ux, uy), breite, hoehe, x, y, ausruestung != 0, ziel, start_stufe, ziel_wert);
    }));
    if r.is_err() { op2ki::cpp::welt::Welt::logZeile(&mut k.welt, &format!("Rust-KI 2: Absturz in start (Spieler {spieler})")); }
    k.z.retain(|(p, _)| *p != spieler);
    k.z.push((spieler, z));
}

/// ki::schritt fuer einen Spieler (jeden Tick aus AIProc).
/// # Safety
/// `h` stammt aus `ki2_neu`.
#[no_mangle]
pub unsafe extern "C" fn ki2_schritt(h: *mut Ki2, spieler: i32) {
    let Some(k) = h.as_mut() else { return };
    let Ki2 { welt, z } = k;
    let Some((_, zs)) = z.iter_mut().find(|(p, _)| *p == spieler) else { return };
    cpp::globale::g_kraftSpieler.with(|g| g.set(spieler));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cpp::schritt(welt, zs)));
    if let Err(e) = r {
        let m = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
        op2ki::cpp::welt::Welt::logZeile(welt, &format!("Rust-KI 2: Absturz in schritt (Spieler {spieler}): {m}"));
    }
}

/// Befehl der Mission waehrend des Spiels (op2ki::befehl, Text "name=wert" je Zeile): Drehbuch-Phase, Angriff,
/// Verteidigung, Rolle, Stellschrauben -- siehe kern/src/cpp/befehl.rs.
/// # Safety
/// `h` stammt aus `ki2_neu`; `text` hat `n` Bytes.
#[no_mangle]
pub unsafe extern "C" fn ki2_befehl(h: *mut Ki2, spieler: i32, text: *const u8, n: i32) {
    let Some(k) = h.as_mut() else { return };
    if text.is_null() || n < 0 { return; }
    let t = String::from_utf8_lossy(std::slice::from_raw_parts(text, n as usize)).into_owned();
    let Ki2 { welt, z } = k;
    let Some((_, zs)) = z.iter_mut().find(|(p, _)| *p == spieler) else { return };
    cpp::globale::g_kraftSpieler.with(|g| g.set(spieler));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cpp::befehl::befehl(welt, zs, &t)));
    if r.is_err() { op2ki::cpp::welt::Welt::logZeile(welt, &format!("Rust-KI 2: Absturz in befehl (Spieler {spieler})")); }
}

/// Rueckmeldung an die Mission (op2ki::status): Phase fertig, Angriffsstand, Angriff auf die Basis ... (befehl::status)
/// # Safety
/// `h` stammt aus `ki2_neu`.
#[no_mangle]
pub unsafe extern "C" fn ki2_status(h: *mut Ki2, spieler: i32, code: i32) -> i32 {
    let Some(k) = h.as_mut() else { return 0 };
    let Ki2 { welt, z } = k;
    let Some((_, zs)) = z.iter_mut().find(|(p, _)| *p == spieler) else { return 0 };
    cpp::globale::g_kraftSpieler.with(|g| g.set(spieler));
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cpp::befehl::status(welt, zs, code))).unwrap_or(0)
}
