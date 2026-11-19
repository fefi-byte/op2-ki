//! Erweiterung der Rust-Fassung (nicht in der C++-KI): eine zweite Basis an einem
//! entfernten Bakenfeld -- Kommandozentrale und Schmelze (ueber die BuildingGroup der KI), dann Wachtuerme, Fahrzeugfabrik
//! und auf Wunsch eine Strukturfabrik; die Mine baut ein Robo-Miner, die Erzroute legt erz() an (naechste Mine zur Schmelze).
//! Ob es sich lohnt, entscheidet der Ertrag im Planungshorizont gegen die Kosten -- keine feste Regel.
//! Second base at a distant beacon field, decided by yield over the horizon against cost and risk.
use std::cell::RefCell;

use super::gelaende::Gelaende;
use super::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct Zb {
    /// 0 pruefen, 1 Kommandozentrale und Schmelze aufgezeichnet, 2 Kommandozentrale steht (Tuerme, Fabriken, Mine)
    pub phase: i32,
    pub x: i32,
    pub y: i32,
    pub bake: Id,
    pub seit: i32,
    pub naechste: i32,
    pub tuerme: i32,
    pub fabrik: i32,
    pub struktur: i32,
    pub minerSeit: i32,
}

/// Hoechstzahl Zweitbasen je Spieler (PAR zbAnzahl wird darauf begrenzt)
pub const MAX_ZB: usize = 4;

thread_local! {
    pub static ZB: RefCell<[[Zb; MAX_ZB]; 8]> = RefCell::new([[Zb::default(); MAX_ZB]; 8]);
    /// Bakenfelder ohne Platz fuer die Kommandozentrale: (Bake, gesperrt bis Tick) je Spieler
    static SPERRE: RefCell<[Vec<(Id, i32)>; 8]> = RefCell::new(Default::default());
}
/// Naechste feindliche Kommandozentrale (Editor-Kacheln), sonst irgendein feindliches Gebaeude.
pub fn feindBasis(welt: &mut dyn Welt, z: &Zustand) -> Option<(i32, i32)> {
    let mut best: Option<(i32, (i32, i32))> = None;
    for p in 0..welt.NoPlayers() {
        if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { continue; }
        let mut ids = welt.PlayerBuildingEnum(p, mapCommandCenter);
        if ids.is_empty() { ids = welt.PlayerBuildingEnum(p, mapNone); }
        for u in ids {
            let l = welt.Location(u);
            let (x, y) = (edX(welt, z, l), edY(welt, z, l));
            let d = abstand2(welt, x, y, z.bx, z.by);
            if best.is_none_or(|b| d < b.0) { best = Some((d, (x, y))); }
        }
    }
    best.map(|b| b.1)
}

pub fn zuruecksetzen_zweitbasis() { ZB.with(|z| *z.borrow_mut() = [[Zb::default(); MAX_ZB]; 8]); SPERRE.with(|s| *s.borrow_mut() = Default::default()); }
pub fn zb(sp: i32, k: usize) -> Zb { ZB.with(|z| z.borrow()[(sp & 7) as usize][k]) }
fn setze(sp: i32, k: usize, v: Zb) { ZB.with(|z| z.borrow_mut()[(sp & 7) as usize][k] = v); }

/// Gebaeude ueber die BuildingGroup an der Zweitbasis aufzeichnen: Platz mit den Regeln von platzWaehlen, aber um (sx, sy).
fn aufzeichnenBei(welt: &mut dyn Welt, z: &mut Zustand, typ: i32, sx: i32, sy: i32, warum: &str) -> bool {
    let (bx, by) = (z.bx, z.by);
    z.bx = sx; z.by = sy;
    let i = platzWaehlen(welt, z, typ);
    z.bx = bx; z.by = by;
    if i < 0 { return false; }
    let iu = i as usize;
    let g = grundflaeche(welt, typ);
    let (px0, py0) = (z.plaetze[iu].x0 as i32, z.plaetze[iu].y0 as i32);
    let ort = engine(welt, z, px0 + g.w / 2, py0 + g.h / 2);
    welt.RecordBuilding(z.bau, ort, typ, mapNone);
    let t = welt.Tick();
    { let p = &mut z.plaetze[iu]; p.zustand = P_FAHRT; p.seit = t; p.convec = 0; }
    melde(welt, z, "bau", typ, px0, py0, &format!("Zweitbasis: {} (Platz {}x{}, Mitte {},{})", warum, g.w, g.h, px0 + g.w / 2, py0 + g.h / 2));
    true
}

/// Wachturm der Zweitbasis: zum Feind hin, 4-7 Kacheln vor der Mitte, Linie quer, freies Schussfeld.
fn turmBei(welt: &mut dyn Welt, z: &mut Zustand, sx: i32, sy: i32) -> bool {
    let mut hin = [Rechteck::default(); 160];
    let nh = hindernisse(welt, z, &mut hin, 160);
    let (mut ux, mut uy) = (0.0f64, 0.0f64);
    feindRichtung(welt, z, sx, sy, &mut ux, &mut uy);
    const VERSATZ: [i32; 7] = [0, 1, -1, 2, -2, 3, -3];
    for r in 4..=7 {
        for v in VERSATZ {
            let x = (sx as f64 + ux * r as f64 - uy * v as f64 * 2.0 + 0.5) as i32;
            let y = (sy as f64 + uy * r as f64 + ux * v as f64 * 2.0 + 0.5) as i32;
            let mut g: &'static str = "";
            if !platzFrei(welt, z, x, y, 1, 1, &hin, nh, &mut g) || !schussfeldFrei(welt, z, &hin, nh, x, y, ux, uy) { continue; }
            let i = platzEintrag(welt, z, mapGuardPost, x, y);
            if i < 0 { return false; }
            let ort = engine(welt, z, x, y);
            let waffe = eigeneWaffe(welt, z);
            welt.RecordBuilding(z.bau, ort, mapGuardPost, waffe);
            mauerNeben(welt, z, x, y);
            let t = welt.Tick();
            { let p = &mut z.plaetze[i as usize]; p.zustand = P_FAHRT; p.seit = t; p.convec = 0; }
            melde(welt, z, "bau", mapGuardPost, x, y, "Zweitbasis: Wachturm zum Feind hin");
            return true;
        }
    }
    false
}

fn bakeErtrag(welt: &mut dyn Welt, z: &Zustand, b: Id) -> f64 {
    let mut bb = b;
    let balken = bakenBalken(welt, z, &mut bb, None);      // 1..3 (unvermessen: 2)
    let f = match balken { 3 => 1.0, 2 => 0.8, _ => 0.6 };
    if welt.GetOreType(b) == 1 { 2.0 * f } else { f }
}

/// Bestes Bakenfeld fuer eine Zweitbasis: (Nutzen, x, y, Bake). Nutzen = Mehrertrag im Horizont / Kosten.
fn bakenfeld(welt: &mut dyn Welt, z: &Zustand, l: &Lage) -> Option<(f64, i32, i32, Id)> {
    let p = PAR(z.spieler);
    let feind = feindBasis(welt, z)?;
    let (b, h) = (z.breite, z.hoehe);
    let mut zellen = vec![0i32; (b * h) as usize];
    for y in 0..h { for x in 0..b { let lo = engine(welt, z, x, y); zellen[(y * b + x) as usize] = welt.GetCellType(lo); } }
    let g = Gelaende::neu(&zellen, b, h, (z.bx, z.by), feind);
    // Baken: ueber alle Einheitennummern wie bakeSuchen
    let bis = welt.hoechsteEinheit();
    let mut baken: Vec<(Id, i32, i32)> = Vec::new();
    for id in 1..=bis.max(0) {
        if !welt.IsLive(id) || welt.GetType(id) != mapMiningBeacon { continue; }
        let lo = welt.Location(id);
        if mineAuf(welt, lo) { continue; }
        baken.push((id, edX(welt, z, lo), edY(welt, z, lo)));
    }
    for &(gx, gy) in &[(z.bx, z.by)] { let _ = (gx, gy); }
    let selten_ok = hatTech(welt, z, TECH_SELTENERZ);
    let kosten = welt.GetOreCost(mapCommandCenter, z.spieler) + welt.GetOreCost(mapCommonOreSmelter, z.spieler)
        + (p.zbTuerme as i32) * (welt.GetOreCost(mapGuardPost, z.spieler) + 450)
        + if p.zbFabrik > 0.0 { welt.GetOreCost(mapVehicleFactory, z.spieler) } else { 0 }
        + if p.zbStruktur > 0.0 { welt.GetOreCost(mapStructureFactory, z.spieler) } else { 0 }
        + 2 * welt.GetOreCost(mapCargoTruck, z.spieler) + welt.GetOreCost(mapRoboMiner, z.spieler);
    let mut best: Option<(f64, i32, i32, Id)> = None;
    for &(id, x, y) in &baken {
        if welt.GetOreType(id) == 1 && !selten_ok { continue; }
        let t = welt.Tick();
        if SPERRE.with(|sp| sp.borrow()[(z.spieler & 7) as usize].iter().any(|&(bk, bis)| bk == id && bis > t)) { continue; }
        // Feld einer bestehenden Zweitbasis: tabu
        if (0..MAX_ZB).any(|k| { let s = zb(z.spieler, k); s.phase > 0 && (s.x - x).pow(2) + (s.y - y).pow(2) <= 12 * 12 }) { continue; }
        let i = (y * b + x) as usize;
        let (de, df) = (g.d_eigen[i], g.d_feind[i]);
        if de < 0 || (de as f64) < p.zbMinWeg { continue; }
        if df >= 0 && (df as f64) < p.zbFeindAbstand * de as f64 { continue; }
        // Ertrag: eine gesaettigte Route (1400 je 1000 Ticks) je Bake im Feld (hoechstens 2), gewichtet nach Balken/Erzart,
        // gegen das, was dieselbe Bake von der Hauptbasis aus braechte (lange Lastwagenfahrt, lastwagenFuerAbstand).
        let mut ertrag = 0.0; let mut n = 0;
        for &(id2, x2, y2) in &baken {
            if n >= 2 || (x2 - x).abs() > 8 || (y2 - y).abs() > 8 { continue; }
            if welt.GetOreType(id2) == 1 && !selten_ok { continue; }
            let d = g.d_eigen[(y2 * b + x2) as usize].max(1) as f64;
            let jeLkwFern = 500.0 * 1000.0 / (300.0 + 60.0 * d);
            let fern = (jeLkwFern * 2.0).min(1400.0);            // zwei Lastwagen von daheim
            ertrag += (1400.0 - fern).max(0.0) * bakeErtrag(welt, z, id2);
            n += 1;
        }
        let nutzen = ertrag * p.horizont / 1000.0 / kosten.max(1) as f64;
        if best.is_none_or(|bb| nutzen > bb.0) { best = Some((nutzen, x, y, id)); }
    }
    let _ = l;
    best
}

/// Ein Schritt der Zweitbasen (aus ki::schritt, nach der Bau-Wahl): laufende weiterfuehren, eine neue beginnen, solange
/// weniger als zbAnzahl stehen und die Lage es erlaubt.
pub fn schritt(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    let p = PAR(z.spieler);
    let anzahl = (p.zbAnzahl.max(0.0) as usize).min(MAX_ZB);
    let aktiv = (0..MAX_ZB).filter(|&k| zb(z.spieler, k).phase > 0).count();
    for k in 0..MAX_ZB {
        let s = zb(z.spieler, k);
        if s.phase > 0 { platzSchritt(welt, z, l, k); }
    }
    if aktiv < anzahl {
        if let Some(k) = (0..MAX_ZB).find(|&k| zb(z.spieler, k).phase == 0) { platzSchritt(welt, z, l, k); }
    }
}

/// Bereit fuer eine neue Zweitbasis? None = ja, sonst der Grund. Die Hauptbasis steht (Fahrzeugfabrik, Armee hat ihr
/// Soll, kein Bauplatz offen), Erz ist uebrig und genug Arbeiter sind frei.
fn bereit(welt: &mut dyn Welt, z: &Zustand, l: &Lage) -> Option<String> {
    let p = PAR(z.spieler);
    let mut g: Vec<String> = Vec::new();
    if l.geb[mapCommonOreSmelter as usize] < 1 || l.geb[mapCommonOreMine as usize] < 1 { g.push("daheim fehlt Mine oder Schmelze".into()); }
    if l.vf == 0 { g.push("noch keine Fahrzeugfabrik".into()); }
    if z.sollArmee <= 0 || (z.eigeneArmee as f64) < p.zbArmee * z.sollArmee as f64 { g.push(format!("Armee {} unter {:.0} % ihres Solls {}", z.eigeneArmee, p.zbArmee * 100.0, z.sollArmee)); }
    if z.bedrohung as f64 >= p.zbBedrohung || z.feindArmee as f64 > p.zbFeind * z.eigeneArmee as f64 { g.push(format!("Bedrohung {}, Feindarmee {} gegen eigene {}", z.bedrohung, z.feindArmee, z.eigeneArmee)); }
    let offen = (0..z.anzahlPlaetze.max(0) as usize).filter(|&i| z.plaetze[i].zustand == P_FAHRT || z.plaetze[i].zustand == P_BAUT).count();
    if offen as f64 > p.zbOffen { g.push(format!("{} Bauplaetze daheim noch offen", offen)); }
    let kosten = (welt.GetOreCost(mapCommandCenter, z.spieler) + welt.GetOreCost(mapCommonOreSmelter, z.spieler)) as f64;
    if p.zbUeberschuss >= 0.5 && z.erzUeberschuss == 0 { g.push("kein Erzueberschuss".into()); }
    if (l.erz as f64) < p.zbErz * kosten { g.push(format!("Erz {} unter {:.0}", l.erz, p.zbErz * kosten)); }
    let arbeiter = welt.GetWorkersRequired(mapCommandCenter, z.spieler) + welt.GetWorkersRequired(mapCommonOreSmelter, z.spieler)
        + if p.zbFabrik > 0.0 { welt.GetWorkersRequired(mapVehicleFactory, z.spieler) } else { 0 };
    if (l.arbFrei as f64) < p.zbArbeiter * arbeiter as f64 { g.push(format!("nur {} von {:.0} Arbeitern frei", l.arbFrei, p.zbArbeiter * arbeiter as f64)); }
    if g.is_empty() { None } else { Some(g.join("; ")) }
}

/// Ein Schritt fuer die Zweitbasis auf Platz k
fn platzSchritt(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, k: usize) {
    let p = PAR(z.spieler);
    let t = welt.Tick();
    let mut s = zb(z.spieler, k);
    if t < s.naechste { return; }
    s.naechste = t + 400;
    match s.phase {
        0 => {
            if let Some(grund) = bereit(welt, z, l) {
                if selten(welt, z, 28, 20000) { melde(welt, z, "zweitbasis", mapCommandCenter, z.bx, z.by, &format!("Zweitbasis {} wartet: {}", k + 1, grund)); }
                setze(z.spieler, k, s); return;
            }
            s.naechste = t + 2000;
            if let Some((nutzen, x, y, bake)) = bakenfeld(welt, z, l) {
                if nutzen >= p.zbNutzen {
                    // Mitte der Zweitbasis: 4 Kacheln neben der Bake, zur eigenen Basis hin (die Mine braucht ihren Platz)
                    let d = (((z.bx - x).pow(2) + (z.by - y).pow(2)) as f64).sqrt().max(1.0);
                    let (mx, my) = ((x as f64 + (z.bx - x) as f64 / d * 5.0) as i32, (y as f64 + (z.by - y) as f64 / d * 5.0) as i32);
                    melde(welt, z, "zweitbasis", mapCommandCenter, mx, my, &format!("Zweitbasis {} bei Bake {},{} (Nutzen {:.2} >= {:.2})", k + 1, x, y, nutzen, p.zbNutzen));
                    if aufzeichnenBei(welt, z, mapCommandCenter, mx, my, "Kommandozentrale") {
                        aufzeichnenBei(welt, z, mapCommonOreSmelter, mx, my, "Schmelze");
                        s.phase = 1; s.x = mx; s.y = my; s.bake = bake; s.seit = t;
                    } else {
                        // kein Platz fuer die Kommandozentrale: dieses Feld eine Weile meiden
                        SPERRE.with(|sp| sp.borrow_mut()[(z.spieler & 7) as usize].push((bake, t + 20000)));
                        melde(welt, z, "zweitbasis", mapCommandCenter, mx, my, &format!("Zweitbasis: kein Platz fuer die Kommandozentrale bei {},{} -- Feld 20000 Ticks gesperrt", mx, my));
                    }
                } else if selten(welt, z, 31, 20000) {
                    melde(welt, z, "zweitbasis", mapNone, x, y, &format!("Zweitbasis lohnt nicht: bestes Bakenfeld {},{} Nutzen {:.2} < {:.2}", x, y, nutzen, p.zbNutzen));
                }
            }
        }
        1 => {
            // warten, bis die Kommandozentrale steht; nach langer Zeit ohne Erfolg wieder pruefen
            let mut cc = false;
            for u in welt.PlayerBuildingEnum(z.spieler, mapCommandCenter) {
                let lo = welt.Location(u); let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
                if (x - s.x).pow(2) + (y - s.y).pow(2) <= 36 && welt.GetCurAction(u) != moObjWait { cc = true; }
            }
            if cc { s.phase = 2; melde(welt, z, "zweitbasis", mapCommandCenter, s.x, s.y, "Kommandozentrale der Zweitbasis steht"); }
            else if t - s.seit > 30000 { s.phase = 0; melde(welt, z, "zweitbasis", mapCommandCenter, s.x, s.y, "Zweitbasis kam nicht zustande -- neu pruefen"); }
        }
        _ => {
            // Tuerme, Fabriken, Mine -- je Schritt hoechstens ein neuer Auftrag
            if (s.tuerme as f64) < p.zbTuerme && l.erz >= 700 {
                if turmBei(welt, z, s.x, s.y) { s.tuerme += 1; setze(z.spieler, k, s); return; }
            }
            if p.zbFabrik > 0.0 && s.fabrik == 0 && kitMoeglich(welt, z, mapVehicleFactory) && l.erz >= welt.GetOreCost(mapVehicleFactory, z.spieler) {
                if aufzeichnenBei(welt, z, mapVehicleFactory, s.x, s.y, "Fahrzeugfabrik") { s.fabrik = 1; setze(z.spieler, k, s); return; }
            }
            if p.zbStruktur > 0.0 && s.struktur == 0 && l.erz >= welt.GetOreCost(mapStructureFactory, z.spieler) {
                if aufzeichnenBei(welt, z, mapStructureFactory, s.x, s.y, "Strukturfabrik") { s.struktur = 1; setze(z.spieler, k, s); return; }
            }
            // Mine: freier Robo-Miner an die Bake, sonst einen in einer Fahrzeugfabrik bauen lassen
            let bl = welt.Location(s.bake);
            if welt.IsLive(s.bake) && !mineAuf(welt, bl) {
                let typ = if welt.GetOreType(s.bake) == 1 { mapRareOreMine } else { mapCommonOreMine };
                let miner = welt.PlayerVehicleEnum(z.spieler).into_iter().find(|&u| welt.GetType(u) == mapRoboMiner && !welt.IsBusy(u) && welt.GetCurAction(u) == moDone);
                if let Some(m) = miner {
                    if t - s.minerSeit > 3000 {
                        welt.DoBuild(m, typ, Loc::new(bl.x + 1, bl.y + 1));
                        s.minerSeit = t;
                        let (bx2, by2) = (edX(welt, z, bl), edY(welt, z, bl));
                        melde(welt, z, "zweitbasis", typ, bx2, by2, &format!("Robo-Miner {} baut die Mine der Zweitbasis", m));
                    }
                } else if t - s.minerSeit > 3000 && l.erz >= welt.GetOreCost(mapRoboMiner, z.spieler) {
                    let vf = l.vfs.iter().copied().take(l.vfAnzahl.max(0) as usize).find(|&v| v != 0 && !welt.IsBusy(v) && welt.HasPower(v) && welt.HasWorkers(v));
                    if let Some(v) = vf {
                        welt.DoProduce(v, mapRoboMiner, mapNone);
                        s.minerSeit = t;
                        melde(welt, z, "zweitbasis", mapRoboMiner, s.x, s.y, "Robo-Miner fuer die Zweitbasis in Auftrag");
                    }
                }
            }
        }
    }
    setze(z.spieler, k, s);
}
