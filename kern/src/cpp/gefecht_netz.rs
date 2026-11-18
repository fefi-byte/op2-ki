//! Lernbare Gefechtsfuehrung (Rust): fuer jede eigene Einheit im Nahkampf werden alle Moeglichkeiten
//! bewertet -- jedes Ziel in Reichweite (Angriff) und jedes Feld bis 2 Kacheln (Stellung, das eigene Feld = halten) -- und
//! die beste ausgefuehrt. Die Bewertung ist ein kleines Netz: linearer Teil (Zwischenstufe "lernbare Formel") plus eine
//! versteckte Schicht (tanh). Gewichte lernt die Gefechts-Arena per Evolutionsstrategie (op2ki-sim --gefecht-lernen).
//! Learnable close-combat control: score every option (attack a target in range, move to a tile) with a small net.
use std::cell::RefCell;
use std::collections::HashMap;

use super::*;

/// Merkmale je (Einheit, Moeglichkeit). Reihenfolge = Bedeutung der Gewichte, nicht aendern ohne neues Training.
pub const M: usize = 24;

#[derive(Clone, Debug)]
pub struct Netz {
    pub versteckt: usize,
    /// linear: M; versteckt: H x M (Eingang), H (Ausgang)
    pub w: Vec<f32>,
}

impl Netz {
    pub fn anzahl(versteckt: usize) -> usize { M + versteckt * M + versteckt }
    /// Startwerte: lineare Handformel (etwa wie die Handsteuerung), versteckte Schicht klein und Ausgang 0.
    pub fn start(versteckt: usize, seed: u64) -> Netz {
        let mut w = vec![0f32; Netz::anzahl(versteckt)];
        let lin: [(usize, f32); 15] = [(6, -1.0), (7, -0.5), (8, 0.5), (11, -0.3), (12, -0.05), (13, 0.2), (14, -1.0), (15, -0.3),
            (17, -2.0), (18, 1.0), (19, -0.2), (20, 0.1), (21, 1.0), (22, 0.5), (23, 2.0)];
        for (i, v) in lin { w[i] = v; }
        let mut s = seed | 1;
        for k in M..M + versteckt * M {
            s ^= s << 13; s ^= s >> 7; s ^= s << 17;
            w[k] = ((s % 2001) as f32 / 1000.0 - 1.0) * 0.3;
        }
        Netz { versteckt, w }
    }
    pub fn bewerte(&self, x: &[f32; M]) -> f32 {
        let mut y = 0.0f32;
        for i in 0..M { y += self.w[i] * x[i]; }
        let h = self.versteckt;
        for j in 0..h {
            let mut a = 0.0f32;
            let o = M + j * M;
            for i in 0..M { a += self.w[o + i] * x[i]; }
            y += self.w[M + h * M + j] * a.tanh();
        }
        y
    }
    pub fn zu_text(&self) -> String {
        let mut s = format!("versteckt={}\n", self.versteckt);
        for v in &self.w { s.push_str(&format!("{:.6}\n", v)); }
        s
    }
    pub fn aus_text(t: &str) -> Option<Netz> {
        let mut z = t.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#'));
        let h: usize = z.next()?.strip_prefix("versteckt=")?.trim().parse().ok()?;
        let w: Vec<f32> = z.filter_map(|l| l.trim().parse().ok()).collect();
        if w.len() != Netz::anzahl(h) { return None; }
        Some(Netz { versteckt: h, w })
    }
}

thread_local! {
    /// Netz je Spieler (die Arena setzt es direkt, die KI laedt es aus KI-Gefecht-Netz.txt)
    pub static NETZE: RefCell<[Option<Netz>; 8]> = RefCell::new(Default::default());
    static ZUG: RefCell<[HashMap<Id, i32>; 8]> = RefCell::new(Default::default());
    static ZIEL: RefCell<[HashMap<Id, Id>; 8]> = RefCell::new(Default::default());
    static GELADEN: RefCell<[bool; 8]> = const { RefCell::new([false; 8]) };
}
pub fn setze(sp: i32, n: Option<Netz>) { NETZE.with(|x| x.borrow_mut()[(sp & 7) as usize] = n); }
pub fn zuruecksetzen() {
    NETZE.with(|x| *x.borrow_mut() = Default::default());
    ZUG.with(|x| *x.borrow_mut() = Default::default());
    ZIEL.with(|x| *x.borrow_mut() = Default::default());
    GELADEN.with(|x| *x.borrow_mut() = [false; 8]);
}

/// Das gelernte Netz (gefecht_netz.txt, 16 versteckte Knoten); eine Datei KI-Gefecht-Netz.txt im Wissensordner geht vor.
const EINGEBAUT: &str = include_str!("gefecht_netz.txt");

/// Netz fuer den Spieler: einmal laden, aus KI-Gefecht-Netz.txt (Wissensordner) oder sonst das eingebaute.
pub fn netz(welt: &mut dyn Welt, sp: i32) -> Option<Netz> {
    let i = (sp & 7) as usize;
    if let Some(n) = NETZE.with(|x| x.borrow()[i].clone()) { return Some(n); }
    if !GELADEN.with(|g| g.borrow()[i]) {
        GELADEN.with(|g| g.borrow_mut()[i] = true);
        let mut pfad = String::new();
        wissenPfad(welt, &mut pfad, 600, "KI-Gefecht-Netz.txt");
        let t = welt.dateiLesen(&pfad).unwrap_or_else(|| EINGEBAUT.to_string());
        if let Some(n) = Netz::aus_text(&t) { setze(sp, Some(n.clone())); return Some(n); }
    }
    None
}

#[derive(Clone, Copy)]
struct F { u: Id, x: i32, y: i32, wert: i32, hp: i32, hpMax: i32, emp: i32, schaum: i32, rw: i32, turm: bool }

/// Eigene Einheit im Gefecht: Id, x, y, Waffe, Reichweite, Wert, HP-Anteil
type Eigen = (Id, i32, i32, i32, i32, i32, f32);
fn d2(ax: i32, ay: i32, bx: i32, by: i32) -> i32 { (ax - bx).pow(2) + (ay - by).pow(2) }
/// Bedrohung eines Feldes: Kampfwert der schussbereiten Feinde, die es erreichen (Reichweite + 1)
fn bedrohung(schuetzen: &[&F], x: i32, y: i32) -> f32 {
    let mut b = 0f32;
    for f in schuetzen.iter() { let r = f.rw + 1; if d2(x, y, f.x, f.y) <= r * r { b += f.wert.max(1) as f32; } }
    b
}
/// Feindliche Fahrzeuge im Umkreis von 2 Kacheln (Haufen, fuer EMP)
fn haufen(feinde: &[F], x: i32, y: i32) -> f32 { feinde.iter().filter(|f| !f.turm && d2(x, y, f.x, f.y) <= 4).count() as f32 }

/// Fuehrt das Gefecht eines Spielers mit dem Netz (ein Aufruf je AIProc): je eigene Einheit bewertet das Netz alle
/// Angriffsziele in Reichweite + 2 und alle Felder bis 2 Kacheln (halten/ausweichen) und nimmt das Beste.
/// Rust 2026-11-18 aufgeteilt (Verhalten gleich).
pub fn fuehren(welt: &mut dyn Welt, z: &Zustand, n: &Netz, radius: i32, sperre: i32) {
    let sp = z.spieler;
    let si = (sp & 7) as usize;
    let t = welt.Tick();
    let feinde = feindeErfassen(welt, sp);
    if feinde.is_empty() { return; }
    let eigene = eigeneErfassen(welt, sp, &feinde, radius);
    if eigene.is_empty() { return; }
    let k = eigene.len() as i32;
    let (mx, my) = (eigene.iter().map(|e| e.1).sum::<i32>() / k, eigene.iter().map(|e| e.2).sum::<i32>() / k);
    let schuetzen: Vec<&F> = feinde.iter().filter(|f| f.emp <= 2).collect();
    let mut bedient: HashMap<Id, f32> = HashMap::new();
    let mut belegt: Vec<(i32, i32)> = eigene.iter().map(|e| (e.1, e.2)).collect();
    let alt: HashMap<Id, Id> = ZIEL.with(|z| z.borrow()[si].clone());
    let gesperrt: HashMap<Id, i32> = ZUG.with(|z| z.borrow()[si].clone());
    let mut angriffe: Vec<(Id, Id)> = Vec::new();
    let mut zuege: Vec<(Id, i32, i32)> = Vec::new();
    for e in eigene.iter() {
        einheitEntscheiden(welt, n, sp, t, sperre, e, &feinde, &schuetzen, (mx, my), &gesperrt, &mut bedient, &mut belegt, &mut angriffe, &mut zuege);
    }
    ZIEL.with(|zz| {
        let mut zz = zz.borrow_mut();
        let m = &mut zz[si];
        for &(u, ziel) in angriffe.iter() {
            if alt.get(&u) == Some(&ziel) { continue; }
            welt.DoAttack(u, ziel);
            m.insert(u, ziel);
        }
        for &(u, _, _) in zuege.iter() { m.remove(&u); }
    });
    ZUG.with(|zg| {
        let mut zg = zg.borrow_mut();
        let m = &mut zg[si];
        m.retain(|_, v| t - *v < 2000);
        for &(u, x, y) in zuege.iter() { welt.DoMove(u, Loc::new(x, y)); m.insert(u, t); }
    });
}

/// Feinde: Kampffahrzeuge und Tuerme samt Trefferpunkten, EMP-/Schaumzeit und Reichweite
fn feindeErfassen(welt: &mut dyn Welt, sp: i32) -> Vec<F> {
    let mut feinde: Vec<F> = Vec::new();
    for p in 0..welt.NoPlayers() {
        if p == sp || welt.IsAlliedTo(sp, p) { continue; }
        for u in welt.PlayerVehicleEnum(p) {
            if !welt.IsLive(u) { continue; }
            let w = kampfWert(welt, u);
            if w <= 0 { continue; }
            let l = welt.Location(u);
            let ty = welt.GetType(u);
            let hpMax = welt.GetHitPoints(ty, p).max(1);
            feinde.push(F { u, x: l.x, y: l.y, wert: w, hp: (hpMax - welt.GetDamage(u)).max(1), hpMax, emp: welt.zeitEMP(u), schaum: welt.zeitSchaum(u),
                            rw: welt.GetWeaponRange(welt.GetWeapon(u), p).max(1), turm: false });
        }
        for u in welt.PlayerBuildingEnum(p, mapGuardPost) {
            if !welt.IsLive(u) { continue; }
            let l = welt.Location(u);
            let hpMax = welt.GetHitPoints(mapGuardPost, p).max(1);
            feinde.push(F { u, x: l.x, y: l.y, wert: kampfWert(welt, u), hp: (hpMax - welt.GetDamage(u)).max(1), hpMax, emp: 0, schaum: 0,
                            rw: welt.GetWeaponRange(welt.GetWeapon(u), p).max(1), turm: true });
        }
    }
    feinde
}

/// Eigene Kampfeinheiten (nicht betaeubt) im Gefechtsradius eines Feindes
fn eigeneErfassen(welt: &mut dyn Welt, sp: i32, feinde: &[F], radius: i32) -> Vec<Eigen> {
    let mut eigene: Vec<(Id, i32, i32, i32, i32, i32, f32)> = Vec::new();   // u, x, y, Waffe, Reichweite, Wert, HP-Anteil
    for u in welt.PlayerVehicleEnum(sp) {
        if !welt.IsLive(u) || !kaempfer(welt, u) || welt.isEMPed(u) { continue; }
        let l = welt.Location(u);
        if !feinde.iter().any(|f| d2(f.x, f.y, l.x, l.y) <= radius * radius) { continue; }
        let w = welt.GetWeapon(u);
        let ty = welt.GetType(u);
        let hpMax = welt.GetHitPoints(ty, sp).max(1);
        eigene.push((u, l.x, l.y, w, welt.GetWeaponRange(w, sp).max(1), kampfWert(welt, u).max(1), (hpMax - welt.GetDamage(u)) as f32 / hpMax as f32));
    }
    eigene
}

/// Eine Einheit: bestes Ziel oder bestes Feld nach dem Netz
#[allow(clippy::too_many_arguments)]
fn einheitEntscheiden(welt: &mut dyn Welt, n: &Netz, sp: i32, t: i32, sperre: i32, e: &Eigen, feinde: &[F], schuetzen: &[&F], (mx, my): (i32, i32),
                      gesperrt: &HashMap<Id, i32>, bedient: &mut HashMap<Id, f32>, belegt: &mut Vec<(i32, i32)>,
                      angriffe: &mut Vec<(Id, Id)>, zuege: &mut Vec<(Id, i32, i32)>) {
    let &(u, x, y, w, rw, eigenWert, hpA) = e;
    let (istE, istS) = (w == mapEMP, w == mapStickyfoam);
    let istD = !istE && !istS;
    let bJetzt = (bedrohung(schuetzen, x, y) / eigenWert as f32).min(3.0);
    let mut basis = [0f32; M];
    basis[0] = 1.0; basis[1] = istE as i32 as f32; basis[2] = istS as i32 as f32; basis[3] = istD as i32 as f32;
    basis[4] = hpA; basis[5] = bJetzt;
    let mut best: (f32, i32, Id, i32, i32) = (f32::MIN, 0, 0, x, y);   // Wertung, Art (1 Angriff, 2 Zug), Ziel, Feld
    // Angriffe
    let reich = rw + 2;
    let mut bestesZiel: Option<(i32, i32)> = None;
    for f in feinde.iter() {
        let d = d2(x, y, f.x, f.y);
        if d > reich * reich { continue; }
        if (istE || istS) && f.turm { continue; }
        let mut m = basis;
        m[6] = 0.0;
        m[7] = f.hp as f32 / f.hpMax as f32;
        m[8] = (f.wert as f32 / eigenWert as f32).min(3.0);
        m[9] = (f.emp > 2) as i32 as f32;
        m[10] = (f.schaum > 2) as i32 as f32;
        m[11] = ((d as f32).sqrt() / rw as f32).min(2.0);
        m[12] = (d2(mx, my, f.x, f.y) as f32).sqrt() / 10.0;
        m[13] = haufen(feinde, f.x, f.y) / 5.0;
        m[14] = (bedient.get(&f.u).copied().unwrap_or(0.0) / f.hp as f32).min(2.0);
        m[15] = f.turm as i32 as f32;
        m[16] = (d <= (f.rw + 1) * (f.rw + 1)) as i32 as f32;
        m[21] = m[1] * m[13]; m[22] = m[2] * m[12]; m[23] = m[3] * (m[9].max(m[10]));
        let v = n.bewerte(&m);
        if v > best.0 { best = (v, 1, f.u, x, y); bestesZiel = Some((f.x, f.y)); }
    }
    // Felder (eigenes = halten); bei Sperre nur halten
    let darf = gesperrt.get(&u).map_or(true, |&tz| t - tz >= sperre);
    let zielXY = bestesZiel.or_else(|| feinde.iter().min_by_key(|f| d2(x, y, f.x, f.y)).map(|f| (f.x, f.y)));
    for dy in -2..=2i32 { for dx in -2..=2i32 {
        if (dx != 0 || dy != 0) && !darf { continue; }
        let (nx, ny) = (x + dx, y + dy);
        if (dx != 0 || dy != 0) && belegt.contains(&(nx, ny)) { continue; }
        if dx != 0 || dy != 0 { let c = welt.GetCellType(Loc::new(nx, ny)); if !zelleBefahrbar(welt, c) { continue; } }
        let mut m = basis;
        m[6] = 1.0;
        m[17] = (bedrohung(schuetzen, nx, ny) / eigenWert as f32).min(3.0);
        m[18] = zielXY.map_or(0.0, |(zx, zy)| (d2(nx, ny, zx, zy) <= rw * rw) as i32 as f32);
        m[19] = ((dx * dx + dy * dy) as f32).sqrt() / 2.0;
        let nf = feinde.iter().map(|f| d2(nx, ny, f.x, f.y)).min().unwrap_or(10000);
        m[20] = ((nf as f32).sqrt() / rw as f32).min(3.0);
        let v = n.bewerte(&m);
        if v > best.0 { best = (v, 2, 0, nx, ny); }
    }}
    match best.1 {
        1 => {
            let f = feinde.iter().find(|f| f.u == best.2).unwrap();
            let schaden = (welt.GetPenetrationDamage(w, sp).max(1) + welt.GetConcussionDamage(w, sp) / 2) as f32 * 8.0;
            *bedient.entry(f.u).or_insert(0.0) += schaden;
            angriffe.push((u, best.2));
        }
        2 if best.3 != x || best.4 != y => { belegt.push((best.3, best.4)); zuege.push((u, best.3, best.4)); }
        _ => {}
    }
}
