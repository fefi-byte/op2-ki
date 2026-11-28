//! Bauplaetze: Kolonie-, Aussen-, Schmelzen- und Turmplaetze, Mauern, Bau aufzeichnen, freier Warteplatz.
use super::*;
use super::super::*;
use super::{GEBAEUDE, TECH_RAUMFAHRT, B_ZIEL};

// ---------------------------------------------------------------------------------------------------
// Bauen ueber eine BuildingGroup der Spiel-KI / building via the game's own BuildingGroup
// Die KI entscheidet nur noch WAS (bedarf) und WO (platzWaehlen) gebaut wird. Den Rest macht die BuildingGroup der Engine.

/// C++: bool stehtDa(const Zustand& z, const Platz& p)  (op2_ki.hpp:2581)
pub fn stehtDa(welt: &mut dyn Welt, z: &Zustand, p: &Platz) -> bool {
    let g = grundflaeche(welt, p.typ as i32);
    let a = engine(welt, z, p.x0 as i32, p.y0 as i32);
    let b = engine(welt, z, p.x0 as i32 + g.w - 1, p.y0 as i32 + g.h - 1);
    let r = Rect::new(a, b);
    for u in welt.InRectEnumerator(r) {
        if welt.IsBuilding(u) && welt.GetType(u) == p.typ as i32 && welt.OwnerID(u) == z.spieler { return true; }
    }
    false
}

// Fertig und ruhig? Ein TakeUnit auf ein Gebaeude im Bau (moDevelop) oder kurz davor macht es kaputt.
/// C++: bool bereit(UnitEx u)  (op2_ki.hpp:2592)
pub fn bereit(welt: &mut dyn Welt, u: Id) -> bool {
    u != 0 && welt.IsLive(u) && welt.GetCurAction(u) == moDone && !welt.IsBusy(u)
}

/// C++: bool inGruppe(BuildingGroup& g, int id)  (op2_ki.hpp:2594)
pub fn inGruppe(welt: &mut dyn Welt, g: Gruppe, id: i32) -> bool {
    for v in welt.GroupEnumerator(g) { if v == id { return true; } }
    false
}

// Wachtuerme verteidigen die Kolonie als Ganzes. Davor muss das Schussfeld frei sein (Laser schiesst nicht ueber Mauern/Gebaeude).
/// C++: bool schussfeldFrei(const Rechteck* hin, int nh, int x, int y, double ux, double uy)  (op2_ki.hpp:2611)
pub fn schussfeldFrei(welt: &mut dyn Welt, z: &Zustand, hin: &[Rechteck], nh: i32, x: i32, y: i32, ux: f64, uy: f64) -> bool {
    // Rust (kampfModell): Laenge = Reichweite der Turmwaffe des Spielers (mit Erforschtem); Waffen, die ueber Mauern und
    // Gebaeude schiessen (Saeure, Thor, EMP, Schaum, ESG, RPG), brauchen keine freie Bahn.
    let mut laenge = SCHUSSFELD;
    if kampfModellNeu() {
        let w = eigeneWaffe(welt, z);
        if matches!(w, mapAcidCloud | mapThorsHammer | mapEMP | mapStickyfoam | mapESG | mapRPG) { return true; }
        let r = welt.GetWeaponRange(w, z.spieler);
        if r > 0 { laenge = r; }
    }
    for d in 1..=laenge {
        let fx = (x as f64 + ux * d as f64 + 0.5) as i32;
        let fy = (y as f64 + uy * d as f64 + 0.5) as i32;
        for h in 0..nh as usize {
            if fx >= hin[h].x0 && fx < hin[h].x0 + hin[h].w && fy >= hin[h].y0 && fy < hin[h].y0 + hin[h].h { return false; }
        }
    }
    true
}

/// Lambda `ausdehnung` aus koloniePlatz.
pub(super) fn ausdehnung(welt: &mut dyn Welt, z: &Zustand, hin: &[Rechteck], nh: i32, ux: f64, uy: f64) -> f64 {
    let mut ext = 2.0;
    for h in 0..nh as usize {
        if hin[h].w == 1 && hin[h].h == 1 { continue; }
        if abstand2(welt, hin[h].x0 + hin[h].w / 2, hin[h].y0 + hin[h].h / 2, z.bx, z.by) > 20 * 20 { continue; }
        for cx in 0..2 {
            for cy in 0..2 {
                let pr = (hin[h].x0 + cx * hin[h].w - z.bx) as f64 * ux + (hin[h].y0 + cy * hin[h].h - z.by) as f64 * uy;
                if pr > ext { ext = pr; }
            }
        }
    }
    ext
}

/// C++: int koloniePlatz(Zustand& z)  (op2_ki.hpp:2620)
pub fn koloniePlatz(welt: &mut dyn Welt, z: &mut Zustand) -> i32 {
    // Erweiterung Gelaende (Rust): erst die Paesse der Engstelle besetzen
    if PAR(z.spieler).gelaende > 0.0 { let i = super::erweiterung::passTurm(welt, z); if i >= 0 { return i; } }
    // Front statt Ring: Tuerme in einer Linie quer zur Feindrichtung, 3 Kacheln vor dem aeussersten Gebaeude, je 2 auseinander.
    // Ist die Linie voll, kommt eine zweite Reihe. Weitere Seiten erst, wenn von dort schon Feinde kamen (angriffSektoren).
    let mut hin = [Rechteck::default(); 160];
    let nh = hindernisse(welt, z, &mut hin, 160);
    let (mut eux, mut euy) = (0.0f64, 0.0f64);
    feindRichtung(welt, z, z.bx, z.by, &mut eux, &mut euy);
    if PAR(z.spieler).mauerLinie >= 0.5 { (eux, euy) = super::super::mauer::achse(eux, euy); }
    #[derive(Clone, Copy, Default)]
    struct R { ux: f64, uy: f64, gewicht: f64 }
    let mut rt = [R::default(); 9];
    let mut nr = 0usize;
    let pt = PAR(z.spieler);
    rt[nr] = R { ux: eux, uy: euy, gewicht: pt.turmFront }; nr += 1;
    for sk in 0..8 {
        if (z.angriffSektoren & (1 << sk)) == 0 { continue; }
        let w = sk as f64 * 3.14159265358979 / 4.0;
        let (ux, uy) = (w.cos(), w.sin());
        if ux * eux + uy * euy > 0.7 { continue; } // liegt schon in der Front
        rt[nr] = R { ux, uy, gewicht: 1.0 }; nr += 1;
    }
    // Richtung mit den wenigsten Tuermen je Gewicht zuerst
    let mut reihe = [0usize; 9];
    let mut last = [0.0f64; 9];
    for i in 0..nr {
        let r = ausdehnung(welt, z, &hin, nh, rt[i].ux, rt[i].uy) + pt.turmVorne;
        let tx = (z.bx as f64 + rt[i].ux * r + 0.5) as i32;
        let ty = (z.by as f64 + rt[i].uy * r + 0.5) as i32;
        last[i] = tuermeBei(welt, z, tx, ty, 9) as f64 / rt[i].gewicht;
        reihe[i] = i;
    }
    for i in 0..nr { for j in (i + 1)..nr { if last[reihe[j]] < last[reihe[i]] { let t = reihe[i]; reihe[i] = reihe[j]; reihe[j] = t; } } }
    const VERSATZ: [i32; 9] = [0, 1, -1, 2, -2, 3, -3, 4, -4];
    for q in 0..nr {
        let d = rt[reihe[q]];
        let r0 = ausdehnung(welt, z, &hin, nh, d.ux, d.uy) + pt.turmVorne;
        for zeile in 0..3 {
            for k in 0..9 {
                let r = r0 + zeile as f64 * pt.turmAbstand;
                let x = (z.bx as f64 + d.ux * r - d.uy * VERSATZ[k] as f64 * pt.turmAbstand + 0.5) as i32;
                let y = (z.by as f64 + d.uy * r + d.ux * VERSATZ[k] as f64 * pt.turmAbstand + 0.5) as i32;
                let mut g: &'static str = "";
                if !platzFrei(welt, z, x, y, 1, 1, &hin, nh, &mut g) || !schussfeldFrei(welt, z, &hin, nh, x, y, d.ux, d.uy) { continue; }
                super::super::mauer::merkeRichtung(z, x, y, d.ux, d.uy);
                let idx = platzEintrag(welt, z, mapGuardPost, x, y);
                if idx >= 0 {
                    melde(welt, z, "kampf", mapGuardPost, x, y, &format!("Wachturm {} (Richtung {} Grad, Reihe {}, Platz {} der Linie)",
                          if reihe[q] == 0 { "in der Front zum Feind" } else { "auf einer Seite, von der schon angegriffen wurde" },
                          (d.uy.atan2(d.ux) * 180.0 / 3.14159265358979) as i32, zeile + 1, k + 1));
                }
                return idx;
            }
        }
    }
    -1
}

// Turm fuer einen Aussenposten: an der Feindseite der Mine, 3-5 Kacheln, mit freiem Schussfeld.
/// C++: int aussenPlatz(Zustand& z, int a)  (op2_ki.hpp:2684)
pub fn aussenPlatz(welt: &mut dyn Welt, z: &mut Zustand, a: i32) -> i32 {
    let a = a as usize;
    let mut hin = [Rechteck::default(); 160];
    let nh = hindernisse(welt, z, &mut hin, 160);
    let (mut ux, mut uy) = (0.0f64, 0.0f64);
    feindRichtung(welt, z, z.aussenX[a], z.aussenY[a], &mut ux, &mut uy);
    if PAR(z.spieler).mauerLinie >= 0.5 { (ux, uy) = super::super::mauer::achse(ux, uy); }
    for ring in 0..4 {
        for s2 in 0..5 {
            let seite = (if s2 % 2 != 0 { 1 } else { -1 }) * ((s2 + 1) / 2);
            let r = (3 + ring) as f64;
            let x = (z.aussenX[a] as f64 + ux * r - uy * seite as f64 * 1.5 + 0.5) as i32;
            let y = (z.aussenY[a] as f64 + uy * r + ux * seite as f64 * 1.5 + 0.5) as i32;
            let mut g: &'static str = "";
            if !platzFrei(welt, z, x, y, 1, 1, &hin, nh, &mut g) || !schussfeldFrei(welt, z, &hin, nh, x, y, ux, uy) { continue; }
            super::super::mauer::merkeRichtung(z, x, y, ux, uy);
            let idx = platzEintrag(welt, z, mapGuardPost, x, y);
            if idx >= 0 {
                melde(welt, z, "kampf", mapGuardPost, x, y, &format!("Wachturm fuer den Aussenposten bei {},{}", z.aussenX[a], z.aussenY[a]));
            }
            return idx;
        }
    }
    -1
}

// Turm an eine Schmelze ohne Schutz (und Kraftwerke). Feindseite, wie beim Aussenposten. Guard the smelter that holds the ore stock.
/// C++: int schmelzPlatz(Zustand& z)  (op2_ki.hpp:2706)
pub fn schmelzPlatz(welt: &mut dyn Welt, z: &mut Zustand) -> i32 {
    let mut hin = [Rechteck::default(); 160];
    // auch Kraftwerke (Eden verlor sie oft; ohne Strom fielen Wohnhaeuser und Agrardome aus)
    let schutzTypen: [i32; 2] = [mapCommonOreSmelter, mapTokamak];
    for st in 0..2usize {
        if st == 1 && schutzTypen[1] == mapCommonOreSmelter { break; }
        for s in welt.PlayerBuildingEnum(z.spieler, schutzTypen[st]) {
            if !welt.IsLive(s) { continue; }
            let ls = welt.Location(s);
            let (sx, sy) = (edX(welt, z, ls), edY(welt, z, ls));
            if tuermeBei(welt, z, sx, sy, 6) > 0 { continue; }
            let nh = hindernisse(welt, z, &mut hin, 160);
            let (mut ux, mut uy) = (0.0f64, 0.0f64);
            feindRichtung(welt, z, sx, sy, &mut ux, &mut uy);
            if PAR(z.spieler).mauerLinie >= 0.5 { (ux, uy) = super::super::mauer::achse(ux, uy); }
            for ring in 0..4 {
                for s2 in 0..5 {
                    let seite = (if s2 % 2 != 0 { 1 } else { -1 }) * ((s2 + 1) / 2);
                    let r = (4 + ring) as f64;
                    let x = (sx as f64 + ux * r - uy * seite as f64 * 1.5 + 0.5) as i32;
                    let y = (sy as f64 + uy * r + ux * seite as f64 * 1.5 + 0.5) as i32;
                    let mut g: &'static str = "";
                    if !platzFrei(welt, z, x, y, 1, 1, &hin, nh, &mut g) || !schussfeldFrei(welt, z, &hin, nh, x, y, ux, uy) { continue; }
                    super::super::mauer::merkeRichtung(z, x, y, ux, uy);
                    let idx = platzEintrag(welt, z, mapGuardPost, x, y);
                    if idx >= 0 {
                        melde(welt, z, "kampf", mapGuardPost, x, y, &format!("Wachturm fuer {} bei {},{}", if st != 0 { "das Kraftwerk" } else { "die Schmelze" }, sx, sy));
                    }
                    return idx;
                }
            }
        }
    }
    -1
}

/// C++: int wachtPlatz(Zustand& z)  (op2_ki.hpp:2739)
pub fn wachtPlatz(welt: &mut dyn Welt, z: &mut Zustand) -> i32 {
    { let i = super::super::mauer::zugangTurm(welt, z); if i >= 0 { return i; } }
    let a = aussenTurmFehlt(welt, z);
    if a >= 0 { return aussenPlatz(welt, z, a); }
    { let i = schmelzPlatz(welt, z); if i >= 0 { return i; } }
    koloniePlatz(welt, z)
}

// Mauer vor einem wichtigen Gebaeude zur Feindseite: 2 Kacheln vor seiner Front, so breit wie das Gebaeude plus je eine Kachel.
/// C++: void mauerVor(Zustand& z, int x0, int y0, int w, int h)  (op2_ki.hpp:2753)
pub fn mauerVor(welt: &mut dyn Welt, z: &mut Zustand, x0: i32, y0: i32, w: i32, h: i32) {
    let mut hin = [Rechteck::default(); 160];
    let nh = hindernisse(welt, z, &mut hin, 160);
    let cx = x0 as f64 + w as f64 / 2.0;
    let cy = y0 as f64 + h as f64 / 2.0;
    let (mut ux, mut uy) = (0.0f64, 0.0f64);
    feindRichtung(welt, z, cx as i32, cy as i32, &mut ux, &mut uy);
    if PAR(z.spieler).mauerLinie >= 0.5 { (ux, uy) = super::super::mauer::achse(ux, uy); }
    let vorn = (ux.abs() * w as f64 + uy.abs() * h as f64) / 2.0 + 2.0;
    let halb = ((uy.abs() * w as f64 + ux.abs() * h as f64) / 2.0) as i32 + 1;
    let mut n = 0;
    for q in -halb..=halb {
        let x = (cx + ux * vorn - uy * q as f64 + 0.5) as i32;
        let y = (cy + uy * vorn + ux * q as f64 + 0.5) as i32;
        let mut g: &'static str = "";
        if !platzFreiOhneMauer(welt, z, x, y, 1, 1, &hin, nh, &mut g) || super::super::mauer::torIn(z, x, y, 1, 1) { continue; }
        super::super::mauer::vormerken(welt, z, x, y);
        n += 1;
    }
    if n != 0 { melde(welt, z, "kampf", mapWall, cx as i32, cy as i32, &format!("{} Mauerkacheln vor {},{} zur Feindseite", n, x0, y0)); }
}

// Mauern neben einem Wachturm: je 2 Kacheln links und rechts, quer zur Feindrichtung. Nicht davor -- ein Laser schiesst nicht ueber Mauern.
/// C++: void mauerNeben(Zustand& z, int tx, int ty)  (op2_ki.hpp:2771)
pub fn mauerNeben(welt: &mut dyn Welt, z: &mut Zustand, tx: i32, ty: i32) {
    let mut hin = [Rechteck::default(); 160];
    let nh = hindernisse(welt, z, &mut hin, 160);
    let (mut ux, mut uy) = (0.0f64, 0.0f64);
    feindRichtung(welt, z, tx, ty, &mut ux, &mut uy);
    let mut gesetzt = 0;
    let linie = PAR(z.spieler).mauerLinie >= 0.5;
    if linie {
        (ux, uy) = super::super::mauer::richtungFuer(z, tx, ty).unwrap_or_else(|| super::super::mauer::achse(ux, uy));
        super::super::mauer::linie(welt, z, tx, ty, ux, uy);
    }
    const SEITE: [i32; 6] = [-2, -3, 2, 3, -4, 4];
    for si in 0..(if linie { 0 } else { 6 }) {
        let seite = SEITE[si];
        let x = (tx as f64 - uy * seite as f64 + 0.5) as i32;
        let y = (ty as f64 + ux * seite as f64 + 0.5) as i32;
        let mut g: &'static str = "";
        if !platzFreiOhneMauer(welt, z, x, y, 1, 1, &hin, nh, &mut g) || super::super::mauer::torIn(z, x, y, 1, 1) { continue; }
        super::super::mauer::vormerken(welt, z, x, y);
        gesetzt += 1;
    }
    // Mauern davor: lohnt nur, wenn der eigene Turm darueber schiessen kann und mindestens die Haelfte der feindlichen
    // Feuerkraft nicht. Walls in front only if our tower fires over them and at least half of the enemy firepower cannot.
    {
        let wf = eigeneWaffe(welt, z);
        let (mut direkt, mut gesamt) = (0, 0);
        for p in 0..welt.NoPlayers() {
            if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { continue; }
            for u in welt.PlayerVehicleEnum(p) {
                if !welt.IsLive(u) || kampfWert(welt, u) <= 0 { continue; }
                let k = kampfWert(welt, u);
                let w = welt.GetWeapon(u);
                gesamt += k;
                if w == mapLaser || w == mapMicrowave || w == mapRailGun { direkt += k; }
            }
        }
        if wf != mapLaser && wf != mapMicrowave && wf != mapRailGun && gesamt > 0 && direkt * 2 >= gesamt {
            let mut vorn = 0;
            for q in -1..=1 {
                let x = (tx as f64 + ux * 2.0 - uy * q as f64 + 0.5) as i32;
                let y = (ty as f64 + uy * 2.0 + ux * q as f64 + 0.5) as i32;
                let mut g: &'static str = "";
                if !platzFreiOhneMauer(welt, z, x, y, 1, 1, &hin, nh, &mut g) || super::super::mauer::torIn(z, x, y, 1, 1) { continue; }
                super::super::mauer::vormerken(welt, z, x, y);
                vorn += 1;
            }
            if vorn != 0 {
                let wn = name(welt, wf);
                melde(welt, z, "kampf", mapWall, tx, ty, &format!("{} Mauerkacheln vor dem Wachturm ({} schiesst darueber, Feind zu {} % direkt)", vorn, wn, direkt * 100 / gesamt));
            }
        }
    }
    if gesetzt != 0 { melde(welt, z, "kampf", mapWall, tx, ty, &format!("{} Mauerkacheln neben dem Wachturm aufgezeichnet (Schussfeld nach vorn frei)", gesetzt)); }
}

/// C++: bool bauAufzeichnen(Zustand& z, int typ, const char* warum)  (op2_ki.hpp:2818)
pub fn bauAufzeichnen(welt: &mut dyn Welt, z: &mut Zustand, typ: i32, warum: &str) -> bool {
    // Ohne freie Bucht und ohne passenden Bausatz bliebe die BuildingGroup an diesem Auftrag haengen.
    // Without a free bay and no matching kit the BuildingGroup would hang on this order.
    if vorratAktiv(z) && !bausatzMoeglich(welt, z, typ) { return false; }
    let i = if let Some(i) = super::super::befehl::platzWunsch(welt, z, typ) { i }
            else if typ == mapGuardPost { wachtPlatz(welt, z) } else { platzWaehlen(welt, z, typ) };
    if i < 0 { return false; }
    let iu = i as usize;
    let g = grundflaeche(welt, typ);
    let (px0, py0) = (z.plaetze[iu].x0 as i32, z.plaetze[iu].y0 as i32);
    let ort = engine(welt, z, px0 + g.w / 2, py0 + g.h / 2);
    let waffe = if typ == mapGuardPost { eigeneWaffe(welt, z) } else { mapNone };
    welt.RecordBuilding(super::super::zweitbasis::gruppeAm(z, px0, py0), ort, typ, waffe);
    // Missionsmodus: keine Mauerstuecke an jedem Turm, die Mauern stehen an den Zugaengen aus dem Gelaende (mauer.rs)
    if typ == mapGuardPost && !super::super::mauer::istZugangTurm(z, px0, py0) && !super::super::modus::mission(z) { mauerNeben(welt, z, px0, py0); }
    if z.feindGemeldet != 0 && (typ == mapTokamak || typ == mapCommonOreSmelter || typ == mapCommandCenter || typ == mapVehicleFactory) {
        let gg = grundflaeche(welt, typ);
        mauerVor(welt, z, px0, py0, gg.w, gg.h);
    }
    let tick = welt.Tick();
    {
        let p = &mut z.plaetze[iu];
        p.zustand = P_FAHRT; p.seit = tick; p.convec = 0;
    }
    let (px0, py0) = (z.plaetze[iu].x0 as i32, z.plaetze[iu].y0 as i32);
    let (ox, oy) = (edX(welt, z, ort), edY(welt, z, ort));
    melde(welt, z, "bau", typ, px0, py0, &format!("BuildingGroup baut: {} (Platz {}x{}, Mitte {},{})", warum, g.w, g.h, ox, oy));
    true
}

// Freier Warteplatz: nahe dem Wunschpunkt ein Feld von (2h+1) x (2h+1) Kacheln, auf dem samt 2 Kacheln Rand kein eigenes Gebaeude
// steht, das nicht auf dem Weg einer Erzroute liegt, befahrbar ist und -- fuer die Wache -- keinen Turm in der Naehe hat.
// Ringsuche, hoechstens 11 Kacheln von der Basismitte. Free waiting area near the wish point.
/// C++: void freierWarteplatz(const Zustand& z, int& x, int& y, int h, bool turmAbstand)  (op2_ki.hpp:2841)
pub fn freierWarteplatz(welt: &mut dyn Welt, z: &Zustand, x: &mut i32, y: &mut i32, h: i32, turmAbstand: bool) {
    let mut turmAbstand = turmAbstand;
    let mut rx0 = [0i32; 160];
    let mut ry0 = [0i32; 160];
    let mut rx1 = [0i32; 160];
    let mut ry1 = [0i32; 160];
    let mut nr = 0usize;
    for &t in GEBAEUDE.iter() {
        for b in welt.PlayerBuildingEnum(z.spieler, t) {
            if nr >= 160 { break; }
            if !welt.IsLive(b) { continue; }
            let lo = welt.Location(b);
            let g = grundflaeche(welt, t);
            rx0[nr] = edX(welt, z, lo) - g.w / 2 - 2; ry0[nr] = edY(welt, z, lo) - g.h / 2 - 2;
            rx1[nr] = rx0[nr] + g.w + 3; ry1[nr] = ry0[nr] + g.h + 3; nr += 1;
        }
    }
    // Erzrouten als Strecken
    let mut sx0 = [0.0f64; 8];
    let mut sy0 = [0.0f64; 8];
    let mut sx1 = [0.0f64; 8];
    let mut sy1 = [0.0f64; 8];
    let mut ns = 0usize;
    let mut paare: Vec<(i32, i32)> = Vec::new();
    for i in 0..MAX_MINEN { paare.push((z.minenId[i], z.schmelzeId[i])); }
    for i in 0..2 { paare.push((z.seltenMineId[i], z.seltenSchmelzeId[i])); }
    for &(mine, schmelze) in paare.iter() {
        // strecke(mine, schmelze)
        if mine == 0 || schmelze == 0 || ns >= 8 { continue; }
        let (m, s2): (Id, Id) = (mine, schmelze);
        if !welt.IsLive(m) || !welt.IsLive(s2) { continue; }
        let a = welt.Location(m);
        let b = welt.Location(s2);
        sx0[ns] = edX(welt, z, a) as f64; sy0[ns] = edY(welt, z, a) as f64;
        sx1[ns] = edX(welt, z, b) as f64; sy1[ns] = edY(welt, z, b) as f64; ns += 1;
    }
    let frei = |welt: &mut dyn Welt, cx: i32, cy: i32, turmAbstand: bool| -> bool {
        if !aufKarte(welt, z, cx - h, cy - h, 1) || !aufKarte(welt, z, cx + h, cy + h, 1) { return false; }
        let c = zelle(welt, z, cx, cy);
        if !zelleBefahrbar(welt, c) { return false; }
        for i in 0..nr { if cx + h >= rx0[i] && cx - h <= rx1[i] && cy + h >= ry0[i] && cy - h <= ry1[i] { return false; } }
        for i in 0..ns {
            let (vx, vy) = (sx1[i] - sx0[i], sy1[i] - sy0[i]);
            let ll = vx * vx + vy * vy;
            let mut u = if ll > 0.0 { ((cx as f64 - sx0[i]) * vx + (cy as f64 - sy0[i]) * vy) / ll } else { 0.0 };
            if u < 0.0 { u = 0.0; }
            if u > 1.0 { u = 1.0; }
            let dx = cx as f64 - (sx0[i] + u * vx);
            let dy = cy as f64 - (sy0[i] + u * vy);
            if dx * dx + dy * dy < (h as f64 + 2.5) * (h as f64 + 2.5) { return false; }
        }
        if turmAbstand && tuermeBei(welt, z, cx, cy, h + 3) > 0 { return false; }
        true
    };
    for durchgang in 0..2 {
        if durchgang == 1 { if !turmAbstand { break; } turmAbstand = false; }
        for r in 0..=14 {
            let (mut bx, mut by) = (0i32, 0i32);
            let mut bd = 1e9f64;
            for dy in -r..=r {
                for dx in -r..=r {
                    if (if dx < 0 { -dx } else { dx }) != r && (if dy < 0 { -dy } else { dy }) != r { continue; }
                    let d = (dx * dx + dy * dy) as f64;
                    if abstand2(welt, *x + dx, *y + dy, z.bx, z.by) > 11 * 11 { continue; } // in der Basis bleiben
                    if d < bd && frei(welt, *x + dx, *y + dy, turmAbstand) { bd = d; bx = dx; by = dy; }
                }
            }
            if bd < 1e9 { *x += bx; *y += by; return; }
        }
    }
}

// Gemerkte Platzmitten je Spieler (g_platz in globale.rs): 0 Wache, 1 Sammelgruppe, 2 Reserve daheim, 3 Baugruppe, 4 Fabrikgruppe.


/// Kann eine Strukturfabrik den Bausatz `typ` liefern: liegt er schon in einer Bucht, oder ist eine Bucht frei?
/// Ohne Strukturfabrik: ja (dann entscheidet anderes). Rust 2026-11-26 (Savant in op2-rs: volle Buchten der Mission).
/// Can a Structure Factory deliver kit `typ`: is it in a bay already, or is a bay free?
pub fn bausatzMoeglich(welt: &mut dyn Welt, z: &Zustand, typ: i32) -> bool {
    let sfs: Vec<Id> = welt.PlayerBuildingEnum(z.spieler, mapStructureFactory).into_iter().filter(|&f| welt.IsLive(f)).collect();
    if sfs.is_empty() { return true; }
    sfs.iter().any(|&f| (0..6).any(|b| { let k = welt.GetFactoryCargo(f, b); k == typ || k == mapNone }))
}

/// Bausaetze aus dem Vorrat: was schon in den Buchten der Strukturfabriken liegt (eine Mission kann sie gefuellt
/// haben), wird zuerst aufgebaut -- es kostet nichts mehr, und volle Buchten halten sonst jeden neuen Bausatz auf.
/// Rust 2026-11-26. Kits in stock: what is already in the Structure Factories' bays is built first.
/// Vorrat beachten: wenn die Strukturfabriken zu Spielbeginn Bausaetze hatten (eine Mehrspielerkarte stattet sie aus), oder
/// mit PAR vorrat auch fuer spaeter liegengebliebene. Use the stock: if the factories had kits at the start, or with PAR vorrat.
pub fn vorratAktiv(z: &Zustand) -> bool { z.vorratStart == 1 || PAR(z.spieler).vorrat >= 0.5 }

/// Beim ersten Schritt, vor dem ersten Bauauftrag: lagen zu Spielbeginn Bausaetze in den Buchten?
/// At the first step, before the first build order: were there kits in the bays at the start?
pub fn vorratPruefen(welt: &mut dyn Welt, z: &mut Zustand) {
    if z.vorratStart == 0 {
        let sfs: Vec<Id> = welt.PlayerBuildingEnum(z.spieler, mapStructureFactory).into_iter().filter(|&f| welt.IsLive(f)).collect();
        let mut typen: Vec<i32> = Vec::new();
        for &f in &sfs { for b in 0..6 { let k = welt.GetFactoryCargo(f, b); if k != mapNone { typen.push(k); } } }
        let kits: Vec<String> = typen.iter().map(|&k| name(welt, k).to_string()).collect();
        z.vorratStart = if kits.is_empty() { 2 } else { 1 };
        // Ein ConVec stellt die Startbausaetze auf, waehrend die BuildingGroup mit den anderen baut (sie nutzt immer nur
        // einen). Jetzt reservieren, bevor die Gruppe ihn aufnimmt -- aus der Gruppe holen liess das Original abstuerzen.
        // One ConVec sets up the starting kits while the BuildingGroup builds with the others; reserved before the
        // group takes it (taking units out of the group crashed the original).
        if !kits.is_empty() {
            let cvs: Vec<Id> = welt.PlayerVehicleEnum(z.spieler).into_iter().filter(|&u| welt.IsLive(u) && welt.GetType(u) == mapConVec).collect();
            if cvs.len() >= 2 { z.vorratCv = *cvs.iter().max().unwrap(); }
        }
        if !kits.is_empty() {
            melde(welt, z, "info", mapStructureFactory, z.bx, z.by, &format!("Bausaetze zu Spielbeginn in der Strukturfabrik: {} -- werden zuerst gebaut", kits.join(", ")));
        }
    }
}

/// Der Vorrat-ConVec: Bausatz aus der Bucht laden (Fahrzeugfabrik zuerst), selbst aufstellen, naechster; am Ende geht er in
/// die BuildingGroup. The stock ConVec: load a kit from a bay (vehicle factory first), set it up, next one; at the end it
/// joins the BuildingGroup.
fn vorratConVec(welt: &mut dyn Welt, z: &mut Zustand) {
    if z.vorratCv <= 0 { return; }
    let c = z.vorratCv;
    let t = welt.Tick();
    if !welt.IsLive(c) { z.vorratCv = -1; z.vorratPlatz = 0; return; }
    if z.vorratPlatz > 0 {
        let i = (z.vorratPlatz - 1) as usize;
        let q = z.plaetze[i];
        if q.zustand == P_STEHT || stehtDa(welt, z, &q) {
            z.plaetze[i].zustand = P_STEHT; z.plaetze[i].seit = t;
            melde(welt, z, "fertig", q.typ as i32, q.x0 as i32, q.y0 as i32, "Gebaeude steht (Vorrat-ConVec)");
            z.vorratPlatz = 0;
        } else if t - z.vorratSeit > 6000 {
            z.plaetze[i].zustand = P_GESPERRT;
            melde(welt, z, "warte", q.typ as i32, q.x0 as i32, q.y0 as i32, "Vorrat-ConVec: Bau kommt nicht zustande, Platz gesperrt");
            z.vorratPlatz = 0;
        } else if welt.GetCurAction(c) == moDone && !welt.IsBusy(c) && t - z.vorratSeit > 200 && welt.GetCargo(c) > 0 && welt.GetCargo(c) != mapNone {
            // noch beladen und untaetig: Bauauftrag erneuern. Still loaded and idle: repeat the order.
            let g = grundflaeche(welt, q.typ as i32);
            let ecke = engine(welt, z, q.x0 as i32 + g.w, q.y0 as i32 + g.h);
            welt.DoBuild(c, q.typ as i32, ecke);
            z.vorratSeit = t;
        }
        return;
    }
    let kit = welt.GetCargo(c);
    if kit > 0 && kit != mapNone && GEBAEUDE.contains(&kit) {
        if welt.GetCurAction(c) != moDone || welt.IsBusy(c) { return; }
        let i = platzWaehlen(welt, z, kit);
        if i < 0 { return; }
        let iu = i as usize;
        z.plaetze[iu].zustand = P_FAHRT; z.plaetze[iu].seit = t; z.plaetze[iu].convec = c;
        let (x0, y0) = (z.plaetze[iu].x0 as i32, z.plaetze[iu].y0 as i32);
        let g = grundflaeche(welt, kit);
        let ecke = engine(welt, z, x0 + g.w, y0 + g.h);
        welt.DoBuild(c, kit, ecke);
        z.vorratPlatz = i + 1; z.vorratSeit = t;
        melde(welt, z, "bau", kit, x0, y0, &format!("Vorrat-ConVec {} stellt den Startbausatz auf (Platz {}x{})", c, g.w, g.h));
        return;
    }
    // Leer: naechsten Bausatz aus einer Bucht holen. Empty: fetch the next kit from a bay.
    const REIHE: [i32; 6] = [mapVehicleFactory, mapTokamak, mapResidence, mapNursery, mapUniversity, mapRobotCommand];
    let sfs: Vec<Id> = welt.PlayerBuildingEnum(z.spieler, mapStructureFactory).into_iter().filter(|&f| welt.IsLive(f)).collect();
    let mut wahl: Option<(usize, Id, i32)> = None;
    let mut irgendeiner = false;   // liegt ueberhaupt noch ein Startbausatz? any kit left at all?
    for &f in &sfs {
        for b in 0..6 {
            let k = welt.GetFactoryCargo(f, b);
            if k == mapNone || !GEBAEUDE.contains(&k) { continue; }
            irgendeiner = true;
            if welt.GetCurAction(f) != moDone { continue; }   // Fabrik beschaeftigt: spaeter. Factory busy: later.
            // nicht, wofuer die BuildingGroup schon einen Auftrag hat. Not what the group already has an order for.
            if z.plaetze.iter().any(|p| p.typ as i32 == k && p.zustand == P_FAHRT) { continue; }
            let r = REIHE.iter().position(|&x| x == k).unwrap_or(REIHE.len());
            if wahl.map_or(true, |w| r < w.0) { wahl = Some((r, f, b)); }
        }
    }
    let Some((_r, f, b)) = wahl else {
        if irgendeiner { return; }
        // Vorrat leer: der ConVec geht in die BuildingGroup. Stock empty: the ConVec joins the group.
        if z.bauAn != 0 && welt.GetCurAction(c) == moDone { welt.TakeUnit(z.bau, c); z.vorratCv = -1;
            melde(welt, z, "info", mapConVec, z.bx, z.by, "Vorrat-ConVec: alle Startbausaetze aufgestellt, er geht in die BuildingGroup"); }
        return;
    };
    let dock = welt.GetDockLocation(f);
    let cl = welt.Location(c);
    if cl.x == dock.x && cl.y == dock.y {
        if t - z.vorratSeit > 500 { welt.DoTransferCargo(f, b); z.vorratSeit = t; }
    } else if welt.GetCurAction(c) == moDone || t - z.vorratSeit > 3000 {
        welt.DoDock(c, dock); z.vorratSeit = t;
    }
}

pub fn vorratAufbauen(welt: &mut dyn Welt, z: &mut Zustand) {
    vorratPruefen(welt, z);
    if !vorratAktiv(z) { return; }
    vorratConVec(welt, z);
    let sfs: Vec<Id> = welt.PlayerBuildingEnum(z.spieler, mapStructureFactory).into_iter().filter(|&f| welt.IsLive(f)).collect();
    let mut im_vorrat: Vec<i32> = Vec::new();
    for &f in &sfs {
        for b in 0..6 {
            let k = welt.GetFactoryCargo(f, b);
            if k != mapNone && GEBAEUDE.contains(&k) { im_vorrat.push(k); }
        }
    }
    // Immer nur ein Vorrat-Bausatz unterwegs: die BuildingGroup baut der Reihe nach, und eigene Vorhaben (Labor fuer
    // die Kampfforschung) reihen sich so abwechselnd ein, sobald eine Bucht frei ist.
    // Only one stock kit under way at a time, so the AI's own plans (the lab for combat research) take turns.
    // Labor zuerst: solange es kein AdvancedLab gibt und keins unterwegs ist, nur einen Bausatz nehmen, damit eine
    // Bucht frei wird -- die Kampfforschung (Lynx) braucht das Labor. Danach der Rest, einzeln.
    // Lab first: until an Advanced Lab stands or is on its way, take only one kit so a bay frees up.
    let labor = welt.PlayerBuildingEnum(z.spieler, mapAdvancedLab).into_iter().any(|u| welt.IsLive(u))
        || z.plaetze.iter().any(|p| p.typ as i32 == mapAdvancedLab && p.zustand != P_FREI);
    let unterwegs = z.plaetze.iter().filter(|p| p.zustand == P_FAHRT && im_vorrat.contains(&(p.typ as i32))).count();
    // Ist eine Bucht frei, kommt das Labor vor allen weiteren Bausaetzen. A free bay: the lab goes first.
    if !labor && bausatzMoeglich(welt, z, mapAdvancedLab) && !im_vorrat.contains(&mapAdvancedLab) {
        if bauAufzeichnen(welt, z, mapAdvancedLab, "Labor zuerst: AdvancedLab fuer die Kampfforschung vor den uebrigen Startbausaetzen") {
            return;
        }
    }
    if unterwegs > 0 || z.vorratCv > 0 { return; }
    const REIHE: [i32; 6] = [mapTokamak, mapVehicleFactory, mapResidence, mapNursery, mapUniversity, mapRobotCommand];
    let rang = |k: i32| REIHE.iter().position(|&r| r == k).unwrap_or(REIHE.len());
    let mut wahl = im_vorrat.clone();
    wahl.sort_by_key(|&k| rang(k));
    wahl.dedup();
    // Nur Bausaetze, fuer die noch kein Auftrag laeuft. Only kits without an order yet.
    let frei = wahl.into_iter().find(|&k| {
        let vorrat = im_vorrat.iter().filter(|&&x| x == k).count();
        let unterwegs = z.plaetze.iter().filter(|p| p.typ as i32 == k && p.zustand == P_FAHRT).count();
        vorrat > unterwegs
    });
    if let Some(k) = frei {
        bauAufzeichnen(welt, z, k, "Bausatz aus dem Vorrat der Strukturfabrik");
    }
}
