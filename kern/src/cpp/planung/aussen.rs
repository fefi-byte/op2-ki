//! Aussenposten: gefaehrdet?, Wachen dort, Turm fehlt.
use super::*;

/// C++: bool aussenGefaehrdet(const Zustand& z, int i)  (op2_ki.hpp:1439)
pub fn aussenGefaehrdet(welt: &mut dyn Welt, z: &Zustand, i: i32) -> bool {
    let i = i as usize;
    if z.aussenId[i] == 0 || z.bedrohung == 0 { return false; }
    if z.aussenAngriff[i] != 0 && welt.Tick() - z.aussenAngriff[i] < 10000 { return true; }
    z.feindGeb != 0 && abstand2(welt, z.aussenX[i], z.aussenY[i], z.feindX, z.feindY) < abstand2(welt, z.bx, z.by, z.feindX, z.feindY)
}

/// C++: int aussenFahrzeuge(const Zustand& z, int i)  (op2_ki.hpp:1444)
pub fn aussenFahrzeuge(welt: &mut dyn Welt, z: &Zustand, i: i32) -> i32 {
    if !aussenGefaehrdet(welt, z, i) { return 0; }
    let iu = i as usize;
    if z.aussenAngriff[iu] != 0 && welt.Tick() - z.aussenAngriff[iu] < 10000 { return 2; }
    if z.bedrohung >= 50 { 1 } else { 0 }
}

/// C++: int aussenTurmFehlt(const Zustand& z)  (op2_ki.hpp:1449)
pub fn aussenTurmFehlt(welt: &mut dyn Welt, z: &Zustand) -> i32 {
    for i in 0..6 {
        if aussenGefaehrdet(welt, z, i) && tuermeBei(welt, z, z.aussenX[i as usize], z.aussenY[i as usize], 6) == 0 { return i; }
    }
    -1
}
