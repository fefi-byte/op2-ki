//! Kartenscan je SCAN_TAKT: Feind (Armee, Gebaeude, unterwegs, Drohung), eigene Staerke, Aussenposten, Gefechte lernen, Soll-Armee.
use super::*;
use super::{SCAN_TAKT, AUSSEN_R, GEBAEUDE};

/// C++: void karteScannen(Zustand& z)  (op2_ki.hpp:1206)
/// Ergebnis des Feindscans: Armeewert, Wert/Anzahl im Nahradius, Gebaeude (Anzahl, Summe der Koordinaten), Fahrzeugfabriken,
/// Wachtuerme, staerkstes Fahrzeug (Wert, Typ, Waffe)
pub(super) struct FeindScan { armee: i32, nah: i32, nahN: i32, geb: i32, gx: i32, gy: i32, fabriken: i32, wt: i32, bestW: i32, bestT: i32, bestWf: i32 }

/// C++: void karteScannen(Zustand& z). Je SCAN_TAKT: Feind, eigene Staerke, Aussenposten, Bedrohung, Soll-Armee.
/// Rust 2026-11-18 in Schritte aufgeteilt (Verhalten gleich).
pub fn karteScannen(welt: &mut dyn Welt, z: &mut Zustand) {
    let t = welt.Tick();
    if t < z.naechsterScan { return; }
    z.naechsterScan = t + SCAN_TAKT;
    let FeindScan { armee, nah, nahN, geb, gx, gy, fabriken, wt, bestW, bestT, bestWf } = feindScannen(welt, z);
    let eigen = eigeneZaehlen(welt, z);
    aussenpostenErfassen(welt, z, t);
    let bekannt = geb > 0 || armee > 0;
    if bekannt && z.feindGemeldet == 0 {
        let mx = if geb != 0 { gx / geb } else { z.bx };
        let my = if geb != 0 { gy / geb } else { z.by };
        melde(welt, z, "kampf", mapNone, mx, my,
              &format!("Feind entdeckt (Kartenscan): {} Gebaeude, davon {} Fahrzeugfabrik(en) und {} Wachtuerme, Armee Wert {}", geb, fabriken, wt, armee));
        z.feindGemeldet = 1;
    }
    z.feindArmee = armee; z.feindNah = nah; z.feindGeb = geb; z.feindFabriken = fabriken; z.feindWT = wt;
    // Qualitaet merken (das staerkste je gesehene bleibt -- ein Tiger, der einmal da war, kommt wieder)
    if bestW > z.feindBestWert {
        z.feindBestWert = bestW;         let fz = fahrgestellWahl(welt, z, 1 << 20);
        let n1 = name(welt, bestT);
        let n2 = name(welt, bestWf);
        let n3 = if fz != 0 { name(welt, fz) } else { "-" };
        let w3 = if fz != 0 { let wf = waffeWahl(welt, z, fz, 1 << 20); sdkWertT(welt, fz, wf) } else { 0 };
        melde(welt, z, "kampf", bestT, z.feindX, z.feindY, &format!("Feind hat jetzt {} mit {} (Wert {}), unser bestes baubares: {} (Wert {})",
              n1, n2, bestW, n3, w3));
    }
    if geb != 0 { z.feindX = gx / geb; z.feindY = gy / geb; }
    unterwegsMessen(welt, z, geb);
    z.eigeneArmee = eigen;
    if nah > 0 {
        if t - z.letzterAngriff > 3000 || z.letzterAngriff == 0 {
            z.angriffe += 1;
            melde(welt, z, "kampf", mapNone, z.bx, z.by, &format!("Angriff {}: {} feindliche Kampfeinheiten (Wert {}) im Umkreis {} der Basis, eigener Wert {}",
                  z.angriffe, nahN, nah, PAR(z.spieler).nahRadius as i32, eigen));
        }
        z.letzterAngriff = t;
        if nah > z.staerksterAngriff { z.staerksterAngriff = nah; }
    }
    if armee > z.staerksterAngriff { z.staerksterAngriff = armee; }
    verlusteMelden(welt, z, t);
    let mut bed = 0;
    if bekannt {
        bed = 15;
        if fabriken != 0 { bed += 10; } // kann Nachschub bauen
        if armee > eigen { bed += if (armee - eigen) * 3 > 40 { 40 } else { (armee - eigen) * 3 }; }
        if z.letzterAngriff != 0 && t - z.letzterAngriff < 6000 { bed += 40; }
    }
    z.bedrohung = if bed > 100 { 100 } else { bed };
    // Wachstum der Feindarmee (Zuwachs je 1000 Ticks, gleitend)
    {
        let d = armee - z.feindArmeeVorher;
        z.feindWachstum = z.feindWachstum * 0.9 + (if d > 0 { d } else { 0 }) as f64 * (1000.0 / SCAN_TAKT as f64) * 0.1;
        z.feindArmeeVorher = armee;
    }
    welleLernen(welt, z, t, nah, eigen);
    sollArmee(welt, z, armee, fabriken);
}

/// Feindliche Kampffahrzeuge (Wert, Nahradius, Sektoren, Positionen) und Gebaeude
pub(super) fn feindScannen(welt: &mut dyn Welt, z: &mut Zustand) -> FeindScan {
    let (mut armee, mut nah, mut nahN, mut geb, mut gx, mut gy, mut fabriken, mut wt) = (0i32, 0i32, 0i32, 0i32, 0i32, 0i32, 0i32, 0i32);
    let (mut bestW, mut bestT, mut bestWf) = (0i32, 0i32, 0i32);
    z.feindPosN = 0; z.feindMinD2 = 1 << 30;
    let par_ = PAR(z.spieler);
    let mut p = 0;
    while p < welt.NoPlayers() {
        if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { p += 1; continue; }
        for u in welt.PlayerVehicleEnum(p) {
            if !welt.IsLive(u) { continue; }
            let w = kampfWert(welt, u);
            if w == 0 { continue; }
            armee += w;
            if w > bestW { bestW = w; bestT = welt.GetType(u); bestWf = welt.GetWeapon(u); }
            let lo = welt.Location(u);
            let ux = edX(welt, z, lo);
            let uy = edY(welt, z, lo);
            let d2 = abstand2(welt, ux, uy, z.bx, z.by);
            if (d2 as f64) < par_.nahRadius * par_.nahRadius {
                nah += w; nahN += 1;
                let wk = ((uy - z.by) as f64).atan2((ux - z.bx) as f64);
                z.angriffSektoren |= 1 << ((((wk / (3.14159265358979 / 4.0) + 0.5).floor() as i32) + 8) % 8);
            }
            if z.feindPosN < 16 { z.feindPosX[z.feindPosN as usize] = ux; z.feindPosY[z.feindPosN as usize] = uy; z.feindPosN += 1; }
            if d2 < z.feindMinD2 { z.feindMinD2 = d2; }
        }
        for &ty in GEBAEUDE.iter() {
            for b in welt.PlayerBuildingEnum(p, ty) {
                if !welt.IsLive(b) { continue; }
                let lo = welt.Location(b);
                geb += 1; gx += edX(welt, z, lo); gy += edY(welt, z, lo);
                if ty == mapVehicleFactory { fabriken += 1; }
                if ty == mapGuardPost { wt += 1; } // verteidigen nur -- kein Angriffswert
            }
        }
        p += 1;
    }
    FeindScan { armee, nah, nahN, geb, gx, gy, fabriken, wt, bestW, bestT, bestWf }
}

/// Eigene Staerke: Kampffahrzeuge (mobil) und fertige Tuerme
pub(super) fn eigeneZaehlen(welt: &mut dyn Welt, z: &mut Zustand) -> i32 {
    let (mut mobil, mut wtWert) = (0i32, 0i32);
    {
        for u in welt.PlayerVehicleEnum(z.spieler) { if welt.IsLive(u) && kaempfer(welt, u) { mobil += kampfWert(welt, u); } }
    }
    {
        for b in welt.PlayerBuildingEnum(z.spieler, mapGuardPost) {
            if welt.IsLive(b) && welt.GetCurAction(b) != moObjWait { wtWert += kampfWert(welt, b); }
        }
    }
    z.eigeneMobil = mobil; z.eigeneWTWert = wtWert;
    mobil + wtWert
}

/// Aussenposten: eigene Minen weiter als AUSSEN_R von der Basismitte. Feind in 10 Kacheln = Angriff dort.
pub(super) fn aussenpostenErfassen(welt: &mut dyn Welt, z: &mut Zustand, t: i32) {
    {
        let mut gesehen = [0i32; 6];
        const MINEN: [i32; 2] = [mapCommonOreMine, mapRareOreMine];
        for &mt in MINEN.iter() {
            for b in welt.PlayerBuildingEnum(z.spieler, mt) {
                let lo = welt.Location(b);
                let mx = edX(welt, z, lo);
                let my = edY(welt, z, lo);
                if abstand2(welt, mx, my, z.bx, z.by) <= AUSSEN_R * AUSSEN_R { continue; }
                let mut k: i32 = -1;
                for i in 0..6 { if z.aussenId[i] == b { k = i as i32; } }
                if k < 0 { for i in 0..6 { if z.aussenId[i] == 0 && z.aussenAn[i] == 0 { k = i as i32; break; } } }
                if k < 0 { continue; }
                let ku = k as usize;
                if z.aussenId[ku] != b {
                    z.aussenId[ku] = b; z.aussenX[ku] = mx; z.aussenY[ku] = my; z.aussenAngriff[ku] = 0;
                    let nm = name(welt, mt);
                    let ab = (abstand2(welt, mx, my, z.bx, z.by) as f64).sqrt() as i32;
                    melde(welt, z, "kampf", mt, mx, my, &format!("Aussenposten: {} {} Kacheln von der Basis", nm, ab));
                }
                gesehen[ku] = 1;
                for f in 0..z.feindPosN as usize {
                    if abstand2(welt, z.feindPosX[f], z.feindPosY[f], mx, my) < 10 * 10 {
                        if z.aussenAngriff[ku] == 0 || t - z.aussenAngriff[ku] > 3000 {
                            let nm = name(welt, mt);
                            melde(welt, z, "kampf", mt, mx, my, &format!("Aussenposten {} wird angegriffen", nm));
                        }
                        z.aussenAngriff[ku] = t;
                        break;
                    }
                }
            }
        }
        for i in 0..6 { if gesehen[i] == 0 { z.aussenId[i] = 0; } }
    }
}

/// Unterwegs = feindliche Kampffahrzeuge mehr als 15 Kacheln von der Mitte ihrer Gebaeude (ohne bekannte Basis: alle);
/// nur, was naeher kommt; Drohung fuer die Kolonie; Heimwache der Feindbasis
pub(super) fn unterwegsMessen(welt: &mut dyn Welt, z: &mut Zustand, geb: i32) {
    let par_ = PAR(z.spieler);
    {
        let mut unterwegs = 0;
        let (mut neuId, mut neuD, mut neuN) = ([0i32; 128], [0i32; 128], 0usize);
        let mut zurueck = 0.0f64;
        let mut drohung = 0;
        let mut p = 0;
        while p < welt.NoPlayers() {
            if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { p += 1; continue; }
            for u in welt.PlayerVehicleEnum(p) {
                let w = kampfWert(welt, u);
                if w == 0 { continue; }
                let lo = welt.Location(u);
                let ex = edX(welt, z, lo);
                let ey = edY(welt, z, lo);
                let draussen = geb == 0 || abstand2(welt, ex, ey, z.feindX, z.feindY) > 15 * 15;
                if geb != 0 {
                    // Abwehr der Feindbasis: wer vor unserer Truppe zurueck sein kann, zaehlt voll, weiter weg abnehmend
                    let dHeim = (abstand2(welt, ex, ey, z.feindX, z.feindY) as f64).sqrt();
                    let dUns = (abstand2(welt, z.bx, z.by, z.feindX, z.feindY) as f64).sqrt();
                    let f = if dHeim <= 15.0 || dHeim <= dUns { 1.0 } else { (1.0 - (dHeim - dUns) / 30.0).clamp(0.0, 1.0) };
                    zurueck += w as f64 * f;
                }
                // Unterwegs zaehlt nur, was auf uns zukommt -- seit dem letzten Scan mehr als 2 Kacheln naeher an der
                // eigenen Basis. Stehende Wachen an Aussenposten oder Paessen sperren sonst jeden Angriff (mp4_12).
                let d = (abstand2(welt, ex, ey, z.bx, z.by) as f64).sqrt() as i32;
                let vorher = (0..z.unterwegsN as usize).find(|&i| z.unterwegsId[i] == u).map(|i| z.unterwegsD[i]);
                if neuN < 128 { neuId[neuN] = u; neuD[neuN] = d; neuN += 1; }
                let naeher = vorher.is_some_and(|v| d < v - 2);
                if draussen && naeher { unterwegs += w; }
                // Drohung fuer die Kolonie: im Nahradius und entweder im Anmarsch oder naeher an uns als an der
                // eigenen Basis. Die Heimwache vor der Feindbasis bedroht nichts, auch wenn die Basen nah beieinander liegen.
                let dFeind = if geb == 0 { i32::MAX } else { abstand2(welt, ex, ey, z.feindX, z.feindY) };
                let dEigen = abstand2(welt, ex, ey, z.bx, z.by);
                if (dEigen as f64) < par_.nahRadius * par_.nahRadius && (naeher || dEigen < dFeind) { drohung += w; }
            }
            p += 1;
        }
        z.unterwegsN = neuN as i32; z.unterwegsId = neuId; z.unterwegsD = neuD;
        z.feindArmeeZurueck = zurueck as i32;
        z.feindDrohung = drohung;
        z.feindUnterwegs = unterwegs;
    }
}

/// Verluste: weniger eigene Gebaeude als beim letzten Scan, waehrend ein Angriff laeuft
pub(super) fn verlusteMelden(welt: &mut dyn Welt, z: &mut Zustand, t: i32) {
    {
        let mut n = 0;
        for &ty in GEBAEUDE.iter() { n += welt.PlayerBuildingEnum(z.spieler, ty).len() as i32; }
        if z.eigeneGeb != 0 && n < z.eigeneGeb && z.letzterAngriff != 0 && t - z.letzterAngriff < 1000 {
            melde(welt, z, "kampf", mapNone, z.bx, z.by, &format!("Verlust beim Angriff: {} Gebaeude weniger (jetzt {})", z.eigeneGeb - n, n));
        }
        z.eigeneGeb = n;
    }
}

/// Gefechte erfassen: ein Gefecht laeuft, solange Feind-Kampfeinheiten in der Kolonie sind, endet 1000 Ticks danach;
/// danach Reserve aus dem Verlust lernen
pub(super) fn welleLernen(welt: &mut dyn Welt, z: &mut Zustand, t: i32, nah: i32, eigen: i32) {
    let par_ = PAR(z.spieler);
    if nah > 0 {
        if z.welleSpitze == 0 { z.welleEigenStart = eigen; z.welleEigenMin = eigen; }
        if nah > z.welleSpitze { z.welleSpitze = nah; }
        if eigen < z.welleEigenMin { z.welleEigenMin = eigen; }
    } else if z.welleSpitze > 0 && t - z.letzterAngriff > 1000 {
        // Lernen: Anteil des eigenen Kampfwerts, der in dieser Welle verloren ging.
        let verlust = z.welleEigenStart - z.welleEigenMin;
        let anteil = if z.welleEigenStart > 0 { verlust as f64 / z.welleEigenStart as f64 } else if verlust > 0 { 1.0 } else { 0.0 };
        let alt = z.reserve;
        // gedaempft: halb so starker Anstieg, Deckel 2,5
        if anteil > 0.05 { z.reserve *= 1.0 + anteil * par_.reserveLernrate; } else { z.reserve *= par_.reserveRueckgang; }
        if z.reserve < 1.0 { z.reserve = 1.0; }
        if z.reserve > par_.reserveMax { z.reserve = par_.reserveMax; }
        if l_kolonieLebt(welt, z) { }
        melde(welt, z, "lernen", mapNone, z.bx, z.by, &format!("Gefecht {}: eigener Wert {} -> Tiefst {} (Verlust {} %) -- Reserve {:.2} -> {:.2}",
              z.anzWellen + 1, z.welleEigenStart, z.welleEigenMin, (anteil * 100.0) as i32, alt, z.reserve));
        z.naechstesSpeichern = 0; // Gelerntes gleich sichern
        if z.anzWellen == 8 { for i in 1..8 { z.wellen[i - 1] = z.wellen[i]; } z.anzWellen = 7; }
        z.wellen[z.anzWellen as usize] = z.welleSpitze; z.anzWellen += 1;
        melde(welt, z, "kampf", mapNone, z.bx, z.by, &format!("Gefecht {} vorbei: Feind in der Kolonie bis Wert {}", z.anzWellen, z.welleSpitze));
        z.welleSpitze = 0;
    }
}

/// Soll-Armee aus dem Feind und seinem Potenzial (Fabriken x Stueck in der Reaktionszeit), mit gelernter Reserve
pub(super) fn sollArmee(welt: &mut dyn Welt, z: &mut Zustand, armee: i32, fabriken: i32) {
    let par_ = PAR(z.spieler);
    {
        let mut fz = fahrgestellWahl(welt, z, 1 << 20); if fz == 0 { fz = mapLynx; }
        let wf = waffeWahl(welt, z, fz, 1 << 20);
        let mut wert = sdkWertT(welt, fz, wf);
        if z.feindBestWert > wert { wert = z.feindBestWert; } // gesehene Feind-Qualitaet statt Annahme Gleichstand
        // Bauzeit je Fahrzeug in Ticks: mindestens 2000 Ticks je Fahrzeug und Fabrik
        let mut bau = welt.GetBuildTime(fz, z.spieler) * 4;
        if bau < 2000 { bau = 2000; }
        let stueck = par_.reaktion as i32 / (if bau > 0 { bau } else { 2400 }) + 1;
        z.feindPotenzial = armee + fabriken * stueck * wert;
    }
    // Reserve nur auf die tatsaechliche Armee und ihr Wachstum
    let echt = ((armee as f64 + z.feindWachstum * par_.reaktion / 1000.0) * z.reserve + 0.5) as i32;
    z.sollArmee = if echt > z.feindPotenzial { echt } else { z.feindPotenzial };
}
