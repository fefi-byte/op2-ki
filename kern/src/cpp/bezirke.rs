//! Bezirke (Missionsmodus, PAR bezirke; Nutzer 2026-11-22: "Die Basis wirkt zu eng. Bau wie Bereiche: kleines
//! Wohngebiet, oder Schmelzen usw."). Statt jedes Gebaeude an den Roehrenstutzen naechst der Mitte zu setzen (ein dichter
//! Klumpen), waechst die Hauptbasis in Bezirken: Industrie zur Erzmine hin, Wohnen vom Feind abgewandt, Forschung und
//! Energie in freien Richtungen, der Kern um die Kommandozentrale. Die Mittelpunkte kommen einmal aus dem Gelaende (genug
//! bebaubare Kacheln, auf kurzem Weg erreichbar); das erste Gebaeude eines Bezirks steht frei an seinem Mittelpunkt (die
//! Roehre folgt ueber anschluss), weitere haengen sich an die Stutzen des Bezirks. Dazwischen bleibt Luft.
//! Districts: the main base grows in industry/housing/research/power districts placed from the terrain.
use std::cell::RefCell;

use super::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Bezirk { Kern = 0, Industrie = 1, Wohnen = 2, Forschung = 3, Energie = 4 }

const NAMEN: [&str; 5] = ["Kern", "Industrie", "Wohnen", "Forschung", "Energie"];

/// Bezirk eines Gebaeudetyps
pub fn bezirk(typ: i32) -> Bezirk {
    if typ == mapCommonOreSmelter || typ == mapRareOreSmelter || typ == mapVehicleFactory || typ == mapGarage || typ == mapCommonStorage
        || typ == mapRareStorage || typ == mapArachnidFactory || typ == mapConsumerFactory { return Bezirk::Industrie; }
    if typ == mapResidence || typ == mapReinforcedResidence || typ == mapAdvancedResidence || typ == mapNursery || typ == mapUniversity
        || typ == mapMedicalCenter || typ == mapRecreationFacility || typ == mapForum || typ == mapDIRT || typ == mapGORF || typ == mapAgridome { return Bezirk::Wohnen; }
    // Labore und Raumhafen in die Mitte (Nutzer 2026-11-22): sie gehoeren zum Kern
    if typ == mapTokamak || typ == mapSolarPowerArray || typ == mapMHDGenerator { return Bezirk::Energie; }
    Bezirk::Kern
}

#[derive(Clone, Copy, Default)]
struct Anker { mitte: (i32, i32), punkte: [Option<(i32, i32)>; 5], gemeldet: bool }

thread_local! {
    static ANKER: RefCell<[Option<Anker>; 8]> = const { RefCell::new([None; 8]) };
    /// Roehrenachse je Bezirk: 0 keine, 1 aufgezeichnet, 2 liegt (der Mittelpunkt ist am Netz); dazu der Tick des Auftrags
    static ACHSE: RefCell<[[u8; 5]; 8]> = const { RefCell::new([[0; 5]; 8]) };
    static ACHSE_SEIT: RefCell<[i32; 8]> = const { RefCell::new([0; 8]) };
    static EW_SEIT: RefCell<[i32; 8]> = const { RefCell::new([-100_000; 8]) };
}
pub fn zuruecksetzen_bezirke() { RAUM_RESERVE.with(|r| *r.borrow_mut() = [None; 8]); FESTUNG.with(|f| *f.borrow_mut() = [false; 8]); ANKER.with(|a| *a.borrow_mut() = [None; 8]); ACHSE.with(|a| *a.borrow_mut() = [[0; 5]; 8]); ACHSE_SEIT.with(|a| *a.borrow_mut() = [0; 8]); EW_SEIT.with(|a| *a.borrow_mut() = [-100_000; 8]); }

/// Eignung eines Mittelpunkts: bebaubare Kacheln im 9x9-Feld, -1 = nicht (auf kurzem Weg) erreichbar
fn eignung(welt: &mut dyn Welt, z: &Zustand, x: i32, y: i32, r: f64) -> i32 {
    if x < 6 || y < 6 || x >= z.breite - 6 || y >= z.hoehe - 6 { return -1; }
    let a = wegAbstand(welt, z, x, y);
    if a < 0 || a as f64 > 2.5 * r + 12.0 { return -1; }
    let mut n = 0;
    for yy in (y - 4)..=(y + 4) { for xx in (x - 4)..=(x + 4) { let c = zelle(welt, z, xx, yy); if zelleBebaubar(welt, c) { n += 1; } } }
    n
}

/// Winkelabstand (0..pi)
fn abw(a: f64, b: f64) -> f64 { let d = (a - b).rem_euclid(std::f64::consts::TAU); d.min(std::f64::consts::TAU - d) }

/// Mittelpunkte der Bezirke (einmal je Basismitte)
fn anker(welt: &mut dyn Welt, z: &mut Zustand) -> Anker {
    let sp = (z.spieler & 7) as usize;
    if let Some(a) = ANKER.with(|a| a.borrow()[sp]) { if a.mitte == (z.bx, z.by) { return a; } }
    let (bx, by) = (z.bx, z.by);
    let (mut ex, mut ey) = (0.0f64, 0.0f64);
    feindRichtung(welt, z, bx, by, &mut ex, &mut ey);
    let feind = if ex != 0.0 || ey != 0.0 { Some(ey.atan2(ex)) } else { None };
    // Erz: naechste eigene Mine, sonst die naechste Bake
    let mut erz: Option<(i32, (i32, i32))> = None;
    for typ in [mapCommonOreMine, mapRareOreMine] { for u in welt.PlayerBuildingEnum(z.spieler, typ) {
        let lo = welt.Location(u); let (x, y) = (edX(welt, z, lo), edY(welt, z, lo)); let d = (x - bx).pow(2) + (y - by).pow(2);
        if erz.is_none_or(|e| d < e.0) { erz = Some((d, (x, y))); }
    } }
    if erz.is_none() { for i in 0..z.kbN.max(0) as usize { let (x, y) = (z.kbX[i] as i32, z.kbY[i] as i32); let d = (x - bx).pow(2) + (y - by).pow(2);
        if erz.is_none_or(|e| d < e.0) { erz = Some((d, (x, y))); } } }
    let erzW = erz.map(|e| ((e.1 .1 - by) as f64).atan2((e.1 .0 - bx) as f64));
    // Kandidaten: 16 Richtungen, Abstand 14 (sonst 16, 12, 18). Gebaut wird im Bezirk erst, wenn seine Roehrenachse liegt
    // (roehrenSchritt) -- ohne das standen bei 14 schon die Startgebaeude lange ohne Strom und Personal (cm03)
    let mut kand: Vec<(f64, (i32, i32), i32)> = Vec::new();
    for k in 0..16 {
        let w = k as f64 * std::f64::consts::PI / 8.0;
        for r in [14.0, 16.0, 12.0, 18.0] {
            let p = ((bx as f64 + w.cos() * r).round() as i32, (by as f64 + w.sin() * r).round() as i32);
            let e = eignung(welt, z, p.0, p.1, r);
            if e >= 45 { kand.push((w, p, e)); break; }
        }
    }
    let mut a = Anker { mitte: (bx, by), punkte: [None; 5], gemeldet: false };
    let mut gewaehlt: Vec<f64> = Vec::new();
    let mut waehle = |bew: &dyn Fn(f64, i32) -> f64, abstand: f64, gewaehlt: &mut Vec<f64>| -> Option<(i32, i32)> {
        let best = kand.iter().filter(|c| gewaehlt.iter().all(|&g| abw(c.0, g) >= abstand))
            .max_by(|x, y| bew(x.0, x.2).total_cmp(&bew(y.0, y.2)))?;
        gewaehlt.push(best.0); Some(best.1)
    };
    let weg = |w: f64| feind.map(|f| abw(w, f) / std::f64::consts::PI).unwrap_or(0.5);   // 1 = vom Feind abgewandt
    a.punkte[Bezirk::Industrie as usize] = waehle(&|w, e| erzW.map(|o| 2.0 * (1.0 - abw(w, o) / std::f64::consts::PI)).unwrap_or(0.0) + e as f64 / 81.0, 0.0, &mut gewaehlt);
    a.punkte[Bezirk::Wohnen as usize] = waehle(&|w, e| 2.0 * weg(w) + e as f64 / 81.0, std::f64::consts::FRAC_PI_2, &mut gewaehlt);
    a.punkte[Bezirk::Energie as usize] = waehle(&|w, e| weg(w) + e as f64 / 81.0, 1.17, &mut gewaehlt);
    // Energie etwas weiter hinaus (Tokamaks explodieren mit Flaechenschaden)
    if let Some(p) = a.punkte[Bezirk::Energie as usize] {
        let (dx, dy) = ((p.0 - bx) as f64, (p.1 - by) as f64); let l = (dx * dx + dy * dy).sqrt().max(1.0);
        let q = ((p.0 as f64 + dx / l * 2.0).round() as i32, (p.1 as f64 + dy / l * 2.0).round() as i32);
        if eignung(welt, z, q.0, q.1, l + 2.0) >= 45 { a.punkte[Bezirk::Energie as usize] = Some(q); }
    }
    ANKER.with(|x| x.borrow_mut()[sp] = Some(a));
    raumhafenReservieren(welt, z, &a);
    a
}

/// Mittelpunkte der Bezirke (fuer das Gebiet der Basis; leer ohne bezirke)
pub fn ankerPunkte(z: &Zustand) -> Vec<(i32, i32)> {
    ANKER.with(|a| a.borrow()[(z.spieler & 7) as usize].map(|a| a.punkte.iter().flatten().copied().collect()).unwrap_or_default())
}

/// Bauplatz im Bezirk des Typs (Index in z.plaetze), None = wie bisher (Kern, kein Bezirk, nichts frei)
pub fn platz(welt: &mut dyn Welt, z: &mut Zustand, typ: i32) -> Option<i32> {
    let b = bezirk(typ);
    if b == Bezirk::Kern { return None; }
    let mut a = anker(welt, z);
    if !a.gemeldet && welt.Tick() > 0 {
        a.gemeldet = true; ANKER.with(|x| x.borrow_mut()[(z.spieler & 7) as usize] = Some(a));
        let txt: Vec<String> = (1..5).map(|i| match a.punkte[i] { Some(p) => format!("{} {},{}", NAMEN[i], p.0, p.1), None => format!("{} am Kern", NAMEN[i]) }).collect();
        melde(welt, z, "plan", mapCommandCenter, z.bx, z.by, &format!("Bezirke der Basis: {}", txt.join(", ")));
    }
    let (ax, ay) = a.punkte[b as usize]?;
    // Roehren zuerst (Nutzer 2026-11-22): solange die Achse zum Bezirk nicht liegt, baut der Kern
    if ACHSE.with(|x| x.borrow()[(z.spieler & 7) as usize][b as usize]) != 2 { return None; }
    let g = grundflaeche(welt, typ);
    let mut hin = [Rechteck::default(); 160];
    let nh = hindernisse(welt, z, &mut hin, 160);
    // an die Stutzen, gemessen vom Mittelpunkt des Bezirks -- nur wenn der Platz im Bezirk liegt
    let (bx, by) = (z.bx, z.by);
    z.bx = ax; z.by = ay;
    let (gef, x, y, _, _) = platzAmStutzen(welt, z, typ, &hin, nh);
    z.bx = bx; z.by = by;
    if gef && (x + g.w / 2 - ax).pow(2) + (y + g.h / 2 - ay).pow(2) <= 49 {
        let i = platzEintrag(welt, z, typ, x, y);
        if i >= 0 { return Some(i); }
    }
    // sonst frei nahe dem Mittelpunkt (erstes Gebaeude des Bezirks); die Roehre legt anschluss
    // erst nahe, dann weiter hinaus (der Bezirk waechst; cm03: ab ~78 Gebaeuden fand das Tokamak sonst keinen Platz mehr, und
    // dahinter wartete der ganze Ausbau)
    let mut best: Option<(i32, i32, i32)> = None;
    for rr in [7, 10, 13] {
    if best.is_some() { break; }
    for dy in -rr..=rr { for dx in -rr..=rr {
        let d2 = dx * dx + dy * dy;
        if d2 > rr * rr || best.is_some_and(|bb| d2 >= bb.0) { continue; }
        let (x0, y0) = (ax + dx - g.w / 2, ay + dy - g.h / 2);
        let mut gr: &'static str = "";
        if !platzFrei(welt, z, x0, y0, g.w, g.h, &hin, nh, &mut gr) { continue; }
        let w = d2 + explosionsStrafe(welt, z, typ, x0, y0, g.w, g.h);
        if best.is_none_or(|bb| w < bb.0) { best = Some((w, x0, y0)); }
    } }
    }
    let (_, x0, y0) = best?;
    let i = platzEintrag(welt, z, typ, x0, y0);
    if i < 0 { return None; }
    melde(welt, z, "plan", typ, x0, y0, &format!("Bezirk {}: Platz {},{} nahe dem Mittelpunkt {},{} (Roehre folgt)", NAMEN[b as usize], x0, y0, ax, ay));
    Some(i)
}

/// Roehren zuerst (Nutzer 2026-11-22): nach dem Start je Bezirk eine Roehrenachse vom Netz bis zu seinem Mittelpunkt, eine
/// nach der anderen (Wohnen, Industrie, Energie, Forschung), jeweils erst wenn keine andere Roehre offen ist. Gebaut wird im
/// Bezirk erst, wenn seine Achse liegt; bis dahin waechst der Kern.
pub fn roehrenSchritt(welt: &mut dyn Welt, z: &mut Zustand) {
    if PAR(z.spieler).bezirke < 0.5 || z.bauAn == 0 { return; }
    let t = welt.Tick();
    if t < 3000 || t % 200 >= TAKT { return; }
    let sp = (z.spieler & 7) as usize;
    let a = anker(welt, z);
    // eine Roehre offen: abwarten. Wartet eine schon lange (der eine Earthworker legt Achsen, Anschluesse und Mauern),
    // einen zweiten bauen lassen (hoechstens zwei, einer je 10000 Ticks)
    let aelteste = (0..z.anzahlRoehren.max(0) as usize).filter(|&i| z.roehren[i].zustand == 1).map(|i| t - z.roehren[i].seit).max();
    if let Some(alt) = aelteste {
        let ew = welt.PlayerVehicleEnum(z.spieler).into_iter().filter(|&u| welt.GetType(u) == mapEarthworker && !super::zweitbasis::gehoertBasis(z, u) && !super::zweitbasis::reserviert(welt, z, u)).count();
        if alt > 6000 && ew < 2 && t - EW_SEIT.with(|e| e.borrow()[sp]) > 10000 && welt.Ore(z.spieler) >= welt.GetOreCost(mapEarthworker, z.spieler) {
            let vf = welt.PlayerBuildingEnum(z.spieler, mapVehicleFactory).into_iter().find(|&v| !super::zweitbasis::gehoertBasis(z, v) && !welt.IsBusy(v) && welt.HasPower(v) && welt.HasWorkers(v));
            if let Some(v) = vf {
                welt.DoProduce(v, mapEarthworker, mapNone); EW_SEIT.with(|e| e.borrow_mut()[sp] = t);
                melde(welt, z, "bau", mapEarthworker, z.bx, z.by, &format!("zweiter Earthworker in Auftrag: Roehren warten seit {} Ticks", alt));
            }
        }
        return;
    }
    let netz = netzKacheln(welt, z);
    let bb = z.breite.min(256);
    let imNetz = |x: i32, y: i32| x >= 0 && y >= 0 && x < bb && y < z.hoehe.min(256) && netz[(y * bb + x) as usize];
    for b in [Bezirk::Wohnen, Bezirk::Industrie, Bezirk::Energie, Bezirk::Forschung] {
        let Some((ax, ay)) = a.punkte[b as usize] else { continue; };
        let st = ACHSE.with(|x| x.borrow()[sp][b as usize]);
        if st == 2 { continue; }
        if imNetz(ax, ay) || (-1..=1).any(|dy| (-1..=1).any(|dx| { let c = zelle(welt, z, ax + dx, ay + dy); zelleRoehre(welt, c) && imNetz(ax + dx, ay + dy) })) {
            ACHSE.with(|x| x.borrow_mut()[sp][b as usize] = 2);
            melde(welt, z, "plan", mapTube, ax, ay, &format!("Bezirk {}: Roehrenachse liegt nach {} Ticks -- ab jetzt wird dort gebaut", NAMEN[b as usize], t - ACHSE_SEIT.with(|x| x.borrow()[sp])));
            continue;
        }
        // Weg vom Netz zum Mittelpunkt (4er-Nachbarschaft wie die Roehren, ueber bebaubare Kacheln und vorhandene Roehren)
        let mut hin = [Rechteck::default(); 160];
        let nh = hindernisse(welt, z, &mut hin, 160);
        let hh = z.hoehe.min(256);
        let docks = dockKacheln(welt, z);
        let frei = |welt: &mut dyn Welt, x: i32, y: i32| -> bool {
            if docks.contains(&(x, y)) { return false; }
            if x < 1 || y < 1 || x >= bb - 1 || y >= hh - 1 { return false; }
            let c = zelle(welt, z, x, y);
            (zelleBebaubar(welt, c) || zelleRoehre(welt, c)) && !hin[..nh as usize].iter().any(|r| x >= r.x0 && x < r.x0 + r.w && y >= r.y0 && y < r.y0 + r.h)
                && !super::mauer::geplantIn(z, x, y, 1, 1)
        };
        let mut vor = vec![-2i32; (bb * hh) as usize];
        let mut q = std::collections::VecDeque::new();
        for y in 1..hh - 1 { for x in 1..bb - 1 {
            if !imNetz(x, y) { continue; }
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                let i = (ny * bb + nx) as usize;
                if vor[i] == -2 && !imNetz(nx, ny) && frei(welt, nx, ny) { vor[i] = -1; q.push_back((nx, ny)); }
            }
        } }
        let mut ziel: Option<(i32, i32)> = None;
        while let Some((x, y)) = q.pop_front() {
            if (x - ax).abs() <= 1 && (y - ay).abs() <= 1 { ziel = Some((x, y)); break; }
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 1 || ny < 1 || nx >= bb - 1 || ny >= hh - 1 { continue; }
                let i = (ny * bb + nx) as usize;
                if vor[i] != -2 || imNetz(nx, ny) || !frei(welt, nx, ny) { continue; }
                vor[i] = y * bb + x; q.push_back((nx, ny));
            }
        }
        let Some((mut cx, mut cy)) = ziel else {
            if st == 0 { melde(welt, z, "fehler", mapTube, ax, ay, &format!("Bezirk {}: kein Weg fuer die Roehrenachse -- der Bezirk entfaellt", NAMEN[b as usize])); }
            ACHSE.with(|x| x.borrow_mut()[sp][b as usize] = 2);   // nicht blockieren: dann eben ohne Achse
            continue;
        };
        let mut weg: Vec<(i32, i32)> = Vec::new();
        loop { weg.push((cx, cy)); let v = vor[(cy * bb + cx) as usize]; if v < 0 || weg.len() > 60 { break; } cx = v % bb; cy = v / bb; }
        let mut neu = 0;
        for &(x, y) in weg.iter().rev() {
            let c = zelle(welt, z, x, y);
            if zelleRoehre(welt, c) { continue; }
            let ort = engine(welt, z, x, y);
            welt.RecordTube(super::zweitbasis::gruppeAm(z, x, y), ort);
            if (z.anzahlRoehren as usize) < MAX_ROEHREN {
                let i = z.anzahlRoehren as usize; z.anzahlRoehren += 1;
                let r = &mut z.roehren[i];
                r.x1 = x as i16; r.x2 = x as i16; r.y1 = y as i16; r.y2 = y as i16; r.zustand = 1; r.versuche = 0; r.seit = t;
            }
            neu += 1;
        }
        ACHSE.with(|x| x.borrow_mut()[sp][b as usize] = 1);
        ACHSE_SEIT.with(|x| x.borrow_mut()[sp] = t);
        melde(welt, z, "plan", mapTube, ax, ay, &format!("Bezirk {}: Roehrenachse {} Kacheln ({} neu) vom Netz zum Mittelpunkt {},{} aufgezeichnet", NAMEN[b as usize], weg.len(), neu, ax, ay));
        return;
    }
}

/// Luftiger Kern (Nutzer 2026-11-22: "Die Startbasis darf auch schon ein paar Roehren haben, damit dort alles luftiger
/// ist"): Platz um die Mitte (z.bx, z.by) mit mindestens 2 Kacheln Luft zu jedem Gebaeude -- die Luecke ueberbrueckt eine
/// kurze Roehre (anschluss), kurze Roehren bevorzugt; die Mittelpunkte der Bezirke bleiben frei. Auch fuer die zweite Basis
/// (dort ist z.bx, z.by ihre Mitte). None = nichts gefunden (dann wie bisher am Stutzen).
pub fn kernPlatz(welt: &mut dyn Welt, z: &mut Zustand, typ: i32) -> Option<i32> {
    if typ == mapCommandCenter { return None; }
    // nur mit Erz fuer die Roehren: knapp gebaut stand die Fahrzeugfabrik ohne Roehre und damit ohne Personal (cm03)
    if welt.Ore(z.spieler) < 1500 { return None; }
    let g = grundflaeche(welt, typ);
    let mut hin = [Rechteck::default(); 160];
    let nh = hindernisse(welt, z, &mut hin, 160);
    let abst = netzAbstand(welt, z, &hin, nh);
    if !abst.iter().any(|&v| v == 0) { return None; }   // noch kein Netz (Kommandozentrale fehlt)
    let anker = ankerPunkte(z);
    let bb = z.breite.min(256);
    let (mx, my) = (z.bx, z.by);
    // in Stufen hinaus: 18 Kacheln mit kurzer Roehre, dann 24 und 30 mit laengerer (cm03: bei ~80 Gebaeuden fanden Wohnhaus
    // und Agrarkuppel keinen Platz mehr -- Hungersnot)
    let mut best: Option<(i32, i32, i32)> = None;
    for (rr, maxRoehre) in [(18, 3), (24, 6), (30, 10)] {
    if best.is_some() { break; }
    for dy in -rr..=rr { for dx in -rr..=rr {
        let d2 = dx * dx + dy * dy;
        if d2 > rr * rr || best.is_some_and(|bb2| d2 >= bb2.0) { continue; }
        let (x0, y0) = (mx + dx - g.w / 2, my + dy - g.h / 2);
        let (cx, cy) = (x0 + g.w / 2, y0 + g.h / 2);
        if anker.iter().any(|a| (a.0 - cx).pow(2) + (a.1 - cy).pow(2) < 36) { continue; }
        // 2 Kacheln Luft
        if hin[..nh as usize].iter().any(|r| ueberlappt(welt, x0 - 2, y0 - 2, g.w + 4, g.h + 4, r.x0, r.y0, r.w, r.h)) { continue; }
        // Roehre: kuerzester Anschluss ueber eine Seitenkachel
        let rr = Rechteck { x0, y0, w: g.w, h: g.h, stutzen: true };
        let mut roehre = i32::MAX;
        for y in (y0 - 1)..=(y0 + g.h) { for x in (x0 - 1)..=(x0 + g.w) {
            if x < 0 || y < 0 || x >= bb || (y * bb + x) as usize >= abst.len() || !anGrundflaeche(welt, &rr, x, y) { continue; }
            let v = abst[(y * bb + x) as usize]; if v >= 0 && v < roehre { roehre = v; }
        } }
        if roehre == i32::MAX || roehre > maxRoehre { continue; }
        let w = d2 + 6 * roehre;
        if best.is_some_and(|bb2| w >= bb2.0) { continue; }
        let mut gr: &'static str = "";
        if !platzFrei(welt, z, x0, y0, g.w, g.h, &hin, nh, &mut gr) { continue; }
        let w = w + explosionsStrafe(welt, z, typ, x0, y0, g.w, g.h);
        if best.is_none_or(|bb2| w < bb2.0) { best = Some((w, x0, y0)); }
    } }
    }
    let (_, x0, y0) = best?;
    let i = platzEintrag(welt, z, typ, x0, y0);
    if i < 0 { None } else { Some(i) }
}

/// Raumhafen in die Mitte (Missionsmodus, Nutzer 2026-11-22): naechster Platz an der Kommandozentrale, um den ringsum 3 Kacheln
/// frei sind -- dort entsteht die Festung: Mauer 2 Kacheln um den Raumhafen an drei Seiten, die Seite zur Kommandozentrale offen
/// (Roehre, Lastwagen, ConVecs), Tuerme an den vier Ecken.
pub fn raumhafenPlatz(welt: &mut dyn Welt, z: &mut Zustand, typ: i32) -> Option<i32> {
    let g = grundflaeche(welt, typ);
    let sp = (z.spieler & 7) as usize;
    // reservierter Platz: die Reserve aufheben (sonst sperrt sie sich selbst), dann pruefen
    let reserve = RAUM_RESERVE.with(|r| r.borrow_mut()[sp].take());
    let mut hin = [Rechteck::default(); 160];
    let nh = hindernisse(welt, z, &mut hin, 160);
    let (mx, my) = (z.bx, z.by);
    let mut best: Option<(i32, i32, i32)> = None;
    if let Some((x0, y0)) = reserve {
        let mut gr: &'static str = "";
        if platzFrei(welt, z, x0, y0, g.w, g.h, &hin, nh, &mut gr) { best = Some((0, x0, y0)); }
        else {
            // gerade belegt (Fahrzeug): warten, nicht woanders bauen -- eigene Fahrzeuge darauf fahren weg (von der Zentrale fort)
            RAUM_RESERVE.with(|r| r.borrow_mut()[sp] = Some((x0, y0)));
            if welt.Tick() % 400 < TAKT {
                let (cx, cy) = (x0 + g.w / 2, y0 + g.h / 2);
                let (vx, vy) = ((cx - mx).signum(), (cy - my).signum());
                let ziel = (cx + vx * (g.w + 8), cy + vy * (g.h + 8));
                for u in welt.PlayerVehicleEnum(z.spieler) {
                    let lo = welt.Location(u); let (ux, uy) = (edX(welt, z, lo), edY(welt, z, lo));
                    if ux >= x0 - 3 && ux < x0 + g.w + 3 && uy >= y0 - 3 && uy < y0 + g.h + 3 && welt.GetCurAction(u) == moDone {
                        let l = engine(welt, z, ziel.0, ziel.1); welt.DoMove(u, l);
                    }
                }
            }
            return Some(-1);
        }
    }
    for dy in -16..=16 { for dx in -16..=16 {
        let d2 = dx * dx + dy * dy;
        if d2 < 16 || d2 > 256 || best.is_some_and(|b| d2 >= b.0 || b.0 == 0) { continue; }
        let (x0, y0) = (mx + dx - g.w / 2, my + dy - g.h / 2);
        let mut gr: &'static str = "";
        // Platz samt 3 Kacheln Rand frei (dort stehen Mauer und Tuerme)
        if !platzFrei(welt, z, x0 - 3, y0 - 3, g.w + 6, g.h + 6, &hin, nh, &mut gr) { continue; }
        best = Some((d2, x0, y0));
    } }
    let (_, x0, y0) = best?;
    let i = platzEintrag(welt, z, typ, x0, y0);
    if i < 0 { return None; }
    // offene Seite: zur Kommandozentrale
    let (cx, cy) = (x0 + g.w / 2, y0 + g.h / 2);
    let (vx, vy) = (mx - cx, my - cy);
    let offen = if vx.abs() >= vy.abs() { if vx > 0 { 0 } else { 1 } } else if vy > 0 { 2 } else { 3 };   // 0 rechts, 1 links, 2 unten, 3 oben
    let (ax, ay, bx2, by2) = (x0 - 2, y0 - 2, x0 + g.w + 1, y0 + g.h + 1);
    let ecken = [(ax, ay), (bx2, ay), (ax, by2), (bx2, by2)];
    let mut mauer = Vec::new();
    for x in ax..=bx2 { for y in [ay, by2] { mauer.push((x, y)); } }
    for y in (ay + 1)..by2 { for x in [ax, bx2] { mauer.push((x, y)); } }
    mauer.retain(|&(x, y)| !ecken.contains(&(x, y)) && match offen { 0 => x != bx2, 1 => x != ax, 2 => y != by2, _ => y != ay });
    melde(welt, z, "plan", typ, x0, y0, &format!("Raumhafen in der Mitte: Platz {},{} ({} Kacheln von der Kommandozentrale), offene Seite zur Kommandozentrale", x0, y0, ((cx - mx).pow(2) + (cy - my).pow(2)) as f64));
    FESTUNG.with(|f| f.borrow_mut()[(z.spieler & 7) as usize] = true);
    super::mauer::festungEintragen(welt, z, (cx, cy), mauer, ecken.to_vec());
    Some(i)
}

thread_local! {
    static FESTUNG: RefCell<[bool; 8]> = const { RefCell::new([false; 8]) };
    static FESTUNG_SEIT: RefCell<[i32; 8]> = const { RefCell::new([0; 8]) };
    /// reservierter Platz des Raumhafens (oben links) samt Festungsrand, bis er gebaut wird
    static RAUM_RESERVE: RefCell<[Option<(i32, i32)>; 8]> = const { RefCell::new([None; 8]) };
}

/// Reserviertes Rechteck des Raumhafens samt 3 Kacheln Rand (fuer hindernisse: dort baut nichts anderes)
pub fn raumhafenReserve(welt: &mut dyn Welt, z: &Zustand) -> Option<Rechteck> {
    let (x0, y0) = RAUM_RESERVE.with(|r| r.borrow()[(z.spieler & 7) as usize])?;
    let g = grundflaeche(welt, mapSpaceport);
    Some(Rechteck { x0: x0 - 3, y0: y0 - 3, w: g.w + 6, h: g.h + 6, stutzen: false })
}

/// Platz fuer Raumhafen und Festung frueh reservieren (beim Festlegen der Bezirke): naechste Flaeche an der Kommandozentrale
/// (5..12 Kacheln), auf der Raumhafen und 3 Kacheln Rand bebaubar und frei sind -- spaeter war die Mitte zugebaut
fn raumhafenReservieren(welt: &mut dyn Welt, z: &mut Zustand, a: &Anker) {
    if PAR(z.spieler).raumhafen < 0.5 || !super::modus::mission(z) { return; }
    let g = grundflaeche(welt, mapSpaceport);
    let mut hin = [Rechteck::default(); 160];
    let nh = hindernisse(welt, z, &mut hin, 160);
    let (mx, my) = (z.bx, z.by);
    let mut best: Option<(i32, i32, i32)> = None;
    for dy in -12..=12 { for dx in -12..=12 {
        let d2 = dx * dx + dy * dy;
        if !(25..=144).contains(&d2) || best.is_some_and(|b| d2 >= b.0) { continue; }
        let (x0, y0) = (mx + dx - g.w / 2, my + dy - g.h / 2);
        let (cx, cy) = (x0 + g.w / 2, y0 + g.h / 2);
        if a.punkte.iter().flatten().any(|p| (p.0 - cx).pow(2) + (p.1 - cy).pow(2) < 100) { continue; }
        let mut gr: &'static str = "";
        if !platzFrei(welt, z, x0 - 3, y0 - 3, g.w + 6, g.h + 6, &hin, nh, &mut gr) { continue; }
        best = Some((d2, x0, y0));
    } }
    if let Some((_, x0, y0)) = best {
        RAUM_RESERVE.with(|r| r.borrow_mut()[(z.spieler & 7) as usize] = Some((x0, y0)));
        melde(welt, z, "plan", mapSpaceport, x0, y0, &format!("Platz fuer den Raumhafen der Mission reserviert bei {},{} (samt Festungsrand)", x0, y0));
    }
}

/// Missionsmodus: fehlende Festungs- und Zugangstuerme anstossen (sonst baute die KI Tuerme nur nach Bedrohung) -- einer je
/// 4000 Ticks, wenn kein anderer Bau der Hauptbasis offen ist
pub fn festungSchritt(welt: &mut dyn Welt, z: &mut Zustand) {
    let sp = (z.spieler & 7) as usize;
    let t = welt.Tick();
    if !FESTUNG.with(|f| f.borrow()[sp]) || t - FESTUNG_SEIT.with(|f| f.borrow()[sp]) < 4000 || z.bauAn == 0 { return; }
    if welt.Ore(z.spieler) < 800 || !super::mauer::zugangTurmFehlt(welt, z) { return; }
    let offen = (0..z.anzahlPlaetze.max(0) as usize).any(|i| { let q = z.plaetze[i]; (q.zustand == P_FAHRT || q.zustand == P_BAUT) && super::zweitbasis::gruppeAm(z, q.x0 as i32, q.y0 as i32) == z.bau });
    if offen { return; }
    FESTUNG_SEIT.with(|f| f.borrow_mut()[sp] = t);
    bauAufzeichnen(welt, z, mapGuardPost, "Turm der Raumhafen-Festung bzw. eines Zugangs");
}
