//! Lagegroessen: Feindrichtung, Tuerme bei, Soll der mobilen Truppe, Rohstoff-Bilanz und Erzwahl, Kampf fehlt.
use super::*;
use super::{WERT_WT, WERT_LYNX, ix};

/// C++: void feindRichtung(const Zustand& z, int x, int y, double& ux, double& uy)  (op2_ki.hpp:1416)
pub fn feindRichtung(_welt: &mut dyn Welt, z: &Zustand, x: i32, y: i32, ux: &mut f64, uy: &mut f64) {
    let dx = if z.feindGeb != 0 { z.feindX - x } else { 1 };
    let dy = if z.feindGeb != 0 { z.feindY - y } else { 0 };
    let len = ((dx * dx + dy * dy) as f64).sqrt();
    *ux = if len > 0.0 { dx as f64 / len } else { 1.0 };
    *uy = if len > 0.0 { dy as f64 / len } else { 0.0 };
}

/// C++: int tuermeBei(const Zustand& z, int x, int y, int r = 6)  (op2_ki.hpp:1422)
pub fn tuermeBei(welt: &mut dyn Welt, z: &Zustand, x: i32, y: i32, r: i32) -> i32 {
    let mut n = 0;
    for b in welt.PlayerBuildingEnum(z.spieler, mapGuardPost) {
        let l = welt.Location(b);
        let ex = edX(welt, z, l);
        let ey = edY(welt, z, l);
        if abstand2(welt, ex, ey, x, y) <= r * r { n += 1; }
    }
    for i in 0..z.anzahlPlaetze as usize {
        let q = z.plaetze[i];
        if q.typ as i32 == mapGuardPost && (q.zustand == P_FAHRT || q.zustand == P_BAUT) && abstand2(welt, q.x0 as i32, q.y0 as i32, x, y) <= r * r { n += 1; }
    }
    n
}


/// C++: int mobilSoll(const Zustand& z, bool mitAngriff = true)  (op2_ki.hpp:1458)
pub fn mobilSoll(welt: &mut dyn Welt, z: &Zustand, mitAngriff: bool) -> i32 {
    let mut n = 0;
    for i in 0..6 { n += aussenFahrzeuge(welt, z, i) * WERT_LYNX; }
    // Ziel "Gegner vernichten": die Angriffstruppe IST das Ziel -- auch ohne Erz-Ueberschuss
    let par_ = PAR(z.spieler);
    if mitAngriff && z.feindGeb != 0 && ((z.feindFabriken != 0 && z.erzUeberschuss != 0) || (z.ziel == 3 && z.zielErreicht == 0)) {
        let mut a = (((z.feindWT * WERT_WT + z.feindArmee) as f64) * par_.ueberlegenheit) as i32;
        // Gegen eine wehrlose Basis: genug Feuerkraft, dass ihre Gebaeude in der Reaktionszeit fallen
        if z.ziel == 3 && z.zielErreicht == 0 {
            let feuer = z.feindGeb * 2 * 850 / (par_.reaktion as i32) * WERT_LYNX + WERT_LYNX;
            if feuer > a { a = feuer; }
        }
        // Eskalation: jeder Angriff staerker als der letzte
        if z.angriffStart > 0 {
            let st = (z.angriffStart as f64 * par_.steigerung + 0.5) as i32;
            if st > a { a = st; }
        }
        n += a;
    }
    n
}

/// C++: int erzRouten(const Zustand& z)  (op2_ki.hpp:1505)
pub fn erzRouten(_welt: &mut dyn Welt, z: &Zustand) -> i32 {
    let mut n = 0; for i in 0..MAX_MINEN { if z.minenId[i] != 0 { n += 1; } } n
}

/// C++: int seltenRouten(const Zustand& z)  (op2_ki.hpp:1506)
pub fn seltenRouten(_welt: &mut dyn Welt, z: &Zustand) -> i32 {
    let mut n = 0; for i in 0..2 { if z.seltenMineId[i] != 0 { n += 1; } } n
}

/// C++: Bilanz bilanz(const Zustand& z, const Lage& l)  (op2_ki.hpp:1508)
pub fn bilanz(welt: &mut dyn Welt, z: &Zustand, l: &Lage) -> Bilanz {
    let mut b = Bilanz { bedarfErz: 0, bedarfSelten: 0, erzHaengtAnSelten: 0, engpassErz: 0, engpassSelten: 0, routeErz: 0, routeSelten: 0, treiber: "-" };
    let (mut seltenZiel, mut seltenKampf) = (0i32, 0i32);
    let sp = z.spieler;
    let par_ = PAR(sp);
    // Ziel: was noch fehlt (Gebaeude, Rakete, Satellit)
    if z.ziel == 1 && z.zielErreicht == 0 {
        const ZIEL: [i32; 3] = [mapStandardLab, mapAdvancedLab, mapSpaceport];
        for &t in ZIEL.iter() {
            if l.geb[ix(t)] + geplant(welt, z, t) + l.kitsImSpiel[ix(t)] != 0 { continue; }
            b.bedarfErz += welt.GetOreCost(t, sp); seltenZiel += welt.GetRareOreCost(t, sp);
            if welt.GetRareOreCost(t, sp) > 0 { b.erzHaengtAnSelten += welt.GetOreCost(t, sp); }
        }
        const RAUM: [i32; 2] = [mapSULV, mapEDWARDSatellite];
        for &t in RAUM.iter() {
            if let Some(e) = tb::einheit(t) { b.bedarfErz += e.erz; seltenZiel += e.selten; if e.selten > 0 { b.erzHaengtAnSelten += e.erz; } }
        }
    }
    // Verteidigung: fehlende Kampfwerte als Fahrzeuge mit dem gewaehlten Fahrgestell und der gewaehlten Waffe
    {
        let mut fehlt = 0;
        let k = kampfFehlt(welt, z, l);
        let m = mobilSoll(welt, z, z.ziel == 3 && z.zielErreicht == 0) - z.eigeneMobil;
        if k > 0 { fehlt += k; }
        if m > 0 { fehlt += m; }
        let fz = fahrgestellWahl(welt, z, 1 << 20); // was die KI bauen wuerde, wenn Seltenerz da waere
        if fehlt > 0 && fz != 0 {
            let waffe = waffeWahl(welt, z, fz, 1 << 20);
            let sw = sdkWertT(welt, fz, waffe);
            let n = (fehlt + sw - 1) / sw;
            b.bedarfErz += n * (welt.GetOreCost(fz, sp) + welt.GetOreCost(waffe, sp));
            seltenKampf += n * (welt.GetRareOreCost(fz, sp) + welt.GetRareOreCost(waffe, sp));
            if welt.GetRareOreCost(fz, sp) + welt.GetRareOreCost(waffe, sp) > 0 { b.erzHaengtAnSelten += n * (welt.GetOreCost(fz, sp) + welt.GetOreCost(waffe, sp)); }
        } else if fehlt > 0 {
            b.bedarfErz += (fehlt + WERT_WT - 1) / WERT_WT * welt.GetOreCost(mapGuardPost, sp);
        }
    }
    // EMP-Rakete (Plymouth), wenn sie sich gegen die jetzige Abwehr lohnt
    if empMoeglich(welt, z) && z.empRakete == 0 && empLohnt(welt, z, None) {
        b.bedarfErz += welt.GetOreCost(mapEMPMissile, sp); seltenKampf += welt.GetRareOreCost(mapEMPMissile, sp);
        b.erzHaengtAnSelten += welt.GetOreCost(mapEMPMissile, sp);
    }
    // Der Bebauungsplan nennt, was im Horizont gebaut wird -- seine Kosten sind Erzbedarf
    for q in super::vorplan::reserviert(z) { b.bedarfErz += welt.GetOreCost(q.typ, sp); }
    b.bedarfSelten = seltenZiel + seltenKampf;
    b.treiber = if seltenZiel >= seltenKampf { if seltenZiel != 0 { "Ziel" } else { "-" } } else { "Verteidigung" };
    // Netto-Fluss nur, wenn er positiv ist
    b.engpassErz = b.bedarfErz - l.erz - ((if z.zuflussErz > 0.0 { z.zuflussErz } else { 0.0 }) * par_.horizont / 1000.0) as i32;
    b.engpassSelten = b.bedarfSelten - l.selten - ((if z.zuflussSelten > 0.0 { z.zuflussSelten } else { 0.0 }) * par_.horizont / 1000.0) as i32;
    // Ertrag einer weiteren Route
    let re = erzRouten(welt, z);
    let rs = seltenRouten(welt, z);
    let mut dMine: f64 = -1.0; // ohne Mine: 10 Kacheln
    for i in 0..MAX_MINEN {
        if z.minenId[i] == 0 { continue; }
        let m: Id = z.minenId[i];
        if !welt.IsLive(m) { continue; }
        let ml = welt.Location(m);
        let ex = edX(welt, z, ml);
        let ey = edY(welt, z, ml);
        let d = (abstand2(welt, ex, ey, z.bx, z.by) as f64).sqrt();
        if dMine < 0.0 || d < dMine { dMine = d; }
    }
    if dMine < 0.0 { dMine = 10.0; }
    let mut geschaetzt = 2.0 * 500.0 * 1000.0 / (300.0 + 60.0 * dMine);
    if geschaetzt > 1400.0 { geschaetzt = 1400.0; }
    let vorgabe = if z.erzJeRouteGelernt > geschaetzt { z.erzJeRouteGelernt } else { geschaetzt };
    let jeErz = if re > 0 && z.zuflussErz / re as f64 > vorgabe { z.zuflussErz / re as f64 } else { vorgabe };
    let jeSelten = if rs > 0 && z.zuflussSelten > 0.0 { z.zuflussSelten / rs as f64 } else { jeErz / 2.0 };
    b.routeErz = (jeErz * par_.horizont / 1000.0) as i32;
    b.routeSelten = (jeSelten * par_.horizont / 1000.0) as i32;
    b
}

/// C++: ErzWahl erzWahl(const Zustand& z, const Lage& l, const Bilanz& b)  (op2_ki.hpp:1591)
pub fn erzWahl(welt: &mut dyn Welt, z: &Zustand, l: &Lage, b: &Bilanz) -> ErzWahl {
    let mut w = ErzWahl { typ: mapNone, nutzen: 0.0 };
    let sp = z.spieler;
    let schm = l.geb[ix(mapCommonOreSmelter)] + geplant(welt, z, mapCommonOreSmelter) + l.kitsImSpiel[ix(mapCommonOreSmelter)];
    let mn = l.geb[ix(mapCommonOreMine)];
    // Arbeiter fuer eine neue Schmelze nur aus freien oder ueberzaehligen (Rang < 50)
    // Erz-gebundene Fabrik: ihre Arbeiter zaehlen fuer die Schmelze mit
    let mut erzGebunden = 0;
    if l.erz < 2000 && b.engpassErz > b.routeErz && b.routeErz > 0 {
        erzGebunden = welt.GetWorkersRequired(mapVehicleFactory, sp) * l.geb[ix(mapVehicleFactory)];
    }
    let stilllegbarS = l.arbStilllegbar[5];
    let rangS = rangNeu(welt, z, l, mapCommonOreSmelter, schm);
    let schmArbeiter = standardVolk(welt, l) || welt.GetWorkersRequired(mapCommonOreSmelter, sp)
        <= l.arbFrei + stilllegbarS + erzGebunden - beansprucht(welt, l, M_ARB as i32, rangS, mapCommonOreSmelter);
    if mn > 0 && schm < 2 * mn && schm < MAX_MINEN as i32 && b.engpassErz > 0 && schmArbeiter && kitMoeglich(welt, z, mapCommonOreSmelter) {
        let mut n = (if b.engpassErz < b.routeErz { b.engpassErz } else { b.routeErz }) as f64 / (welt.GetOreCost(mapCommonOreSmelter, sp) + 1) as f64;
        // keine zweite Schmelze vor Kindergarten, Universitaet und Eroeffnungslabor: die eine BuildingGroup baut der Reihe nach
        if schm >= 1 && super::forschung::eroeffnungBautNoch(welt, z, l) { n = 0.0; }
        if n > w.nutzen { w = ErzWahl { typ: mapCommonOreSmelter, nutzen: n }; }
    }
    let sel = l.geb[ix(mapRareOreSmelter)] + geplant(welt, z, mapRareOreSmelter) + l.kitsImSpiel[ix(mapRareOreSmelter)];
    if sel == 0 && b.engpassSelten > 0 && kitMoeglich(welt, z, mapRareOreSmelter) && seltenBakeDa(welt, z) {
        // Ohne jeden Seltenerz-Zufluss sind die Vorhaben, die Seltenerz brauchen, ganz blockiert -- ihr Erz-Wert zaehlt mit
        let mut gewinn = (if b.engpassSelten < b.routeSelten { b.engpassSelten } else { b.routeSelten }) as f64;
        if z.zuflussSelten <= 0.0 && seltenRouten(welt, z) == 0 { gewinn += b.erzHaengtAnSelten as f64; }
        let n = gewinn / (welt.GetOreCost(mapRareOreSmelter, sp) + 1) as f64;
        if n > w.nutzen { w = ErzWahl { typ: mapRareOreSmelter, nutzen: n }; }
    }
    w
}

/// C++: int kampfFehlt(const Zustand& z, const Lage& l)  (op2_ki.hpp:1632)
pub fn kampfFehlt(welt: &mut dyn Welt, z: &Zustand, l: &Lage) -> i32 {
    let unterwegs = (geplant(welt, z, mapGuardPost) + l.kitsImSpiel[ix(mapGuardPost)] + l.imBau[ix(mapGuardPost)]) * WERT_WT
        + (if istKampfFz(welt, z.fzTyp) { WERT_LYNX } else { 0 });
    z.sollArmee - z.eigeneArmee - unterwegs
}

/// C++: int hatGeb(const Zustand& z, const Lage& l, int t)  (op2_ki.hpp:1638)
pub fn hatGeb(welt: &mut dyn Welt, z: &Zustand, l: &Lage, t: i32) -> i32 {
    l.geb[ix(t)] + geplant(welt, z, t) + l.kitsImSpiel[ix(t)]
}

/// C++: bool standardVolk(const Lage& l)  (op2_ki.hpp:1639)
pub fn standardVolk(_welt: &mut dyn Welt, l: &Lage) -> bool { l.menschen >= 8000 } // KI-Kolonisten der Engine

/// C++: int kolonieGewicht(const Lage& l)  (op2_ki.hpp:1643)
pub fn kolonieGewicht(welt: &mut dyn Welt, l: &Lage) -> i32 {
    if standardVolk(welt, l) { return 20; }
    let g = 100 - (l.menschen - 50);
    if g > 100 { 100 } else if g < 20 { 20 } else { g }
}

/// C++: int nahrungsReichweite(const Lage& l)  (op2_ki.hpp:1650)
pub fn nahrungsReichweite(_welt: &mut dyn Welt, l: &Lage) -> i32 {
    if l.nahrungNetto >= 0 { return 1 << 30; }
    l.nahrung * 100 / (-l.nahrungNetto) // netto ist je 100 Ticks (eine Markierung)
}
