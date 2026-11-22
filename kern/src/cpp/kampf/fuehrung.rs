//! Kampffuehrung je SCAN_TAKT, aufgeteilt nach Schritten (Rust 2026-11-18; vorher eine Funktion kampfFuehrungInnen mit
//! 560 Zeilen, Verhalten unveraendert -- geprueft mit ki-test/gleich.sh). Reihenfolge der Schritte wie in der C++-KI.
use super::*;
use super::super::*;
use super::{GRUPPE_MAX, SCAN_TAKT, WERT_WT, WERT_LYNX, EMP_RADIUS, EMP_FLUG, EMP_AUFMARSCH, GEBAEUDE};

/// Warteplatz der Wache (zum Feind hin, mit gelaende hinter dem Pass), Sammelplatz (hinter der Kolonie), Basisrechteck
#[derive(Clone, Copy)]
pub(super) struct Plaetze { pub wx: i32, pub wy: i32, pub sx: i32, pub sy: i32, pub basis: Rect }
/// Sollzahlen: Fahrgestell, Waffe, Wache und Sammelgruppe (Fahrzeuge)
#[derive(Clone, Copy)]
pub(super) struct Soll { pub fz: i32, pub waffe: i32, pub wacheZiel: i32, pub sammelZiel: i32 }
/// Bestand nach dem Einsammeln: Fahrzeuge in Wache (nw), Sammelgruppe (ns), Angriff (na) und Angriffswert (wa)
#[derive(Clone, Copy)]
pub(super) struct Bestand { pub nw: i32, pub ns: i32, pub na: i32, pub wa: i32 }

pub(super) fn kampfFuehrungInnen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32) {
    let pl = plaetzeBestimmen(welt, z);
    gruppenAnlegen(welt, z, &pl);
    fabrikGruppe(welt, z, l);
    let mut soll = sollzahlen(welt, z, l);
    // Missionen (Nutzer 2026-11-18): keine Wache, keine Reserve -- alles, was gebaut wird, kommt an die Front
    let missionen = super::super::missionen::an(z);
    if missionen { soll.sammelZiel = (soll.sammelZiel + soll.wacheZiel).max(4).min(GRUPPE_MAX); soll.wacheZiel = 0; }
    beimischungSetzen(welt, z, l, &soll);
    if missionen {
        if z.reserveZiel != 0 { let g = z.eingreif; welt.ClearTargCount(g); z.reserveZiel = 0; }
        let _ = streunerAufnehmen(welt, z, t, &pl, &soll);
        super::super::missionen::fuehren(welt, z, l, t);
        return;
    }
    hinweis(welt, z, "ki::kampfFuehrung/reserve"); reserveFuehren(welt, z);
    aussenpostenVerteilen(welt, z, t, &pl);
    let mut b = streunerAufnehmen(welt, z, t, &pl, &soll);
    hinweis(welt, z, "ki::kampfFuehrung/geleit"); geleitSchutz(welt, z, t, &pl);
    // 3. Angriff
    hinweis(welt, z, "ki::kampfFuehrung/angriff");
    let (feindWTWert, feindWTBasis) = feindTuerme(welt, z);
    hinweis(welt, z, "ki::kampfFuehrung/verfolgen"); verfolgen(welt, z);
    hinweis(welt, z, "ki::kampfFuehrung/spinnen"); spinnenEinsatz(welt, z);
    if z.kampfModus >= 2 {
        hinweis(welt, z, "ki::kampfFuehrung/kontakt"); bilanzKontakt(welt, z);
        hinweis(welt, z, "ki::kampfFuehrung/rueckzug"); let wg = z.wache; rueckzugPruefen(welt, z, wg);
        hinweis(welt, z, "ki::kampfFuehrung/angriff");
    }
    if z.kampfModus == 3 || z.kampfModus == 4 { aufmarschUndRakete(welt, z, t, &mut b); return; }
    if z.kampfModus == 2 { angriffLaeuft(welt, z, &pl, &mut b); return; }
    super::zermuerben::neuerAngriff(welt, z, l, t, &soll, &mut b, feindWTWert, feindWTBasis);
}

/// Platz fuer eine ganze Wache: mindestens 85 % befahrbar im 11x11-Feld um (x, y). Nutzer 2026-11-22 (Bild): die Wache stand
/// in der schmalen Felsluecke suedlich der Basis, verstopfte sie, die Lastwagen steckten fest, das Erz blieb aus.
fn weitesFeld(welt: &mut dyn Welt, z: &Zustand, x: i32, y: i32) -> bool {
    // nicht auf dem reservierten Raumhafen-Platz warten (Nutzer-Spiel cm03 2026-11-22: die Armee stand dort, der Raumhafen wartete
    // auf den freien Platz, und mit ihm die ganze Bauliste)
    if let Some(r) = super::super::bezirke::raumhafenReserve(welt, z) {
        if x + 5 >= r.x0 && x - 5 < r.x0 + r.w && y + 5 >= r.y0 && y - 5 < r.y0 + r.h { return false; }
    }
    let mut n = 0;
    for yy in (y - 5)..=(y + 5) { for xx in (x - 5)..=(x + 5) {
        if xx < 1 || yy < 1 || xx >= z.breite - 1 || yy >= z.hoehe - 1 { continue; }
        let c = zelle(welt, z, xx, yy); if zelleBefahrbar(welt, c) { n += 1; }
    } }
    n * 100 >= 121 * 85
}

thread_local! {
    /// PAR wachePatrouille: aktueller Patrouillenpunkt (1-basiert) und Zahl der Punkte je Spieler
    static PATROUILLE: std::cell::RefCell<[Option<(usize, usize)>; 8]> = const { std::cell::RefCell::new([None; 8]) };
}

/// Plaetze von Wache und Sammelgruppe
fn plaetzeBestimmen(welt: &mut dyn Welt, z: &mut Zustand) -> Plaetze {
    hinweis(welt, z, "ki::kampfFuehrung/gruppen");
    let (mut eux, mut euy) = (0.0f64, 0.0f64);
    feindRichtung(welt, z, z.bx, z.by, &mut eux, &mut euy);
    let (mut wx, mut wy) = ((z.bx as f64 + eux * 8.0 + 0.5) as i32, (z.by as f64 + euy * 8.0 + 0.5) as i32);   // Warteplatz der Wache zum Feind hin
    // Erweiterung Gelaende (Rust): Wache hinter dem Pass
    let mut pass = false;
    if PAR(z.spieler).gelaende > 0.0 { if let Some((x, y)) = super::erweiterung::wachePunkt(welt, z) { wx = x; wy = y; pass = true; } }
    let (mut sx, mut sy) = ((z.bx as f64 - eux * 6.0 + 0.5) as i32, (z.by as f64 - euy * 6.0 + 0.5) as i32);   // Sammelplatz hinter der Kolonie
    // PAR wacheVorBasis (Missionsmodus, Nutzer: Armee vor der Basis statt darin): auf offenem Feld lagen Warte- und
    // Sammelplatz in der gewachsenen Basis, 30 Lynx versperrten Dock, Lastwagen und ConVecs (cm03). Beide Plaetze vor den
    // bebauten Umkreis (weiteste Gebaeude bis wacheMax), die Wache zum Feind hin, die Sammelgruppe dahinter.
    if PAR(z.spieler).wacheVorBasis >= 0.5 {
        let weit = PAR(z.spieler).wacheMax as i32;
        let mut r2 = 0;
        for &typ in super::GEBAEUDE.iter() { for u in welt.PlayerBuildingEnum(z.spieler, typ) {
            if !welt.IsLive(u) || typ == mapGuardPost { continue; }
            let lo = welt.Location(u); let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
            let d2 = abstand2(welt, x, y, z.bx, z.by); if d2 <= weit * weit && d2 > r2 { r2 = d2; }
        } }
        let r = (r2 as f64).sqrt();
        if !pass {
            // vor die Bebauung zum Feind hin -- auf freies Feld (in einer Felsluecke verstopfte die Wache den Durchgang)
            let mut gefunden = false;
            for extra in [6.0, 9.0, 4.0, 12.0] {
                let d = (r + extra).max(8.0);
                let (x, y) = ((z.bx as f64 + eux * d + 0.5) as i32, (z.by as f64 + euy * d + 0.5) as i32);
                if weitesFeld(welt, z, x, y) { wx = x; wy = y; gefunden = true; break; }
            }
            if !gefunden { let d = (r + 6.0).max(8.0); wx = (z.bx as f64 + eux * d + 0.5) as i32; wy = (z.by as f64 + euy * d + 0.5) as i32; }
        }
        let d = (r + 5.0).max(6.0); sx = (z.bx as f64 - eux * d + 0.5) as i32; sy = (z.by as f64 - euy * d + 0.5) as i32;
        // auch der Sammelplatz auf freies Feld (weiter hinaus oder seitlich versetzt)
        if !weitesFeld(welt, z, sx, sy) {
            'such: for extra in [3.0, 6.0, -2.0] { for seit in [0.0, 6.0, -6.0] {
                let dd = d + extra;
                let (x, y) = ((z.bx as f64 - eux * dd - euy * seit + 0.5) as i32, (z.by as f64 - euy * dd + eux * seit + 0.5) as i32);
                if weitesFeld(welt, z, x, y) { sx = x; sy = y; break 'such; }
            } }
        }
        let (b, h) = (z.breite, z.hoehe);
        // PAR wachePatrouille (Missionsmodus, Nutzer 2026-11-22: "lass sie um die Basis patrouillieren"): ohne Angriff
        // wandert der Warteplatz alle wachePatrouilleTakt Ticks zum naechsten Punkt eines Rings um die Basis (8 Richtungen im
        // Abstand des Warteplatzes, nur befahrbare und auf kurzem Weg erreichbare Punkte, beginnend zum Feind hin). Die Wache
        // bewacht dabei weiter das ganze Wachgebiet; bei einem Angriff bleibt sie am Punkt zum Feind (bzw. hinter dem Pass).
        let pm = PAR(z.spieler);
        let t = welt.Tick();
        let angriff = z.letzterAngriff != 0 && t - z.letzterAngriff < 4000;
        if pm.wachePatrouille >= 0.5 && !angriff {
            let d = (r + 6.0).max(8.0);
            let grenze = (2.5 * d) as i32 + 12;
            let start = euy.atan2(eux);
            // Erzwege (Mine -> naechste Schmelze): dort parkt die Wache nicht, sie versperrte sonst die Lastwagen (cm03: das
            // Erz stand 18 000 Ticks still, zwei Punkte lagen an der Mine)
            let orte = |welt: &mut dyn Welt, typen: &[i32]| -> Vec<(f64, f64)> { let mut v = Vec::new();
                for &ty in typen { for u in welt.PlayerBuildingEnum(z.spieler, ty) { let lo = welt.Location(u); v.push((edX(welt, z, lo) as f64, edY(welt, z, lo) as f64)); } } v };
            let minen = orte(welt, &[mapCommonOreMine, mapRareOreMine]);
            let schmelzen = orte(welt, &[mapCommonOreSmelter, mapRareOreSmelter]);
            let wege: Vec<((f64, f64), (f64, f64))> = minen.iter().filter_map(|&m| schmelzen.iter().copied()
                .min_by(|a, c| ((a.0 - m.0).powi(2) + (a.1 - m.1).powi(2)).total_cmp(&((c.0 - m.0).powi(2) + (c.1 - m.1).powi(2)))).map(|s| (m, s))).collect();
            let amWeg = |px: f64, py: f64| wege.iter().any(|&((ax, ay), (bx2, by2))| {
                let (dx, dy) = (bx2 - ax, by2 - ay); let l2 = (dx * dx + dy * dy).max(1.0);
                let u = (((px - ax) * dx + (py - ay) * dy) / l2).clamp(0.0, 1.0);
                (px - ax - u * dx).powi(2) + (py - ay - u * dy).powi(2) < 36.0 });
            let mut punkte: Vec<(i32, i32)> = Vec::new();
            for k in 0..8 {
                let w = start + k as f64 * std::f64::consts::FRAC_PI_4;
                // nicht erreichbar (Fels, Wasser, hinter einer Wand): etwas naeher oder weiter versuchen
                for dd in [d, d - 3.0, d + 3.0, d - 6.0] {
                    let (px, py) = (((z.bx as f64 + w.cos() * dd + 0.5) as i32).clamp(3, b - 4), ((z.by as f64 + w.sin() * dd + 0.5) as i32).clamp(3, h - 4));
                    let a = super::super::hilfen::wegAbstand(welt, z, px, py);
                    if a >= 0 && a <= grenze && !amWeg(px as f64, py as f64) && weitesFeld(welt, z, px, py) { punkte.push((px, py)); break; }
                }
            }
            if punkte.len() >= 3 {
                let takt = pm.wachePatrouilleTakt.max(500.0) as i32;
                let i = ((t / takt) as usize) % punkte.len();
                wx = punkte[i].0; wy = punkte[i].1;
                PATROUILLE.with(|p| p.borrow_mut()[(z.spieler & 7) as usize] = Some((i + 1, punkte.len())));
            } else { PATROUILLE.with(|p| p.borrow_mut()[(z.spieler & 7) as usize] = None); }
        } else { PATROUILLE.with(|p| p.borrow_mut()[(z.spieler & 7) as usize] = None); }
        wx = wx.clamp(3, b - 4); wy = wy.clamp(3, h - 4); sx = sx.clamp(3, b - 4); sy = sy.clamp(3, h - 4);
    }
    // Wachgebiet (Nutzer 2026-11-19: Tuerme der Basis wurden angegriffen, die Fahrzeuge blieben drin): das Rechteck um alle
    // eigenen Gebaeude bis PAR wacheMax Kacheln von der Mitte -- Tuerme eingeschlossen -- plus PAR wacheRand, mindestens das
    // alte feste Rechteck (+-16 x +-14). Ferne Aussenposten bleiben draussen.
    let (mut x0, mut y0, mut x1, mut y1) = (z.bx - 16, z.by - 14, z.bx + 16, z.by + 14);
    let (rand, weit) = (PAR(z.spieler).wacheRand as i32, PAR(z.spieler).wacheMax as i32);
    for &typ in super::GEBAEUDE.iter() {
        for u in welt.PlayerBuildingEnum(z.spieler, typ) {
            if !welt.IsLive(u) { continue; }
            let lo = welt.Location(u);
            let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
            if abstand2(welt, x, y, z.bx, z.by) > weit * weit { continue; }
            x0 = x0.min(x - rand); y0 = y0.min(y - rand); x1 = x1.max(x + rand); y1 = y1.max(y + rand);
        }
    }
    let (x0, y0) = (x0.max(1), y0.max(1));
    let (x1, y1) = (x1.min(z.breite - 2), y1.min(z.hoehe - 2));
    z.wacheGebiet = [x0, y0, x1, y1];
    let basis = Rect::new(engine(welt, z, x0, y0), engine(welt, z, x1, y1));
    Plaetze { wx, wy, sx, sy, basis }
}

/// Gruppen anlegen; alle 2000 Ticks Plaetze pruefen und verlegen; Zugangswachen (PAR zugang)
fn gruppenAnlegen(welt: &mut dyn Welt, z: &mut Zustand, pl: &Plaetze) {
    let Plaetze { wx, wy, sx, sy, basis } = *pl;
    let sp = z.spieler;
    if z.wacheAn == 0 { z.wache = welt.CreateFightGroup(z.spieler); z.wacheAn = 1; heim(welt, z, wx, wy, basis, None); }
    if z.sammelAn == 0 { z.sammel = welt.CreateFightGroup(z.spieler); z.sammelAn = 1; sammelPlatz(welt, z, sx, sy, basis); }
    // Die Basis waechst: alle 2000 Ticks pruefen, ob die Plaetze noch frei sind, sonst verlegen.
    if welt.Tick() % 2000 < TAKT && z.geleitSeit == 0 {   // im Geleitschutz nicht heimholen
        let mut hx = wx; let mut hy = wy; freierWarteplatz(welt, z, &mut hx, &mut hy, 3, true);
        if hx != platzLesen(sp, 0) || hy != platzLesen(sp, 1) {
            match PATROUILLE.with(|p| p.borrow()[(sp & 7) as usize]) {
                Some((i, n)) => heim(welt, z, wx, wy, basis, Some(&format!("Patrouille um die Basis: Punkt {} von {}", i, n))),
                None => heim(welt, z, wx, wy, basis, Some("Warteplatz verlegt (alter Platz belegt)")),
            }
        }
        else if z.wacheGebiet != z.wacheGebietAlt {
            let g = z.wacheGebiet;
            heim(welt, z, wx, wy, basis, Some(&format!("Wachgebiet {},{} bis {},{} (alle Gebaeude der Basis, Tuerme eingeschlossen)", g[0], g[1], g[2], g[3])));
        }
        z.wacheGebietAlt = z.wacheGebiet;
        let mut qx = sx; let mut qy = sy; freierWarteplatz(welt, z, &mut qx, &mut qy, 2, false);
        if qx != platzLesen(sp, 2) || qy != platzLesen(sp, 3) {
            sammelPlatz(welt, z, sx, sy, basis);
            melde(welt, z, "kampf", mapLynx, qx, qy, &format!("Sammelplatz verlegt nach {},{} (alter Platz belegt)", qx, qy));
        }
        if z.fabrikAn != 0 {
            let mut fx = z.bx; let mut fy = z.by + 7; freierWarteplatz(welt, z, &mut fx, &mut fy, 2, false);
            if fx != platzLesen(sp, 8) || fy != platzLesen(sp, 9) {
                platzSetzen(sp, 8, fx); platzSetzen(sp, 9, fy);
                let r = Rect::new(engine(welt, z, fx - 3, fy - 1), engine(welt, z, fx + 3, fy + 2)); welt.SetRect(z.fabrik, r);
            }
        }
    }
    if z.angriffAn == 0 { z.angriff = welt.CreateFightGroup(z.spieler); z.angriffAn = 1; }
    if z.angriff2An == 0 { z.angriff2 = welt.CreateFightGroup(z.spieler); z.angriff2An = 1; }
    if z.reserveAn == 0 { z.eingreif = welt.CreateFightGroup(z.spieler); z.reserveAn = 1; z.reserveOrt = -2; }
    // Rust (PAR zugang): Wache auf die Zugaenge der Kolonie verteilen, Rueckzug bei Uebermacht
    if PAR(z.spieler).zugang >= 0.5 && PAR(z.spieler).gelaende > 0.0 && PAR(z.spieler).zugangGruppen >= 0.5 { super::erweiterung::zugangWachen(welt, z, basis); }
}

/// Geleitschutz (PAR geleit; Nutzer 2026-11-20: Transporter wurden angegriffen, die Kampfeinheiten blieben in der Basis):
/// Feinde an Lastwagen, Bergbau- und Bau-Fahrzeugen oder Minen ausserhalb des Wachgebiets (bis PAR geleitMax Kacheln von der
/// Basis) -- die Wache faehrt hin, wenn sie PAR geleitUeber-mal staerker ist, und kehrt 300 Ticks nach dem letzten Feind
/// dort heim, sofort, wenn die Basis selbst angegriffen wird oder der Feind zu stark wird.
fn geleitSchutz(welt: &mut dyn Welt, z: &mut Zustand, t: i32, pl: &Plaetze) {
    if PAR(z.spieler).geleit < 0.5 || z.wacheAn == 0 { return; }
    let (r, weit, ueber) = (PAR(z.spieler).geleitRadius as i32, PAR(z.spieler).geleitMax as i32, PAR(z.spieler).geleitUeber);
    let g = z.wacheGebiet;
    let draussen = |x: i32, y: i32| x < g[0] || x > g[2] || y < g[1] || y > g[3];
    let mut punkte: Vec<(i32, i32)> = Vec::new();
    for u in welt.PlayerVehicleEnum(z.spieler) {
        if !welt.IsLive(u) { continue; }
        let typ = welt.GetType(u);
        if typ != mapCargoTruck && typ != mapRoboMiner && typ != mapConVec && typ != mapRoboSurveyor { continue; }
        let lo = welt.Location(u); punkte.push((edX(welt, z, lo), edY(welt, z, lo)));
    }
    for &typ in [mapCommonOreMine, mapRareOreMine].iter() {
        for u in welt.PlayerBuildingEnum(z.spieler, typ) {
            if !welt.IsLive(u) { continue; }
            let lo = welt.Location(u); punkte.push((edX(welt, z, lo), edY(welt, z, lo)));
        }
    }
    let mut best: Option<(i32, i32, i32)> = None;
    for &(x, y) in punkte.iter() {
        if !draussen(x, y) || abstand2(welt, x, y, z.bx, z.by) > weit * weit { continue; }
        let d = feindStaerkeBei(welt, z, x, y, r);
        if d > 0 && best.map_or(true, |b| d > b.2) { best = Some((x, y, d)); }
    }
    let basisFeind = feindStaerkeBei(welt, z, z.bx, z.by, 16);
    let mut nw = 0; let ew = gruppenWert(welt, z.wache, &mut nw);
    match best {
        Some((x, y, d)) if basisFeind == 0 && nw > 0 && ew as f64 >= d as f64 * ueber => {
            if z.geleitSeit == 0 || abstand2(welt, x, y, z.geleitX, z.geleitY) > 36 {
                let warte = Rect::new(engine(welt, z, x - 2, y - 2), engine(welt, z, x + 2, y + 2));
                let schutz = Rect::new(engine(welt, z, x - r, y - r), engine(welt, z, x + r, y + r));
                wachen(welt, z.wache, warte, &[schutz], 1);
                let zl = engine(welt, z, x, y);
                for v in welt.GroupEnumerator(z.wache) { if welt.IsLive(v) && welt.IsVehicle(v) { welt.DoMove(v, zl); } }
                melde(welt, z, "kampf", mapLynx, x, y, &format!("Geleitschutz: Wache ({} Fahrzeuge, Wert {}) zu Feinden (Wert {}) bei {},{}", nw, ew, d, x, y));
                z.geleitX = x; z.geleitY = y; z.geleitSeit = t;
            }
            z.geleitRuhe = t;
        }
        _ => {
            if let Some((x, y, d)) = best {
                if z.geleitSeit == 0 && welt.Tick() % 1000 < TAKT {
                    melde(welt, z, "kampf", mapLynx, x, y, &format!("Kein Geleitschutz: Feinde (Wert {}) bei {},{} -- Wache {} Fahrzeuge, Wert {}{}", d, x, y, nw, ew, if basisFeind > 0 { ", Basis angegriffen" } else { "" }));
                }
            }
            if z.geleitSeit == 0 { return; }
            let warum = if basisFeind > 0 { "Basis wird angegriffen" } else if best.is_some() { "Feind zu stark" } else if t - z.geleitRuhe >= 300 { "kein Feind mehr" } else { return };
            heim(welt, z, pl.wx, pl.wy, pl.basis, Some(&format!("Geleitschutz beendet ({})", warum)));
            z.geleitSeit = 0;
        }
    }
}

/// Fabrikgruppe mit der ersten fertigen Fahrzeugfabrik, weitere Fabriken dazu
fn fabrikGruppe(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    let sp = z.spieler;
    // Fabrikgruppe mit der (fertigen, ruhigen) Fahrzeugfabrik
    if l.vf != 0 && (z.fabrikAn == 0 || z.fabrikVf != l.vf) && bereit(welt, l.vf) {
        hinweis(welt, z, "ki::kampfFuehrung/fabrikgruppe");
        z.fabrik = welt.CreateBuildingGroup(z.spieler);
        let mut fx = z.bx; let mut fy = z.by + 7; freierWarteplatz(welt, z, &mut fx, &mut fy, 2, false);   // frei von Gebaeuden und Erzwegen
        platzSetzen(sp, 8, fx); platzSetzen(sp, 9, fy);
        let r = Rect::new(engine(welt, z, fx - 3, fy - 1), engine(welt, z, fx + 3, fy + 2));
        welt.SetRect(z.fabrik, r);
        welt.TakeUnit(z.fabrik, l.vf);
        welt.RecordVehReinforceGroup(z.fabrik, z.wache, 3000);
        welt.RecordVehReinforceGroup(z.fabrik, z.sammel, 1000);
        welt.RecordVehReinforceGroup(z.fabrik, z.eingreif, 500);
        z.fabrikAn = 1; z.fabrikVf = l.vf;
        for i in 0..MAX_VF { z.fabrikDrin[i] = 0; }
        z.fabrikDrin[0] = l.vf;
        let vl = welt.Location(l.vf); let (ex, ey) = (edX(welt, z, vl), edY(welt, z, vl));
        melde(welt, z, "kampf", mapVehicleFactory, ex, ey,
              "Fabrikgruppe angelegt: die Fahrzeugfabrik verstaerkt Wache (Prioritaet 3000) und Sammelgruppe (1000)");
    }
    // Weitere Fahrzeugfabriken (fertig, ruhig) in dieselbe Fabrikgruppe
    if z.fabrikAn != 0 {
        for j in 1..l.vfAnzahl.max(0) as usize {
            let v: Id = l.vfs[j];
            if !bereit(welt, v) { continue; }
            let mut frei: i32 = -1; let mut drin = false;
            for i in 0..MAX_VF {
                if z.fabrikDrin[i] == v { drin = true; }
                let alt: Id = z.fabrikDrin[i];
                if frei < 0 && (z.fabrikDrin[i] == 0 || !welt.IsLive(alt) || welt.GetType(alt) != mapVehicleFactory) { frei = i as i32; }
            }
            if drin || frei < 0 { continue; }
            hinweis(welt, z, "ki::kampfFuehrung/fabrik-dazu");
            welt.TakeUnit(z.fabrik, v);
            z.fabrikDrin[frei as usize] = v;
            let vl = welt.Location(v); let (ex, ey) = (edX(welt, z, vl), edY(welt, z, vl));
            melde(welt, z, "kampf", mapVehicleFactory, ex, ey, &format!("Weitere Fahrzeugfabrik in die Fabrikgruppe ({} Fabriken)", l.vfAnzahl));
        }
    }
}

/// Sollzahlen von Wache und Sammelgruppe aus Kampfwert-Bedarf, Erzvorrat und Zufluss; Reservegruppe
fn sollzahlen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) -> Soll {
    // Sollzahlen: Wache deckt den fehlenden Kampfwert (ohne Tuerme) plus die Aussenposten, Sammelgruppe die Angriffstruppe
    hinweis(welt, z, "ki::kampfFuehrung/soll");
    let fz = fahrgestellWahl(welt, z, l.selten);
    let mut wacheZiel = 0; let mut sammelZiel = 0; let mut waffe = mapNone;
    if fz != 0 {
        let fiSel = welt.GetRareOreCost(fz, z.spieler);
        waffe = waffeWahl(welt, z, fz, l.selten - fiSel);
        let wert = if sdkWertT(welt, fz, waffe) > 0 { sdkWertT(welt, fz, waffe) } else { WERT_LYNX };
        let turm = z.eigeneWTWert + (geplant(welt, z, mapGuardPost) + l.kitsImSpiel[mapGuardPost as usize] + l.imBau[mapGuardPost as usize]) * WERT_WT;
        let fehlt = z.sollArmee - turm;
        wacheZiel = if fehlt > 0 { (fehlt + wert - 1) / wert } else { 0 };
        for i in 0..6 { wacheZiel += aussenFahrzeuge(welt, z, i); }
        let angriffSoll = mobilSoll(welt, z, true) - mobilSoll(welt, z, false);
        sammelZiel = if angriffSoll > 0 { (angriffSoll + wert - 1) / wert } else { 0 };
        // Rust (kampfModell): wehrloser Feind (keine Armee, keine Tuerme) -- daheim droht nichts, also kommt alles in die
        // Angriffstruppe. Sonst blieb sie bei 1 Lynx, alles Weitere ging in die Reserve, und der Angriff (braucht 2) kam nie.
        let haelt = super::super::modus::mission(z) && !super::super::modus::eigeneAngriffe(z, welt.Tick());
        if kampfModellNeu() && PAR(z.spieler).wehrlos >= 0.5 && z.feindGeb > 0 && z.feindArmee == 0 && z.feindWT == 0 && !haelt { sammelZiel = sammelZiel.max(GRUPPE_MAX); }
        // Keine feste Obergrenze: vorhandene Fahrzeuge plus was Vorrat und Zufluss in der Reaktionszeit hergeben.
        let wacheOhne = wacheZiel; let sammelOhne = sammelZiel;
        let ks = welt.GetOreCost(fz, z.spieler) + welt.GetOreCost(waffe, z.spieler);
        let kosten = if ks > 0 { ks } else { 1 };
        let bl = bilanz(welt, z, l);
        let mut zufluss = erzRouten(welt, z) as f64 * bl.routeErz as f64 * 1000.0 / PAR(z.spieler).horizont;
        if z.zuflussErz > zufluss { zufluss = z.zuflussErz; }
        let neu = ((l.erz as f64 + zufluss * PAR(z.spieler).reaktion / 1000.0) / kosten as f64) as i32;
        let mut nw = 0; let mut ns = 0;
        gruppenWert(welt, z.wache, &mut nw); gruppenWert(welt, z.sammel, &mut ns);
        if wacheZiel > nw + neu { wacheZiel = nw + neu; }
        let rest = neu - if wacheZiel > nw { wacheZiel - nw } else { 0 };
        if sammelZiel > ns + if rest > 0 { rest } else { 0 } { sammelZiel = ns + if rest > 0 { rest } else { 0 }; }
        // Die Erz-Grenze bremst nur das Erhoehen: ein schon gesetztes Ziel faellt nicht.
        if wacheZiel < z.wacheZiel && wacheOhne >= z.wacheZiel { wacheZiel = z.wacheZiel; }
        if sammelZiel < z.sammelZiel && sammelOhne >= z.sammelZiel { sammelZiel = z.sammelZiel; }
        // Seltenerz-Schmelze geht vor neuen Kampffahrzeugen, solange keine akute Bedrohung besteht.
        // Missionsmodus: Vorrang des befohlenen Aussenpostens -- neue Kampffahrzeuge warten, solange die Bedrohung unter 50 liegt
        let roehren = super::super::modus::mission(z) && PAR(z.spieler).roehrenVorrang > 0.0 && (0..z.anzahlRoehren.max(0) as usize).any(|i| z.roehren[i].zustand == 1);
        let posten = z.bedrohung < 50 && (super::super::zweitbasis::postenWartet(welt, z, l).is_some() || roehren);
        let sparen = ((l.seltenSpart || l.erzSpart) && (z.bedrohung as f64) < PAR(z.spieler).sparBedrohung) || posten;   // PAR sparBedrohung (50)
        if sparen { if wacheZiel > nw { wacheZiel = nw; } if sammelZiel > ns { sammelZiel = ns; } }
        // Fabrik laeuft durch (Nutzer 2026-11-20: Fahrzeuge ununterbrochen, vor allem am Anfang): liegt die mobile Armee
        // unter ihrem Soll (Tuerme zaehlen nicht), bekommt die Fabrikgruppe immer mindestens ein Fahrzeug Bedarf -- auch
        // beim Sparen und an der Erz-Grenze. Sonst legte die KI die Fabrik still und bekam die Arbeiter nicht zurueck.
        // Ausnahme (2026-11-20): spart die KI fuer eine lohnende Schmelze, wartet die Fabrik -- ausser im Fenster der fruehen
        // Wehr. Sonst kam die zweite Schmelze nie (Erz ging restlos in Fahrzeuge) und mit ihr nicht das Medical Center.
        let tk = welt.Tick();
        let fwFenster = z.fwAngriff > 0 && tk >= z.fwAngriff - PAR(z.spieler).fwVorlauf as i32 && tk <= z.fwAngriff + PAR(z.spieler).fwNach as i32;
        // Missionsmodus ohne eigene Angriffe (Rolle defensiv): die Sammelgruppe zieht nie los -- nicht ueber ihr Soll wachsen
        // lassen, kein "wehrloser Feind"-Aufschlag auf 32 und keine Reserve (Mission cm03: 60 Lynx, das Erz fehlte der zweiten
        // Basis; ihre Groesse richtet sich nach der Feindarmee)
        if wacheZiel <= nw && sammelZiel <= ns && z.eigeneMobil < mobilSoll(welt, z, true) && (!sparen || fwFenster) && !haelt { sammelZiel = ns + 1; }
        // Reserve: erst wenn Wache und Sammelgruppe stehen, und nur so viel, wie Vorrat und Zufluss hergeben
        {
            let mut nr = 0; gruppenWert(welt, z.eingreif, &mut nr);
            // ohne eigene Angriffe (Missionsmodus) haelt die Reserve, was sie hat, und bestellt nichts nach (Aufstieg)
            let mut rz = if haelt { 0 } else { nr };
            if !sparen && !haelt && nw >= wacheZiel && ns >= sammelZiel { rz = nr + neu; if rz > GRUPPE_MAX { rz = GRUPPE_MAX; } }
            if rz != z.reserveZiel { welt.ClearTargCount(z.eingreif); if rz > 0 { welt.SetTargCount(z.eingreif, fz, waffe, rz); } z.reserveZiel = rz; }
        }
        // Missionsmodus: ein Angriffsauftrag der Mission baut die Truppe, die er braucht (auch beim Sparen)
        let noetig = super::super::modus::noetig(z);
        if noetig > 0 { let n = (noetig + wert - 1) / wert; if sammelZiel < n { sammelZiel = n; } }
        // Eine Gruppe fasst 32, die Fabrik haelt dort an.
        { if wacheZiel > GRUPPE_MAX { wacheZiel = GRUPPE_MAX; } if sammelZiel > GRUPPE_MAX { sammelZiel = GRUPPE_MAX; } }
    }
    Soll { fz, waffe, wacheZiel, sammelZiel }
}

/// EMP- und Stickyfoam-Beimischung der Angriffstruppe; Sollzahlen an die Gruppen geben, wenn sich etwas aendert
fn beimischungSetzen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, soll: &Soll) {
    let Soll { fz, waffe, wacheZiel, sammelZiel } = *soll;
    // EMP-Beimischung: 25 % der Angriffstruppe, wenn die gemessene Verstaerkung den Aufpreis uebersteigt.
    let mut empN = 0;
    let mut empFz = fz;
    if kampfModellNeu() {
        // Rust (kampfModell): EMP-Anteil q und EMP-Traeger nach Staerke je Erz (Seltenerz doppelt) der ganzen Angriffstruppe:
        // ((1-q) g x (1 + a q) + q g_EMP) / ((1-q) Kosten + q Kosten_EMP), q in 0 .. 50 % (Arena 8: a je Hauptwaffe).
        if fz != 0 && sammelZiel >= 2 && waffe != mapEMP && einheitErforscht(welt, z, mapEMP) {
            let s = z.spieler;
            let kosten = |welt: &mut dyn Welt, f: i32, w: i32| (welt.GetOreCost(f, s) + welt.GetOreCost(w, s) + 2 * (welt.GetRareOreCost(f, s) + welt.GetRareOreCost(w, s))).max(1) as f64;
            let gm = gegenwert(welt, s, fz, waffe, 1.0); let km = kosten(welt, fz, waffe);
            let a = empVerstaerkung(waffe);
            let mut best = gm / km; let mut bestQ = 0.0;
            for ec in [mapLynx, fz] {
                if !einheitErforscht(welt, z, ec) || welt.GetRareOreCost(ec, s) + welt.GetRareOreCost(mapEMP, s) > l.selten { continue; }
                let ge = gegenwert(welt, s, ec, mapEMP, 1.0); let ke = kosten(welt, ec, mapEMP);
                for i in 1..=5 {
                    let q = i as f64 / 10.0;
                    let v = ((1.0 - q) * gm * (1.0 + a * q) + q * ge) / ((1.0 - q) * km + q * ke);
                    if v > best { best = v; bestQ = q; empFz = ec; }
                }
            }
            empN = (bestQ * sammelZiel as f64 + 0.5) as i32;
            if empN == 0 { empFz = fz; }
        }
    } else if fz != 0 && sammelZiel >= 4 && waffe != mapEMP && hatTech(welt, z, 5508) {
        let pm = (welt.GetOreCost(fz, z.spieler) + welt.GetOreCost(waffe, z.spieler)) as f64;
        let ph = (welt.GetOreCost(fz, z.spieler) + welt.GetOreCost(mapEMP, z.spieler)) as f64;
        let faktor = if einheitErforscht(welt, z, mapTiger) { 1.12 } else { 1.06 };
        if pm > 0.0 && faktor > (3.0 * pm + ph) / (4.0 * pm) { empN = sammelZiel / 4; }
    }
    // Rust: Stickyfoam-Anteil der Angriffstruppe (Stellschraube schaumAnteil, Plymouth, Lynx-Traeger)
    let mut schaumN = 0;
    if kampfModellNeu() && PAR(z.spieler).schaumAnteil > 0.0 && fz != 0 && sammelZiel >= 3 && !welt.IsEden(z.spieler)
        && einheitErforscht(welt, z, mapStickyfoam) && einheitErforscht(welt, z, mapLynx) {
        schaumN = ((PAR(z.spieler).schaumAnteil * sammelZiel as f64 + 0.5) as i32).min(sammelZiel - empN - 1).max(0);
    }
    if schaumN != z.zielSchaum { z.wacheZiel = -1; }   // erzwingt das Neusetzen
    if z.wirtVorrang != 0 {
        // fabrikWirtschaft: die Fabrik baut gleich ein Wirtschaftsfahrzeug -- keine Kampf-Auftraege; -2 erzwingt das Neusetzen danach
        if z.wacheZiel != -2 { welt.ClearTargCount(z.wache); welt.ClearTargCount(z.sammel); z.wacheZiel = -2; }
    } else if fz != z.zielFz || waffe != z.zielWaffe || wacheZiel != z.wacheZiel || sammelZiel != z.sammelZiel || empN != z.zielEmp || (empN > 0 && empFz != z.zielEmpFz) {
        welt.ClearTargCount(z.wache); welt.ClearTargCount(z.sammel);
        if fz != 0 && wacheZiel > 0 { welt.SetTargCount(z.wache, fz, waffe, wacheZiel); }
        if fz != 0 && sammelZiel > 0 { welt.SetTargCount(z.sammel, fz, waffe, sammelZiel - empN - schaumN); }
        if fz != 0 && (schaumN > 0 || z.zielSchaum > 0) { welt.SetTargCount(z.sammel, mapLynx, mapStickyfoam, schaumN); }
        if schaumN != z.zielSchaum { melde(welt, z, "kampf", mapLynx, z.bx, z.by, &format!("Stickyfoam-Beimischung: {} von {} Fahrzeugen der Angriffstruppe", schaumN, sammelZiel)); }
        z.zielSchaum = schaumN;
        if fz != 0 && (empN > 0 || z.zielEmp > 0) { welt.SetTargCount(z.sammel, empFz, mapEMP, empN); }
        if empN != z.zielEmp || (empN > 0 && empFz != z.zielEmpFz) {
            let ne = name(welt, empFz);
            melde(welt, z, "kampf", fz, z.bx, z.by, &format!("EMP-Beimischung: {} von {} Fahrzeugen der Angriffstruppe ({})", empN, sammelZiel, ne));
        }
        z.zielEmp = empN; z.zielEmpFz = empFz;
        let (nf, nwf) = (name(welt, fz), name(welt, waffe));
        let ms = mobilSoll(welt, z, true) - mobilSoll(welt, z, false);
        melde(welt, z, "kampf", if fz != 0 { fz } else { mapLynx }, z.bx, z.by,
              &format!("Sollzahlen: Wache {}, Sammelgruppe {} {} mit {} (Soll {}, Tuerme {}, Angriffstruppe {})",
                       wacheZiel, sammelZiel, nf, nwf, z.sollArmee, z.eigeneWTWert, ms));
        z.zielFz = fz; z.zielWaffe = waffe; z.wacheZiel = wacheZiel; z.sammelZiel = sammelZiel;
    }
}

/// Aussenposten: Fahrzeuge aus der Wache abziehen bzw. zurueckgeben
fn aussenpostenVerteilen(welt: &mut dyn Welt, z: &mut Zustand, t: i32, pl: &Plaetze) {
    let Plaetze { wx, wy, basis, .. } = *pl;
    // 1. Aussenposten: Fahrzeuge aus der Wache abziehen bzw. zurueckgeben (nur lebende)
    hinweis(welt, z, "ki::kampfFuehrung/aussenposten");
    for i in 0..6usize {
        let soll = aussenFahrzeuge(welt, z, i as i32);
        if z.aussenAn[i] == 0 && soll == 0 { continue; }
        if z.aussenAn[i] == 0 { z.aussenGruppe[i] = welt.CreateFightGroup(z.spieler); z.aussenAn[i] = 1; }
        let mut n = 0; let mut nw = 0;
        gruppenWert(welt, z.aussenGruppe[i], &mut n);
        gruppenWert(welt, z.wache, &mut nw);
        let mut geaendert = false;
        while n > soll {
            let mut v: Id = 0; let mut da = false;
            for u in welt.GroupEnumerator(z.aussenGruppe[i]) { v = u; if welt.IsLive(u) && kaempfer(welt, u) { da = true; break; } }
            if !da { break; }
            if welt.TotalUnitCount(z.wache) >= GRUPPE_MAX { break; }
            welt.RemoveUnit(z.aussenGruppe[i], v); welt.TakeUnit(z.wache, v); n -= 1; geaendert = true;
        }
        while n < soll && nw > 1 {
            let mut v: Id = 0; let mut da = false;
            for u in welt.GroupEnumerator(z.wache) { v = u; if welt.IsLive(u) && kaempfer(welt, u) { da = true; break; } }
            if !da { break; }
            if welt.TotalUnitCount(z.aussenGruppe[i]) >= GRUPPE_MAX { break; }
            welt.RemoveUnit(z.wache, v); welt.TakeUnit(z.aussenGruppe[i], v); n += 1; nw -= 1; geaendert = true;
        }
        if geaendert {
            let r = Rect::new(engine(welt, z, z.aussenX[i] - 2, z.aussenY[i] - 2), engine(welt, z, z.aussenX[i] + 2, z.aussenY[i] + 2));
            let g = Rect::new(engine(welt, z, z.aussenX[i] - 7, z.aussenY[i] - 7), engine(welt, z, z.aussenX[i] + 7, z.aussenY[i] + 7));
            wachen(welt, z.aussenGruppe[i], r, &[g], 1);
            heim(welt, z, wx, wy, basis, None);
            let grund = if z.aussenAngriff[i] != 0 && t - z.aussenAngriff[i] < 10000 { "wurde angegriffen" } else if soll != 0 { "naeher am Feind als die Basis" } else { "keine Gefahr mehr" };
            melde(welt, z, "kampf", mapLynx, z.aussenX[i], z.aussenY[i],
                  &format!("Aussenposten bei {},{}: {} Fahrzeug(e) ({})", z.aussenX[i], z.aussenY[i], n, grund));
        }
        if z.aussenId[i] == 0 && n == 0 { z.aussenAn[i] = 0; }
    }
}

/// Kampffahrzeuge ohne Gruppe zur Wache; Bestand der Gruppen zaehlen
fn streunerAufnehmen(welt: &mut dyn Welt, z: &mut Zustand, t: i32, pl: &Plaetze, soll: &Soll) -> Bestand {
    let Plaetze { wx, wy, basis, .. } = *pl;
    let Soll { fz, wacheZiel, sammelZiel, .. } = *soll;
    // 2. Kampffahrzeuge ohne Gruppe (z.B. nach Neuanlage) zur Wache -- nur fertige, ruhige
    hinweis(welt, z, "ki::kampfFuehrung/streuner");
    let mut neu = 0; let mut alle = 0;
    for u in welt.PlayerVehicleEnum(z.spieler) {
        if !welt.IsLive(u) || !kaempfer(welt, u) { continue; }
        alle += 1;
        if u == z.garDran || u == z.garRaus { continue; }   // gehoert gerade der Garage
        if super::super::modus::mission(z) && super::super::zweitbasis::amPosten(welt, z, u) { continue; }   // Wache der zweiten Basis
        if inGruppeG(welt, z.wache, u) || inGruppeG(welt, z.sammel, u) || inGruppeG(welt, z.angriff, u) || inGruppeG(welt, z.angriff2, u) || inGruppeG(welt, z.eingreif, u) { continue; }
        let mut aussen = false;
        for i in 0..6 { if aussen { break; } if z.aussenAn[i] != 0 && inGruppeG(welt, z.aussenGruppe[i], u) { aussen = true; } }
        if aussen || welt.GetCurAction(u) != moDone || welt.IsBusy(u) { continue; }
        let mut aufGebaeude = false;
        for o in welt.LocationEnumerator(welt.Location(u)) { if welt.IsBuilding(o) { aufGebaeude = true; break; } }
        if aufGebaeude { continue; }
        if sicherNehmen(welt, z, z.wache, u) || sicherNehmen(welt, z, z.eingreif, u) { neu += 1; }
    }
    if alle > z.kampfFzAnzahl { z.letztesKampfFz = t; }                 // fuer die Selbstpruefung
    z.kampfFzAnzahl = alle;
    if neu != 0 { heim(welt, z, wx, wy, basis, Some("Kampffahrzeuge ohne Gruppe aufgenommen")); }
    // Missionsmodus (aufstieg, Nutzer 2026-11-22: "Fahrzeuge mit der Forschung besser werden"): leichtere Fahrgestelle als das
    // gewaehlte wechseln aus der Sammelgruppe in die Reserve (verteidigen weiter); die Fabrik fuellt die Luecke mit dem neuen
    // Fahrgestell. EMP- und Stickyfoam-Beimischung bleiben. Je Takt hoechstens zwei, damit die Truppe nie leer ist.
    if PAR(z.spieler).aufstieg >= 0.5 && super::super::modus::mission(z) && fz != 0 && t % 400 < TAKT {
        let stufe = |typ: i32| match typ { x if x == mapTiger => 3, x if x == mapPanther => 2, x if x == mapLynx => 1, _ => 0 };
        let mut n = 0;
        let alt: Vec<Id> = welt.GroupEnumerator(z.sammel).into_iter().filter(|&v| welt.IsLive(v) && stufe(welt.GetType(v)) > 0
            && stufe(welt.GetType(v)) < stufe(fz) && !matches!(welt.GetWeapon(v), w if w == mapEMP || w == mapStickyfoam)).collect();
        for v in alt {
            if n >= 2 { break; }
            welt.RemoveUnit(z.sammel, v); welt.TakeUnit(z.eingreif, v); n += 1;
        }
        if n > 0 { let nf = name(welt, fz); melde(welt, z, "kampf", fz, z.bx, z.by, &format!("Aufstieg: {} aeltere Fahrzeuge in die Reserve, die Sammelgruppe bekommt {}", n, nf)); }
    }
    let mut nw = 0; let mut ns = 0; let mut na = 0;
    gruppenWert(welt, z.wache, &mut nw);
    gruppenWert(welt, z.sammel, &mut ns);
    let wa = angriffWert(welt, z, &mut na);
    z.wacheAnzahl = nw;
    { let mut nr = 0; gruppenWert(welt, z.eingreif, &mut nr); z.gruppenBedarf = if fz != 0 && (nw < wacheZiel || ns < sammelZiel || nr < z.reserveZiel) { 1 } else { 0 }; }
    Bestand { nw, ns, na, wa }
}

/// Feindliche Tuerme: Wert aller und Wert der Tuerme, die die Feindbasis decken (15 Kacheln)
fn feindTuerme(welt: &mut dyn Welt, z: &mut Zustand) -> (i32, i32) {
    let mut feindWTWert = z.feindWT * WERT_WT;
    let mut feindWTBasis = 0;
    {                                                  // echte Turmwerte statt Schaetzung
        feindWTWert = 0;
        let mut p = 0;
        while p < welt.NoPlayers() {
            if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { p += 1; continue; }
            for g in welt.PlayerBuildingEnum(p, mapGuardPost) {
                if !welt.IsLive(g) { continue; }
                let w = kampfWert(welt, g);
                feindWTWert += w;
                // Rust (kampfModell): fuer den Basisangriff zaehlen nur Tuerme, die die Basis decken (15 Kacheln um ihre
                // Mitte, wie feindArmeeHeim) -- Tuerme an fernen Aussenposten oder Paessen stehen nicht im Weg.
                let gl = welt.Location(g);
                let (gx, gy) = (edX(welt, z, gl), edY(welt, z, gl));
                if abstand2(welt, gx, gy, z.feindX, z.feindY) <= 15 * 15 { feindWTBasis += w; }
            }
            p += 1;
        }
    }
    if !kampfModellNeu() { feindWTBasis = feindWTWert; }
    (feindWTWert, feindWTBasis)
}

/// Modus 3 (Aufmarsch) und 4 (EMP-Rakete im Flug)
fn aufmarschUndRakete(welt: &mut dyn Welt, z: &mut Zustand, t: i32, b: &mut Bestand) {
    let na = b.na; let mut nw = b.nw;
    // Aufmarsch / Rakete im Flug. Bricht etwas weg, wird daraus ein normaler Angriff (Modus 2).
    let a = gruppenWert(welt, z.wache, &mut nw); let b = gruppenWert(welt, z.eingreif, &mut nw);
    let kolonieInNot = z.feindDrohung > z.eigeneWTWert + a + b;
    if na == 0 || z.feindGeb == 0 || kolonieInNot { z.kampfModus = 2; return; }
    if z.kampfModus == 3 {
        // Feldherr (PAR marschSammeln): erst in Etappen bis zum Aufmarschpunkt, die Aufmarschfrist zaehlt ab der letzten Etappe
        if super::marsch::an(z) && super::marsch::marschieren(welt, z, t) { z.aufmarschSeit = t; return; }
        let mut dort = 0;
        for v in AngriffEnum(welt, z) {
            if welt.IsLive(v) { let lo = welt.Location(v); let (ex, ey) = (edX(welt, z, lo), edY(welt, z, lo)); if abstand2(welt, ex, ey, z.aufmarschX, z.aufmarschY) <= 5 * 5
                || (z.aufmarsch2X != 0 && abstand2(welt, ex, ey, z.aufmarsch2X, z.aufmarsch2Y) <= 5 * 5) { dort += 1; } }
        }
        // Frist nach der Entfernung (Lynx ~50 Ticks je Kachel im Pulk)
        let mut aufD = abstand2(welt, z.bx, z.by, z.aufmarschX, z.aufmarschY);
        if z.aufmarsch2X != 0 { aufD = aufD.max(abstand2(welt, z.bx, z.by, z.aufmarsch2X, z.aufmarsch2Y)); }   // Rust (PAR parallel)
        let aufFrist = 1000 + (70.0 * (aufD as f64).sqrt()) as i32;
        if dort * 10 >= na * 7 || t - z.aufmarschSeit > aufFrist {
            let mut sp: Id = 0;
            for u in welt.PlayerBuildingEnum(z.spieler, mapSpaceport) { if welt.IsLive(u) { sp = u; break; } }
            if z.empRakete == 0 {                                            // ohne Rakete: die Truppe steht, geschlossen los
                melde(welt, z, "kampf", mapLynx, z.aufmarschX, z.aufmarschY, &format!("Truppe steht ({} von {} nach {} Ticks) -- Angriff", dort, na, t - z.aufmarschSeit));
                z.kampfModus = 2; z.angriffZiel = 0; let (ax, ay) = (z.aufmarschX, z.aufmarschY); angriffZielSetzen(welt, z, ax, ay); return;
            }
            let wert = { let (mut ex, mut ey) = (z.empZielX, z.empZielY); let w = empZiel(welt, z, &mut ex, &mut ey); z.empZielX = ex; z.empZielY = ey; w };   // Ziel neu bestimmen
            // Start nur, wenn das Ziel die Rakete noch wert ist -- sonst ohne EMP angreifen
            if sp == 0 || z.empRakete == 0 || (wert as f64) < empKostenWert(welt, z) as f64 * PAR(z.spieler).empSchwelle {
                melde(welt, z, "kampf", mapLynx, z.aufmarschX, z.aufmarschY, &format!("Aufmarsch steht -- Ziel ist die Rakete nicht wert (Abwehr {}), Angriff ohne EMP", wert));
                z.kampfModus = 2; let (ax, ay) = (z.aufmarschX, z.aufmarschY); angriffZielSetzen(welt, z, ax, ay); return;
            }
            let ziel = engine(welt, z, z.empZielX, z.empZielY);
            welt.DoLaunch(sp, ziel.x * 32 + 16, ziel.y * 32 + 16, true);
            z.empRakete = 0; z.empStart = t; z.kampfModus = 4;
            melde(welt, z, "kampf", mapEMPMissile, z.empZielX, z.empZielY,
                  &format!("EMP-Rakete gestartet auf {},{} (Abwehr Wert {}, {} von {} Fahrzeugen am Aufmarschpunkt)", z.empZielX, z.empZielY, wert, dort, na));
        }
        return;
    }
    // Modus 4: nach dem Einschlag angreifen, waehrend die Abwehr gelaehmt ist
    if t - z.empStart >= EMP_FLUG {
        let mut gelaehmt = 0;
        let mut p = 0;
        while p < welt.NoPlayers() {
            if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { p += 1; continue; }
            for u in welt.PlayerVehicleEnum(p) { if welt.IsLive(u) && welt.isEMPed(u) { gelaehmt += 1; } }
            p += 1;
        }
        melde(welt, z, "kampf", mapLynx, z.empZielX, z.empZielY, &format!("EMP eingeschlagen ({} feindliche Fahrzeuge gelaehmt) -- Angriff", gelaehmt));
        z.kampfModus = 2; z.angriffZiel = 0;
        let (ax, ay) = (z.aufmarschX, z.aufmarschY); angriffZielSetzen(welt, z, ax, ay);
    }
    return;
}

/// Modus 2: laufender Angriff -- Abbruch pruefen, Route abfahren, Ziel setzen
fn angriffLaeuft(welt: &mut dyn Welt, z: &mut Zustand, pl: &Plaetze, b: &mut Bestand) {
    let Plaetze { wx, wy, basis, .. } = *pl;
    let (wa, na) = (b.wa, b.na); let mut nw = b.nw;
    bilanzAnkunft(welt, z);
    // kein Abbruch bei halber Staerke -- gekaempft wird bis zum Ende
    let _ = wa;
    let abbruch = abbruchGrund(welt, z, na, &mut nw);
    if let Some(abbruch) = abbruch { angriffBeenden(welt, z, na, pl, abbruch); return; }
    if kampfModellNeu() && PAR(z.spieler).welle >= 1.0 { let w = PAR(z.spieler).welle as i32; welleNachschub(welt, z, w); }
    if routeFolgen(welt, z) { return; }
    let (mut mx, mut my, mut k) = (0, 0, 0);
    for v in AngriffEnum(welt, z) { if welt.IsLive(v) { let lo = welt.Location(v); mx += edX(welt, z, lo); my += edY(welt, z, lo); k += 1; } }
    if k != 0 { angriffZielSetzen(welt, z, mx / k, my / k); }
    return;
}

/// Grund, den laufenden Angriff abzubrechen (Feindbasis weg, Gruppe aufgerieben, Kolonie in Not, Truppplan-Schwelle)
fn abbruchGrund(welt: &mut dyn Welt, z: &mut Zustand, na: i32, nw: &mut i32) -> Option<&'static str> {
    let mut abbruch: Option<&'static str> = None;
    if z.feindGeb == 0 { abbruch = Some("Feindbasis vernichtet"); }
    else if na == 0 { abbruch = Some("Angriffsgruppe aufgerieben"); }
    else {
        let a = gruppenWert(welt, z.wache, nw); let b = gruppenWert(welt, z.eingreif, nw);
        if z.feindDrohung > z.eigeneWTWert + a + b { abbruch = Some("Kolonie wird angegriffen, Wache und Tuerme zu schwach"); }
    }
    // Rust (Truppnetz): der gewaehlte Plan -- Rueckzug unter der Verlustschwelle, nach dem Ziel heim (wenn nicht weiter)
    if abbruch.is_none() && z.planAn != 0 && z.angriffArt == 2 {
        let mut jetzt = 0;
        for v in AngriffEnum(welt, z) { if welt.IsLive(v) && welt.IsVehicle(v) { jetzt += super::truppplan::fzWert(welt, v); } }
        let zl = z.planZiel;
        if z.planStart == 0 {
            z.planStart = jetzt.max(1);
        }
        if z.planRueck > 0 && (jetzt as i64) * 1000 < z.planRueck as i64 * z.planStart as i64 { abbruch = Some("Truppplan: Verlustschwelle erreicht -- Rueckzug"); }
        else if z.planWeiter == 0 && (!welt.IsLive(zl) || welt.OwnerID(zl) == z.spieler) { abbruch = Some("Truppplan: Ziel zerstoert -- heim"); }
    }
    abbruch
}

/// Angriff beenden: Bilanz lernen (Zermuerbungsabstand), Gruppe heim
fn angriffBeenden(welt: &mut dyn Welt, z: &mut Zustand, na: i32, pl: &Plaetze, abbruch: &'static str) {
    let Plaetze { wx, wy, basis, .. } = *pl;
    z.planAn = 0;
    // Lernen: was hat dieser Angriff gebracht und gekostet?
    let zer = z.zerstoertWert - z.angriffZerstoertStart; let ver = z.verlorenWert - z.angriffVerlorenStart;
    let quote = zer as f64 / (if ver > 0 { ver } else { 1 }) as f64;
    let art = z.angriffArt;
    let au = art as usize;
    z.angriffQuote[au] = z.angriffQuote[au] * 0.7 + (if quote > 5.0 { 5.0 } else { quote }) * 0.3;
    if art == 2 {
        let alt = z.zermuerbAbstand;
        z.zermuerbAbstand = if z.angriffQuote[2] < 0.7 { z.zermuerbAbstand * 3 / 2 } else { z.zermuerbAbstand * 4 / 5 };
        if z.zermuerbAbstand < 4000 { z.zermuerbAbstand = 4000; }
        if z.zermuerbAbstand > 30000 { z.zermuerbAbstand = 30000; }
        melde(welt, z, "lernen", mapLynx, z.bx, z.by, &format!("Zermuerben: zerstoert {}, verloren {} (Quote {:.2}, gleitend {:.2}) -- Abstand {} -> {} Ticks",
              zer, ver, quote, z.angriffQuote[2], alt, z.zermuerbAbstand));
    } else {
        melde(welt, z, "lernen", mapLynx, z.bx, z.by, &format!("{}: zerstoert {}, verloren {} (Quote {:.2}, gleitend {:.2})",
              if art == 1 { "Zielangriff" } else { "Angriff auf die Basis" }, zer, ver, quote, z.angriffQuote[au]));
    }
    hinweis(welt, z, "ki::kampfFuehrung/bilanzEnde"); bilanzEnde(welt, z, zer, ver, abbruch);
    z.naechstesSpeichern = 0;
    if na != 0 && { let a = sicherAlle(welt, z, z.wache, z.angriff); let b = sicherAlle(welt, z, z.wache, z.angriff2); a + b > 0 } {   // kein Platz in der Wache: heim
        let daheim = engine(welt, z, z.bx, z.by + 4);
        for v in AngriffEnum(welt, z) { if welt.IsLive(v) && welt.IsVehicle(v) { welt.DoMove(v, daheim); } }
    }
    z.kampfModus = 0; z.angriffWunsch = 0;
    heim(welt, z, wx, wy, basis, Some(abbruch));
}

/// Rust (Truppnetz, Route): vom Aufmarsch die Wegpunkte der gewaehlten Route ab (durch den gewaehlten Zugang);
/// erreicht (5 Kacheln) oder schon naeher am Ziel als er -> naechster, danach normal angreifen. true = noch unterwegs.
fn routeFolgen(welt: &mut dyn Welt, z: &mut Zustand) -> bool {
    if !(z.planAn != 0 && z.planWpI < z.planWpN) { return false; }
    let (mut mx, mut my, mut k) = (0, 0, 0);
    let ge = AngriffEnum(welt, z);
    for &v in ge.iter() { if welt.IsLive(v) { let lo = welt.Location(v); mx += edX(welt, z, lo); my += edY(welt, z, lo); k += 1; } }
    let zl = z.planZiel;
    if k > 0 && welt.IsLive(zl) {
        let (mx, my) = (mx / k, my / k);
        let lo = welt.Location(zl); let (zx, zy) = (edX(welt, z, lo), edY(welt, z, lo));
        while z.planWpI < z.planWpN {
            let i = z.planWpI as usize; let (wx2, wy2) = (z.planWpX[i], z.planWpY[i]);
            if abstand2(welt, mx, my, wx2, wy2) <= 25 || abstand2(welt, mx, my, zx, zy) < abstand2(welt, wx2, wy2, zx, zy) { z.planWpI += 1; } else { break; }
        }
        if z.planWpI < z.planWpN {
            let i = z.planWpI as usize;
            let ziel = engine(welt, z, z.planWpX[i], z.planWpY[i]);
            for v in ge { if welt.IsLive(v) && welt.IsVehicle(v) && (!welt.IsBusy(v) || welt.GetCurAction(v) == moMove) { welt.DoMove(v, ziel); } }
            return true;
        }
        melde(welt, z, "kampf", mapLynx, mx, my, "Truppplan: Route durchfahren -- Angriff");
    } else { z.planWpN = 0; }
    false
}
