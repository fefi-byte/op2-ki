//! Erweiterung der Rust-Fassung (nicht in der C++-KI), zuschaltbar mit PAR.gefecht (nur mit kampfModell): Gefechtsfuehrung
//! im Nahbereich, bei jedem AIProc-Aufruf (alle 4 Ticks), solange gekaempft wird. Nach dem Vorbild menschlicher Spieler
//! (Nutzer, Video Leviathan/xamlit 2016): EMP betaeubt die vorderen Feinde, Stickyfoam haelt die nachrueckenden fest, damit
//! sie die Betaeubten nicht freikaempfen; die Schadenswaffen zerstoeren gezielt die Betaeubten und Festgeklebten.
//! Kein Einheitentyp bricht dafuer durch die feindlichen Reihen: Ziele nur in Reichweite plus wenig Spiel.
//! Close combat control every AIProc while fighting: EMP the front, foam the followers, focus fire on the disabled.
use std::cell::RefCell;
use std::collections::HashMap;

use super::*;

#[derive(Clone, Default)]
struct Stand {
    /// letzter eigener Zielbefehl je Einheit: (Ziel, Tick)
    ziel: HashMap<Id, (Id, i32)>,
    /// Tick der letzten Feindberuehrung (Gefecht laeuft)
    gefecht: i32,
    /// letzter Ausweichzug je Einheit (Tick) -- Sperrzeit gegen Pendeln
    zug: HashMap<Id, i32>,
}

thread_local! {
    static STAND: RefCell<[Stand; 8]> = RefCell::new(Default::default());
}
pub fn zuruecksetzen_gefecht() { STAND.with(|s| *s.borrow_mut() = Default::default()); super::gefecht_netz::zuruecksetzen(); }

/// Zugziel (Engine-Kachel) mindestens 2 Kacheln vom Kartenrand -- ein Fahrbefehl ueber den Rand laesst das Original haengen.
/// Ohne bekannte Karte (Zustand ohne breite) keine Pruefung.
fn imFeld(welt: &mut dyn Welt, z: &Zustand, x: i32, y: i32) -> bool {
    z.breite <= 0 || aufKarte(welt, z, x - z.ux, y - z.uy, 2)
}

pub fn an(z: &Zustand) -> bool { kampfModellNeu() && PAR(z.spieler).gefecht >= 0.5 }

#[derive(Clone, Copy)]
struct Feind { u: Id, x: i32, y: i32, wert: i32, hp: i32, emp: i32, schaum: i32, rw: i32 }

/// Je AIProc: Ziele im Nahkampf verteilen.
/// Eigene Kampfeinheit nahe am Feind: Id, x, y, Waffe, Reichweite
type Eigen = (Id, i32, i32, i32, i32);
fn d2(ax: i32, ay: i32, bx: i32, by: i32) -> i32 { (ax - bx).pow(2) + (ay - by).pow(2) }

/// Gefechtsfuehrung eines Spielers (Handsteuerung oder Netz). Rust 2026-11-18 in Schritte aufgeteilt (Verhalten gleich).
pub fn fuehren(welt: &mut dyn Welt, z: &Zustand) {
    if !an(z) { return; }
    // gefecht >= 1.5: gelerntes Netz (gefecht_netz.rs); ohne Netz die Handsteuerung
    if netzFuehren(welt, z) { return; }
    let t = welt.Tick();
    let sp = (z.spieler & 7) as usize;
    // Ohne laufendes Gefecht nur alle 20 Ticks pruefen
    let letzt = STAND.with(|s| s.borrow()[sp].gefecht);
    if t - letzt > 40 && t % 20 >= 4 { return; }
    let r = PAR(z.spieler).gefechtRadius as i32;
    let (feinde, tuerme) = feindeErfassen(welt, z);
    if feinde.is_empty() && tuerme.is_empty() { return; }
    let eigene = eigeneNahe(welt, z, &feinde, &tuerme, r);
    if eigene.is_empty() { return; }
    STAND.with(|s| s.borrow_mut()[sp].gefecht = t);
    // Schwerpunkt der eigenen Truppe: "vorne" = nah an ihr, "hinten" = weiter weg
    let (mx, my) = {
        let n = eigene.len() as i32;
        (eigene.iter().map(|e| e.1).sum::<i32>() / n, eigene.iter().map(|e| e.2).sum::<i32>() / n)
    };
    let spiel = PAR(z.spieler).gefechtSpiel as i32;   // so weit darf eine Einheit fuer ihr Ziel vorruecken
    let mut befehle = zieleVerteilen(welt, z, &eigene, &feinde, &tuerme, mx, my, spiel);
    let mut zuege = stellung(welt, z, t, sp, &eigene, &feinde, &tuerme, &befehle);
    abstandHalten(welt, z, t, sp, &eigene, &feinde, &mut zuege);
    let ziehen: Vec<Id> = zuege.iter().map(|z| z.0).collect();
    befehle.retain(|b| !ziehen.contains(&b.0));
    // Befehle nur bei neuem Ziel (sonst setzt die Engine den Angriff jedes Mal neu an)
    STAND.with(|s| {
        let mut s = s.borrow_mut();
        let st = &mut s[sp];
        st.ziel.retain(|_, v| t - v.1 < 400);
        st.zug.retain(|_, v| t - *v < 2000);
        for &(u, x, y) in zuege.iter() {
            st.zug.insert(u, t);
            welt.DoMove(u, Loc::new(x, y));
            st.ziel.remove(&u);   // naechste Runde greift sie von dort wieder an
        }
        for (u, ziel) in befehle {
            if st.ziel.get(&u).is_some_and(|v| v.0 == ziel) { continue; }
            welt.DoAttack(u, ziel);
            st.ziel.insert(u, (ziel, t));
        }
    });
}

/// gefecht >= 1.5: gelerntes Netz (gefecht_netz.rs); true = das Netz hat gefuehrt (oder wartet auf seinen Takt)
fn netzFuehren(welt: &mut dyn Welt, z: &Zustand) -> bool {
    if PAR(z.spieler).gefecht >= 1.5 {
        if let Some(n) = super::gefecht_netz::netz(welt, z.spieler) {
            let t = welt.Tick();
            let sp = (z.spieler & 7) as usize;
            let letzt = STAND.with(|s| s.borrow()[sp].gefecht);
            if t - letzt > 40 && t % 20 >= 4 { return true; }
            STAND.with(|s| s.borrow_mut()[sp].gefecht = t);
            let (r, sperre) = (PAR(z.spieler).gefechtRadius as i32, PAR(z.spieler).gefechtSperre as i32);
            super::gefecht_netz::fuehren(welt, z, &n, r, sperre);
            return true;
        }
    }
    false
}

/// Feinde: Kampffahrzeuge und Tuerme (Tuerme nur als Ziel der Schadenswaffen, nicht fuer EMP/Schaum)
fn feindeErfassen(welt: &mut dyn Welt, z: &Zustand) -> (Vec<Feind>, Vec<Feind>) {
    let mut feinde: Vec<Feind> = Vec::new();
    let mut tuerme: Vec<Feind> = Vec::new();
    for p in 0..welt.NoPlayers() {
        if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { continue; }
        for u in welt.PlayerVehicleEnum(p) {
            if !welt.IsLive(u) { continue; }
            let w = kampfWert(welt, u);
            if w <= 0 { continue; }
            let l = welt.Location(u);
            let hp = (welt.GetHitPoints(welt.GetType(u), p) - welt.GetDamage(u)).max(1);
            let rw = welt.GetWeaponRange(welt.GetWeapon(u), p).max(1);
            feinde.push(Feind { u, x: l.x, y: l.y, wert: w, hp, emp: welt.zeitEMP(u), schaum: welt.zeitSchaum(u), rw });
        }
        for u in welt.PlayerBuildingEnum(p, mapGuardPost) {
            if !welt.IsLive(u) { continue; }
            let l = welt.Location(u);
            let hp = (welt.GetHitPoints(mapGuardPost, p) - welt.GetDamage(u)).max(1);
            let rw = welt.GetWeaponRange(welt.GetWeapon(u), p).max(1);
            tuerme.push(Feind { u, x: l.x, y: l.y, wert: kampfWert(welt, u), hp, emp: 0, schaum: 0, rw });
        }
    }
    (feinde, tuerme)
}

/// Eigene Kampfeinheiten (nicht betaeubt) im Gefechtsradius eines Feindes
fn eigeneNahe(welt: &mut dyn Welt, z: &Zustand, feinde: &[Feind], tuerme: &[Feind], r: i32) -> Vec<Eigen> {
    let mut eigene: Vec<(Id, i32, i32, i32, i32)> = Vec::new();   // u, x, y, Waffe, Reichweite
    for u in welt.PlayerVehicleEnum(z.spieler) {
        if !welt.IsLive(u) || !kaempfer(welt, u) || welt.isEMPed(u) { continue; }
        let l = welt.Location(u);
        let nah = feinde.iter().chain(tuerme.iter()).any(|f| (f.x - l.x).pow(2) + (f.y - l.y).pow(2) <= r * r);
        if !nah { continue; }
        let w = welt.GetWeapon(u);
        let rw = welt.GetWeaponRange(w, z.spieler).max(1);
        eigene.push((u, l.x, l.y, w, rw));
    }
    eigene
}

/// Ziele: 1. EMP auf die vordersten Starken, 2. Schaum auf die Nachrueckenden, 3. Schadenswaffen auf Gelaehmte und
/// Schwache, Feuer buendeln bis das Ziel sicher faellt. Liefert (Einheit, Ziel).
#[allow(clippy::too_many_arguments)]
fn zieleVerteilen(welt: &mut dyn Welt, z: &Zustand, eigene: &[Eigen], feinde: &[Feind], tuerme: &[Feind], mx: i32, my: i32, spiel: i32) -> Vec<(Id, Id)> {
    // Wer zielt schon auf wen (Schaden je Ziel, damit nicht alle auf einen schiessen)
    let mut zugeteilt: HashMap<Id, i32> = HashMap::new();
    let mut empZiel: Vec<Id> = Vec::new();
    let mut schaumZiel: Vec<Id> = Vec::new();
    let mut befehle: Vec<(Id, Id)> = Vec::new();
    // 1. EMP: vorderste nicht betaeubte Feinde, staerkste zuerst
    for &(u, x, y, w, rw) in eigene.iter().filter(|e| e.3 == mapEMP) {
        let reich = (rw + spiel).pow(2);
        let mut best: Option<(i64, Id)> = None;
        for f in feinde.iter() {
            if f.emp > 2 || empZiel.contains(&f.u) || d2(x, y, f.x, f.y) > reich { continue; }
            // vorne (nah am eigenen Schwerpunkt) und stark
            let wertung = f.wert as i64 * 1000 - d2(mx, my, f.x, f.y) as i64;
            if best.map_or(true, |b| wertung > b.0) { best = Some((wertung, f.u)); }
        }
        if let Some((_, ziel)) = best { empZiel.push(ziel); befehle.push((u, ziel)); }
        let _ = w;
    }
    // 2. Stickyfoam: die nachrueckenden -- weiter hinten als die Betaeubten, nicht betaeubt, nicht schon verklebt
    for &(u, x, y, _, rw) in eigene.iter().filter(|e| e.3 == mapStickyfoam) {
        let reich = (rw + spiel).pow(2);
        let mut best: Option<(i64, Id)> = None;
        for f in feinde.iter() {
            if f.schaum > 2 || f.emp > 2 || empZiel.contains(&f.u) || schaumZiel.contains(&f.u) || d2(x, y, f.x, f.y) > reich { continue; }
            // hinten (weit vom eigenen Schwerpunkt), aber erreichbar; staerkere zuerst
            let wertung = d2(mx, my, f.x, f.y) as i64 * 10 + f.wert as i64;
            if best.map_or(true, |b| wertung > b.0) { best = Some((wertung, f.u)); }
        }
        if let Some((_, ziel)) = best { schaumZiel.push(ziel); befehle.push((u, ziel)); }
    }
    // 3. Schadenswaffen: Betaeubte und Festgeklebte zuerst, dann die schwaechsten; Feuer buendeln bis das Ziel sicher faellt
    let mut sorte: Vec<&(Id, i32, i32, i32, i32)> = eigene.iter().filter(|e| e.3 != mapEMP && e.3 != mapStickyfoam).collect();
    sorte.sort_by_key(|e| d2(e.1, e.2, mx, my));
    for &&(u, x, y, w, rw) in sorte.iter() {
        let reich = (rw + spiel).pow(2);
        let schaden = welt.GetPenetrationDamage(w, z.spieler).max(1) + welt.GetConcussionDamage(w, z.spieler) / 2;
        let mut best: Option<(i64, Id)> = None;
        for f in feinde.iter().chain(tuerme.iter()) {
            if d2(x, y, f.x, f.y) > reich { continue; }
            let schon = *zugeteilt.get(&f.u).unwrap_or(&0);
            if schon >= f.hp * 2 { continue; }   // faellt schon sicher
            let gelaehmt = f.emp > 2 || f.schaum > 2 || empZiel.contains(&f.u) || schaumZiel.contains(&f.u);
            let wertung = (if gelaehmt { 1_000_000 } else { 0 }) + (f.wert as i64 * 1000) / (f.hp as i64).max(1) - d2(x, y, f.x, f.y) as i64;
            if best.map_or(true, |b| wertung > b.0) { best = Some((wertung, f.u)); }
        }
        if let Some((_, ziel)) = best {
            *zugeteilt.entry(ziel).or_insert(0) += schaden * 8;   // grob: Schaden einiger Schuesse
            befehle.push((u, ziel));
        }
    }
    befehle
}

/// 4. Stellung (Nutzer): ausserhalb der feindlichen Feuerreichweite bleiben, wo es geht -- nicht um jeden Preis.
/// Bedrohung eines Feldes = Summe der Kampfwerte feindlicher Schuetzen (nicht betaeubt), die es erreichen. Eine Einheit
/// weicht nur aus, wenn ein befahrbares Feld bis 2 Kacheln weit deutlich weniger bedroht ist. Liefert Zuege (Einheit, x, y).
#[allow(clippy::too_many_arguments)]
fn stellung(welt: &mut dyn Welt, z: &Zustand, t: i32, sp: usize, eigene: &[Eigen], feinde: &[Feind], tuerme: &[Feind], befehle: &[(Id, Id)]) -> Vec<(Id, i32, i32)> {
    let mut zuege: Vec<(Id, i32, i32)> = Vec::new();
    if PAR(z.spieler).gefechtStellung >= 0.5 {
        let pg = PAR(z.spieler);
        let plus = pg.gefechtReichweitePlus;
        let gw = pg.gefechtBedrohungWert.clamp(0.0, 1.0);
        let turmIds: Vec<Id> = tuerme.iter().map(|f| f.u).collect();
        let schuetzen: Vec<&Feind> = feinde.iter().chain(tuerme.iter()).filter(|f| f.emp <= 2).collect();
        // Bedrohung eines Feldes: je erreichender Schuetze (1 - gw) + gw x Kampfwert, Tuerme mal gefechtTurmGewicht
        let bedrohung = |x: i32, y: i32| -> i32 {
            let mut b = 0.0f64;
            for f in schuetzen.iter() {
                let r = f.rw as f64 + plus;
                if (d2(x, y, f.x, f.y) as f64) <= r * r {
                    let e = (1.0 - gw) + gw * f.wert.max(1) as f64;
                    b += if turmIds.contains(&f.u) { e * pg.gefechtTurmGewicht } else { e };
                }
            }
            (b * 10.0).round() as i32
        };
        let ziele: HashMap<Id, Id> = befehle.iter().cloned().collect();
        let mut belegt: Vec<(i32, i32)> = eigene.iter().map(|e| (e.1, e.2)).collect();
        let gesperrt: HashMap<Id, i32> = STAND.with(|s| s.borrow()[sp].zug.clone());
        let sperre = PAR(z.spieler).gefechtSperre as i32;
        for &(u, x, y, w, rw) in eigene.iter() {
            if gesperrt.get(&u).is_some_and(|&tz| t - tz < sperre) { continue; }
            let jetzt = bedrohung(x, y);
            if jetzt == 0 { continue; }
            let typ = welt.GetType(u);
            let hpMax = welt.GetHitPoints(typ, z.spieler).max(1);
            let beschaedigt = welt.GetDamage(u) as f64 > hpMax as f64 * pg.gefechtRueckzugSchaden;
            let stuetze = w == mapEMP || w == mapStickyfoam;
            let ziel = ziele.get(&u).and_then(|&zz| feinde.iter().chain(tuerme.iter()).find(|f| f.u == zz)).map(|f| (f.x, f.y));
            // Wertung eines Feldes: Bedrohung, Verlust des Ziels, Weg
            let wert = |bx: i32, by: i32, b: i32| -> i32 {
                let mut v = b as f64;
                if let Some((zx, zy)) = ziel { if !beschaedigt && d2(bx, by, zx, zy) > rw * rw { v += if stuetze { pg.gefechtStuetzeStrafe } else { pg.gefechtZielStrafe }; } }
                (v + pg.gefechtWeg * d2(bx, by, x, y) as f64) as i32
            };
            let start = wert(x, y, jetzt);
            let mut best = (start, x, y);
            let aw = pg.gefechtAusweich.max(1.0) as i32;
            for dy in -aw..=aw { for dx in -aw..=aw {
                if dx == 0 && dy == 0 { continue; }
                let (nx, ny) = (x + dx, y + dy);
                if belegt.contains(&(nx, ny)) { continue; }
                if !imFeld(welt, z, nx, ny) { continue; }
                let c = welt.GetCellType(Loc::new(nx, ny));
                if !zelleBefahrbar(welt, c) { continue; }
                let v = wert(nx, ny, bedrohung(nx, ny));
                if v < best.0 { best = (v, nx, ny); }
            } }
            // nur bei deutlicher Verbesserung (ein Viertel der jetzigen Bedrohung) -- sonst bleibt sie und kaempft
            if best.1 != x || best.2 != y {
                if (start - best.0) as f64 >= (jetzt as f64 * pg.gefechtBesser).max(10.0) {
                    belegt.push((best.1, best.2));
                    zuege.push((u, best.1, best.2));
                }
            }
        }
    }
    zuege
}

/// 5. Abstand halten (Nutzer 2026-11-17): wer weiter reicht als sein naechster schussbereiter Gegner, faehrt zurueck,
/// sobald der Gegner auf dessen Reichweite + 1 heran ist -- bis knapp innerhalb der eigenen Reichweite, hoechstens 3 Kacheln.
fn abstandHalten(welt: &mut dyn Welt, z: &Zustand, t: i32, sp: usize, eigene: &[Eigen], feinde: &[Feind], zuege: &mut Vec<(Id, i32, i32)>) {
    if PAR(z.spieler).gefechtAbstand >= 0.5 {
        let sperreA = PAR(z.spieler).gefechtAbstandSperre as i32;
        let gesperrt: HashMap<Id, i32> = STAND.with(|s| s.borrow()[sp].zug.clone());
        let mut belegt: Vec<(i32, i32)> = eigene.iter().map(|e| (e.1, e.2)).chain(zuege.iter().map(|z| (z.1, z.2))).collect();
        for &(u, x, y, w, rw) in eigene.iter() {
            if w == mapEMP || w == mapStickyfoam || zuege.iter().any(|z| z.0 == u) { continue; }
            if gesperrt.get(&u).is_some_and(|&tz| t - tz < sperreA) { continue; }
            // naechster Gegner, der schiessen kann (nicht betaeubt; festgeklebte schiessen weiter, fahren aber nicht)
            let Some(f) = feinde.iter().filter(|f| f.emp <= 2).min_by_key(|f| d2(x, y, f.x, f.y)) else { continue };
            if rw <= f.rw { continue; }
            let d = (d2(x, y, f.x, f.y) as f64).sqrt();
            if d > f.rw as f64 + 1.0 || d < 0.5 { continue; }
            // Zielabstand knapp innerhalb der eigenen Reichweite
            let soll = (rw as f64 - 0.5).max(f.rw as f64 + 1.5);
            let schritt = (soll - d).clamp(1.0, 3.0);
            let (ux, uy) = ((x - f.x) as f64 / d, (y - f.y) as f64 / d);
            // geradeaus zurueck, sonst schraeg links/rechts
            let mut gewaehlt: Option<(i32, i32)> = None;
            for winkel in [0.0f64, 0.5, -0.5, 1.0, -1.0] {
                let (c, s) = (winkel.cos(), winkel.sin());
                let (rx, ry) = (ux * c - uy * s, ux * s + uy * c);
                let (nx, ny) = ((x as f64 + rx * schritt).round() as i32, (y as f64 + ry * schritt).round() as i32);
                if (nx, ny) == (x, y) || belegt.contains(&(nx, ny)) { continue; }
                if !imFeld(welt, z, nx, ny) { continue; }
                let c = welt.GetCellType(Loc::new(nx, ny));
                if !zelleBefahrbar(welt, c) { continue; }
                // nicht in die Reichweite eines anderen Gegners hinein
                if feinde.iter().any(|g| g.u != f.u && g.emp <= 2 && (d2(nx, ny, g.x, g.y) as f64) < ((g.rw as f64) + 0.5).powi(2)) { continue; }
                gewaehlt = Some((nx, ny)); break;
            }
            if let Some((nx, ny)) = gewaehlt { belegt.push((nx, ny)); zuege.push((u, nx, ny)); }
        }
    }
}
