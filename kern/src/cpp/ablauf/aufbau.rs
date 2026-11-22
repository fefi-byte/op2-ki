//! Startaufstellung: Startausruestung, Basis aufbauen, Baken sichern (Missionsstart).
use super::*;
use super::super::*;
use super::{WERTUNG_TICK, SCAN_TAKT, B_ANZAHL, GEBAEUDE, OreTypeCommon, Bar3, Bar2, VariantRandom, KI_MESSUNG};

/// C++: void startausruestung(const Zustand& z, int x, int y, int stufe = 0)  (op2_ki.hpp:6681)
// Startausruestung "von null": ConVecs mit Bausaetzen, Bergbau- und Hilfsfahrzeuge um (x, y).
// `stufe`: 0 knapp (wie bisher), 1 normal (+1 Wohnhaus), 2 komfortabel (+2 Wohnhaeuser, +1 Agridome).
pub fn startausruestung(welt: &mut dyn Welt, z: &Zustand, x: i32, y: i32, stufe: i32) {
    // (typ, fracht, dx, dy)
    const TEILE: [(i32, i32, i32, i32); 10] = [
        (mapConVec, mapCommandCenter, 0, 0),      (mapConVec, mapStructureFactory, 1, 0),
        (mapConVec, mapTokamak, 2, 0),            (mapConVec, mapCommonOreSmelter, -1, 0),
        (mapConVec, mapAgridome, 0, 1),           (mapRoboMiner, mapNone, 1, 1),
        (mapRoboSurveyor, mapNone, 2, 1),         (mapCargoTruck, mapNone, -1, 1),
        (mapCargoTruck, mapNone, -2, 1),          (mapEarthworker, mapNone, -2, 0),
    ];
    const EXTRA: [(i32, i32, i32, i32); 3] = [
        (mapConVec, mapResidence, 3, 0), (mapConVec, mapResidence, 3, 1), (mapConVec, mapAgridome, -3, 0),
    ];
    let mut convecs = 0;
    for t in TEILE.iter() {
        // Stufe 3 ("Basis steht"): die Gebaeude stehen schon (basisAufbauen) -- nur zwei leere ConVecs
        if stufe >= 3 && t.0 == mapConVec { convecs += 1; if convecs > 2 { continue; } }
        let fracht = if stufe >= 3 && t.0 == mapConVec { mapNone } else { t.1 };
        // Abseits der Basis parken: stehende Fahrzeuge blockieren Bauplaetze.
        let lo = engine(welt, z, x + 2 * t.2, y + 9 + t.3);
        welt.CreateUnit(t.0, lo, z.spieler, fracht, 0);
    }
    let mehr = if stufe <= 0 { 0 } else if stufe == 1 { 1 } else if stufe == 2 { 3 } else { 0 };
    for i in 0..mehr {
        let e = EXTRA[i];
        let lo = engine(welt, z, x + 2 * e.2, y + 9 + e.3);
        welt.CreateUnit(e.0, lo, z.spieler, e.1, 0);
    }
}

/// C++: void basisAufbauen(Zustand& z)  (op2_ki.hpp:6710)
// Stufe 3 ("Basis steht"): die Gebaeude der Startausruestung plus Wohnhaus, Standard Lab und ein zweites Agridome
// stehen schon, an den Plaetzen, die die KI selbst gewaehlt haette.
pub fn basisAufbauen(welt: &mut dyn Welt, z: &mut Zustand) {
    const GEB: [i32; 8] = [mapCommandCenter, mapStructureFactory, mapTokamak, mapCommonOreSmelter, mapAgridome,
                           mapResidence, mapStandardLab, mapAgridome];
    for typ in GEB {
        let i = if typ == mapCommonOreSmelter && PAR(z.spieler).startSchmelze > 0.0 { startSchmelzPlatz(welt, z) } else { platzWaehlen(welt, z, typ) };
        if i < 0 {
            let nm = name(welt, typ);
            melde(welt, z, "fehler", typ, z.bx, z.by, &format!("Basis steht: kein Platz fuer {}", nm));
            continue;
        }
        let iu = i as usize;
        let g = grundflaeche(welt, typ);
        let lo = engine(welt, z, z.plaetze[iu].x0 as i32 + g.w / 2, z.plaetze[iu].y0 as i32 + g.h / 2);
        welt.CreateUnit(typ, lo, z.spieler, mapNone, 0);
        z.plaetze[iu].zustand = P_STEHT; z.plaetze[iu].seit = welt.Tick();
        let nm = name(welt, typ);
        let (px, py) = (z.plaetze[iu].x0 as i32, z.plaetze[iu].y0 as i32);
        melde(welt, z, "info", typ, px, py, &format!("Basis steht: {} gesetzt (Platz {}x{})", nm, g.w, g.h));
    }
}

/// PAR startSchmelze (Nutzer 2026-11-21): die Schmelze der Startbasis so setzen, dass ihr Fahrweg zur ersten Bake moeglichst
/// genau startSchmelze Kacheln lang ist -- auf beiden Seiten gleich. Gemessen ml2_10: Platz 1 hatte die naehere Bake (25 statt
/// 36 Kacheln), aber 47 statt 32 Kacheln Erzweg und verlor 8 von 8. Fahrweg = Breitensuche ueber das Gelaende ab der Bake bis
/// zum Rand des Schmelzplatzes; nur freie, fuer ConVecs erreichbare Plaetze; bei gleichem Abstand der naehere zur Zentrale.
fn startSchmelzPlatz(welt: &mut dyn Welt, z: &mut Zustand) -> i32 {
    let f = bakeSuchen(welt, z, 0);
    if f.beste == 0 { return platzWaehlen(welt, z, mapCommonOreSmelter); }
    let bl = welt.Location(f.beste);
    let (bx, by) = (edX(welt, z, bl), edY(welt, z, bl));
    let r = super::super::routen::raster(welt, z);
    let (b, h) = (r.b, r.h);
    let mut d = vec![-1i32; (b * h) as usize];
    let mut q = std::collections::VecDeque::new();
    for dy in -1..=1 { for dx in -2..=1 {
        let (x, y) = (bx + dx, by + dy);
        if x < 0 || y < 0 || x >= b || y >= h { continue; }
        let i = (y * b + x) as usize; if d[i] < 0 { d[i] = 0; q.push_back((x, y)); }
    } }
    while let Some((x, y)) = q.pop_front() {
        let dv = d[(y * b + x) as usize];
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= b || ny >= h { continue; }
            let j = (ny * b + nx) as usize;
            if d[j] >= 0 || !r.frei[j] { continue; }
            if dx != 0 && dy != 0 && !r.frei[(y * b + nx) as usize] && !r.frei[(ny * b + x) as usize] { continue; }
            d[j] = dv + 1; q.push_back((nx, ny));
        }
    }
    let g = grundflaeche(welt, mapCommonOreSmelter);
    let mut hin = [Rechteck::default(); 160];
    let nh = hindernisse(welt, z, &mut hin, 160);
    let ziel = PAR(z.spieler).startSchmelze as i32;
    let mut best: Option<(i32, i32, i32, i32)> = None;   // (Wertung, x0, y0, Weg)
    for y0 in (by - ziel - 6)..=(by + ziel + 6) {
        for x0 in (bx - ziel - 6)..=(bx + ziel + 6) {
            let mut grund: &'static str = "";
            if !platzFrei(welt, z, x0, y0, g.w, g.h, &hin, nh, &mut grund) { continue; }
            if !super::super::hilfen::erreichbar(welt, z, x0, y0, g.w, g.h) { continue; }
            let mut weg = i32::MAX;
            for y in (y0 - 1)..=(y0 + g.h) { for x in (x0 - 1)..=(x0 + g.w) {
                if x >= x0 && x < x0 + g.w && y >= y0 && y < y0 + g.h { continue; }
                if x < 0 || y < 0 || x >= b || y >= h { continue; }
                let v = d[(y * b + x) as usize]; if v >= 0 && v < weg { weg = v; }
            } }
            if weg == i32::MAX { continue; }
            let zentrale = (x0 + g.w / 2 - z.bx).abs().max((y0 + g.h / 2 - z.by).abs());
            let wertung = (weg - ziel).abs() * 100 + zentrale;
            if best.map_or(true, |bb| wertung < bb.0) { best = Some((wertung, x0, y0, weg)); }
        }
    }
    let Some((_, x0, y0, weg)) = best else { return platzWaehlen(welt, z, mapCommonOreSmelter); };
    let i = platzEintrag(welt, z, mapCommonOreSmelter, x0, y0);
    if i >= 0 { melde(welt, z, "info", mapCommonOreSmelter, x0, y0, &format!("Startschmelze: Weg {} Kacheln zur Bake {},{} (Ziel {})", weg, bx, by, ziel)); }
    i
}

/// C++: void bakenSichern(Zustand& z)  (op2_ki.hpp:6728)
// Ohne gewoehnliche Erz-Bake in Reichweite der Basis zwei anlegen (Teil der Startausruestung).
pub fn bakenSichern(welt: &mut dyn Welt, z: &mut Zustand) {
    let f = bakeSuchen(welt, z, 0);
    let b: Id = f.beste;
    if b != 0 {
        let bl = welt.Location(b);
        let (ex, ey) = (edX(welt, z, bl), edY(welt, z, bl));
        if abstand2(welt, ex, ey, z.bx, z.by) < 40 * 40 { return; }
    }
    let mut gesetzt = 0;
    const RX: [i32; 16] = [1, 1, 0, -1, -1, -1, 0, 1, 1, 0, -1, 0, 1, -1, 1, -1];
    const RY: [i32; 16] = [0, 1, 1, 1, 0, -1, -1, -1, 0, 1, 0, -1, 1, 1, -1, -1];
    let mut r = 12;
    while r <= 30 && gesetzt < 2 {
        let mut k = 0usize;
        while k < 16 && gesetzt < 2 {
            let x = z.bx + RX[k] * r;
            let y = z.by + RY[k] * r + 2;
            if !aufKarte(welt, z, x - 2, y - 2, 3) || !aufKarte(welt, z, x + 2, y + 2, 3) { k += 1; continue; }
            let mut frei = true;
            let mut yy = y - 1;
            while yy <= y + 1 && frei {
                let mut xx = x - 2;
                while xx <= x + 2 && frei {
                    let c = zelle(welt, z, xx, yy);
                    if !zelleBebaubar(welt, c) { frei = false; }
                    xx += 1;
                }
                yy += 1;
            }
            if !frei { k += 1; continue; }
            let l = engine(welt, z, x, y);
            let rueck = welt.CreateBeacon(mapMiningBeacon, l.x, l.y, OreTypeCommon, if gesetzt == 0 { Bar2 } else { Bar3 }, VariantRandom);
            if z.anzahlBaken < 4 { z.bakenX[z.anzahlBaken as usize] = x; z.bakenY[z.anzahlBaken as usize] = y; z.anzahlBaken += 1; }
            let neu: Id = rueck;
            let typDort = if rueck > 0 && rueck < 4096 { welt.GetType(neu) } else { -1 };
            melde(welt, z, "info", mapMiningBeacon, x, y, &format!("keine Erz-Bake nahe der Basis -- Bake angelegt (Rueckgabe {}, Typ dort {}, Bake ist Typ {})",
                rueck, typDort, mapMiningBeacon));
            gesetzt += 1;
            k += 7;                                                   // zweite Bake in eine andere Richtung
            k += 1;
        }
        r += 3;
    }
}

// ---------------------------------------------------------------------------------------------------
// Langzeit-Wissen / long-term knowledge (KI-Wissen.txt im Log-Ordner, ueber alle Laeufe und Missionen)
