//! Angriffsziel waehlen und setzen (Zielstufen), Gruppenwert.
use super::*;
use super::super::*;
use super::{GRUPPE_MAX, SCAN_TAKT, WERT_WT, WERT_LYNX, EMP_RADIUS, EMP_FLUG, EMP_AUFMARSCH, GEBAEUDE};

/// C++: int gruppenWert(ScGroup& g, int& n)  (op2_ki.hpp:4219)
pub fn gruppenWert(welt: &mut dyn Welt, g: Gruppe, n: &mut i32) -> i32 {
    // Jede Einheit nur einmal (eine Einheit stand mehrfach in der Gruppe)
    let mut w = 0; *n = 0;
    let mut ids = [0i32; 64]; let mut ni = 0usize;
    for v in welt.GroupEnumerator(g) {
        if !welt.IsLive(v) || !kaempfer(welt, v) { continue; }
        let mut schon = false;
        for i in 0..ni { if ids[i] == v { schon = true; break; } }
        if schon { continue; }
        if ni < 64 { ids[ni] = v; ni += 1; }
        w += kampfWert(welt, v); *n += 1;
    }
    w
}

/// C++: bool inGruppeG(ScGroup& g, int id)  (op2_ki.hpp:4235)
pub fn inGruppeG(welt: &mut dyn Welt, g: Gruppe, id: i32) -> bool {
    for v in welt.GroupEnumerator(g) { if v == id { return true; } }
    false
}

/// C++: UnitEx angriffZielWaehlen(const Zustand& z, int vonX, int vonY, int& stufe)  (op2_ki.hpp:4248)
/// Zielreihenfolge: Zentrale, Strukturfabrik, Kraftwerke, Fahrzeugfabrik, Rest -- je Stufe das naechste.
pub fn angriffZielWaehlen(welt: &mut dyn Welt, z: &Zustand, vonX: i32, vonY: i32, stufe: &mut i32) -> Id {
    let mut beste: Id = 0; let mut besterD = 1 << 30; *stufe = -1;
    let mut s2 = 0usize;
    while s2 < 5 && beste == 0 {
        let mut p = 0;
        while p < welt.NoPlayers() {
            if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { p += 1; continue; }
            if s2 < 4 {
                for k in 0..4 {
                    if STUFE[s2][k] == mapNone { continue; }
                    for b in welt.PlayerBuildingEnum(p, STUFE[s2][k]) {
                        let lo = welt.Location(b);
                        let (ex, ey) = (edX(welt, z, lo), edY(welt, z, lo));
                        let d = abstand2(welt, ex, ey, vonX, vonY);
                        if d < besterD { besterD = d; beste = b; *stufe = s2 as i32; }
                    }
                }
            } else {
                for &ty in GEBAEUDE.iter() {
                    for b in welt.PlayerBuildingEnum(p, ty) {
                        let lo = welt.Location(b);
                        let (ex, ey) = (edX(welt, z, lo), edY(welt, z, lo));
                        let d = abstand2(welt, ex, ey, vonX, vonY);
                        if d < besterD { besterD = d; beste = b; *stufe = 4; }
                    }
                }
            }
            p += 1;
        }
        s2 += 1;
    }
    beste
}

/// C++: void angriffZielSetzen(Zustand& z, int vonX, int vonY)  (op2_ki.hpp:4286)
pub fn angriffZielSetzen(welt: &mut dyn Welt, z: &mut Zustand, vonX: i32, vonY: i32) {
    let mut stufe = -1;
    let mut ziel: Id = 0;
    // Ziel- und Zermuerbungsangriff: erst das gewaehlte Gebaeude, danach weiter nach Funktion
    { let w: Id = z.angriffWunsch; if z.angriffWunsch != 0 && welt.IsLive(w) { ziel = w; stufe = 4; } else { z.angriffWunsch = 0; } }
    if ziel == 0 { ziel = angriffZielWaehlen(welt, z, vonX, vonY, &mut stufe); }
    if ziel == 0 { return; }
    if ziel == z.angriffZiel {
        let alt: Id = z.angriffZiel;
        if welt.IsLive(alt) {
            // Ziel steht noch: untaetige Fahrzeuge bekommen den Befehl neu
            let ge = AngriffEnum(welt, z);
            // DoAttack bewegt nicht in Reichweite -- ausser Reichweite erst heranfahren.
            let zl = welt.Location(alt);
            // Kampfmodell: wer eine Bedrohung in Reichweite hat, schiesst auf sie, nicht auf das Gebaeude.
            let mut bedr = [0 as Id; 128]; let mut nb = 0usize;
            let mut p = 0;
            while p < welt.NoPlayers() {
                if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { p += 1; continue; }
                for u in welt.PlayerVehicleEnum(p) { if nb >= 128 { break; } if welt.IsLive(u) && kampfWert(welt, u) > 0 { bedr[nb] = u; nb += 1; } }
                for u in welt.PlayerBuildingEnum(p, mapGuardPost) { if nb >= 128 { break; } if welt.IsLive(u) { bedr[nb] = u; nb += 1; } }
                p += 1;
            }
            for v in ge {
                if !welt.IsLive(v) || !welt.IsVehicle(v) { continue; }
                let akt = welt.GetCurAction(v);
                if nb != 0 && (akt == moDone || akt == moMove) {
                    let wi0 = welt.GetWeapon(v);
                    let rw0 = if welt.GetWeaponRange(wi0, z.spieler) > 0 { welt.GetWeaponRange(wi0, z.spieler) } else { 4 };
                    let vl0 = welt.Location(v);
                    let mut best = (rw0 + 1) * (rw0 + 1) + 1; let mut bi: i32 = -1;
                    for i in 0..nb {
                        if !welt.IsLive(bedr[i]) { continue; }
                        let bl = welt.Location(bedr[i]); let d = abstand2(welt, vl0.x, vl0.y, bl.x, bl.y);
                        if d < best { best = d; bi = i as i32; }
                    }
                    if bi >= 0 { if akt == moMove || !welt.IsBusy(v) { welt.DoAttack(v, bedr[bi as usize]); } continue; }
                }
                // In Reichweite: angreifen, auch wenn sie noch "faehrt".
                if akt != moDone && akt != moMove { continue; }
                let wi = welt.GetWeapon(v);
                let rw = if welt.GetWeaponRange(wi, z.spieler) > 0 { welt.GetWeaponRange(wi, z.spieler) } else { 4 };
                let vl = welt.Location(v);
                let d2 = abstand2(welt, vl.x, vl.y, zl.x, zl.y);
                if d2 <= (rw + 1) * (rw + 1) { if akt == moMove || !welt.IsBusy(v) { welt.DoAttack(v, alt); } continue; }
                if akt != moDone || welt.IsBusy(v) { continue; }
                // Heranfahren bis knapp in Reichweite, nicht auf das Gebaeude selbst
                let d = (d2 as f64).sqrt(); let f = (d - (rw - 1) as f64) / d;
                let hin = Loc::new(vl.x + (((zl.x - vl.x) as f64) * f + 0.5) as i32, vl.y + (((zl.y - vl.y) as f64) * f + 0.5) as i32);
                welt.DoMove(v, hin);
            }
            return;
        }
    }
    let lo = welt.Location(ziel);
    let x = edX(welt, z, lo); let y = edY(welt, z, lo);
    // Wartezone (SetRect) neben dem Ziel, auf unserer Seite; bewachte Zone ueber der Feindbasis (DoGuardRect).
    {
        let (mut ux, mut uy) = (0.0f64, 0.0f64);
        feindRichtung(welt, z, z.bx, z.by, &mut ux, &mut uy);                       // Richtung Feind -> Wartezone davor (unsere Seite)
        let wx = (x as f64 - ux * 3.0 + 0.5) as i32; let wy = (y as f64 - uy * 3.0 + 0.5) as i32;
        let r = Rect::new(engine(welt, z, wx - 1, wy - 1), engine(welt, z, wx + 1, wy + 1));
        let feind = Rect::new(engine(welt, z, z.feindX - 12, z.feindY - 12), engine(welt, z, z.feindX + 12, z.feindY + 12));
        wachen(welt, z.angriff, r, &[feind], 1);
        if z.angriff2An != 0 { wachen(welt, z.angriff2, r, &[feind], 1); }
    }
    z.angriffZiel = ziel;
    let zt = welt.GetType(ziel);
    let nm = name(welt, zt);
    melde(welt, z, "kampf", zt, x, y, &format!("Angriffsziel: {} {} bei {},{}", ART[if stufe < 0 { 4 } else { stufe as usize }], nm, x, y));
}
