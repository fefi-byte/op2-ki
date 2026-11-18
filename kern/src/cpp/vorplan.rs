//! Erweiterung der Rust-Fassung (nicht in der C++-KI): Bebauungsplan.
//! Bisher sucht die KI den Platz erst, wenn sie ein Gebaeude bauen will -- kleine Gebaeude belegen dann die Flaechen, die
//! spaeter Fahrzeugfabrik oder Grosslabor fehlen (mp2_08: Eden an der Kartenecke ohne Fabrikplatz). Der Plan rechnet den
//! Bedarf fuer den Horizont voraus (Bevoelkerung -> Wohnraum, Nahrung; Strom der geplanten Gebaeude; Fabriken, Labore,
//! Dienste), legt die Plaetze vorab fest (grosse zuerst, nah, kurze Roehre, dicht gepackt, alle Gebaeude bleiben
//! erreichbar) und haelt sie fuer ihren Typ frei. platzWaehlen nimmt zuerst einen Platz aus dem Plan.
//! Building plan: forecast demand over the horizon, reserve plots up front (large first), keep them for their type.
use std::cell::RefCell;

use super::*;

fn ix(t: i32) -> usize { t as usize }
fn snp(s: String, n: usize) -> String { s.chars().take(n).collect() }

/// Ein reservierter Platz (Editor-Kacheln, Grundflaeche oben links).
#[derive(Clone, Copy, Debug, Default)]
pub struct PlanPlatz { pub typ: i32, pub x0: i32, pub y0: i32, pub w: i32, pub h: i32 }

#[derive(Clone, Debug, Default)]
pub struct Plan { pub plaetze: Vec<PlanPlatz>, pub naechster: i32, pub gemeldet: i32 }

thread_local! {
    pub static PLAN: RefCell<[Plan; 8]> = RefCell::new(Default::default());
    /// Platzsuche ohne Reservierungen (Rueckfall, wenn sonst kein Platz frei ist)
    static AUSSETZEN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
pub fn ausgesetzt() -> bool { AUSSETZEN.with(|a| a.get()) }
pub fn aussetzen(an: bool) { AUSSETZEN.with(|a| a.set(an)); }
pub fn zuruecksetzen_vorplan() { PLAN.with(|p| *p.borrow_mut() = Default::default()); }

/// Die reservierten Plaetze eines Spielers (fuer hindernisse()).
pub fn reserviert(z: &Zustand) -> Vec<PlanPlatz> {
    if ausgesetzt() { return Vec::new(); }
    PLAN.with(|p| p.borrow()[(z.spieler & 7) as usize].plaetze.clone())
}

/// Bedarf im Horizont: (Typ, Anzahl zusaetzlich). Grosse Gebaeude zuerst (sie sind am schwersten unterzubringen).
fn bedarf(welt: &mut dyn Welt, z: &Zustand, l: &Lage) -> Vec<(i32, i32)> {
    let sp = z.spieler;
    let p = PAR(sp);
    let hat = |l: &Lage, t: i32| l.geb[ix(t)] + l.imBau[ix(t)];
    let mut b: Vec<(i32, i32)> = Vec::new();
    // Fabriken und Labore: was die Partie sicher braucht
    let vf = hat(l, mapVehicleFactory);
    if vf < 2 { b.push((mapVehicleFactory, 2 - vf)); }
    if hat(l, mapAdvancedLab) < 1 { b.push((mapAdvancedLab, 1)); }
    let gs = hat(l, mapCommonOreSmelter);
    let gm = l.geb[ix(mapCommonOreMine)];
    let schmelzen = ((gm.max(1) as f64) * p.schmelzenJeMine).ceil() as i32;
    if gs < schmelzen.min(3) { b.push((mapCommonOreSmelter, schmelzen.min(3) - gs)); }
    if hat(l, mapStandardLab) < 1 { b.push((mapStandardLab, 1)); }
    if vf > 0 && hat(l, mapGarage) < 1 { b.push((mapGarage, 1)); }
    // Bevoelkerung im Horizont -> Wohnraum und Nahrung
    let menschen = l.menschen.max(1);
    let ziel = (menschen as f64 * (1.0 + p.vorplanWachstum)).ceil() as i32;
    let wt = super::planung::wohnTyp(welt, z, l);
    let kap = welt.GetProductionCapacity(wt, sp).max(1);
    let wohn = l.wohnraum + l.imBau[ix(mapResidence)] * welt.GetProductionCapacity(mapResidence, sp);
    if ziel > wohn { b.push((wt, (ziel - wohn + kap - 1) / kap)); }
    let agri = hat(l, mapAgridome);
    let agriZiel = if l.geb[ix(mapAgridome)] > 0 { ((l.geb[ix(mapAgridome)] as f64) * ziel as f64 / menschen as f64).ceil() as i32 } else { 1 };
    let agriZiel = if l.nahrungNetto < 0 { agriZiel.max(agri + 1) } else { agriZiel };
    if agriZiel > agri { b.push((mapAgridome, agriZiel - agri)); }
    // Dienste der Kolonie (je eines)
    for t in [mapNursery, mapUniversity, mapMedicalCenter, mapDIRT] {
        if hat(l, t) < 1 && welt.GetOreCost(t, sp) > 0 { b.push((t, 1)); }
    }
    // Strom fuer alles Geplante
    let mut strom = 0;
    for &(t, n) in b.iter() { strom += welt.GetPowerRequired(t, sp).max(0) * n; }
    let tk = welt.GetProductionCapacity(mapTokamak, sp).max(1);
    let fehlt = strom - l.stromFrei;
    if fehlt > 0 { b.push((mapTokamak, (fehlt + tk - 1) / tk)); }
    // Groesste Grundflaeche zuerst
    b.sort_by_key(|&(t, _)| { let g = grundflaeche(welt, t); -(g.w * g.h) });
    b
}

/// Erreichbarkeit: von der Basismitte ueber befahrbare Kacheln ausserhalb aller Grundflaechen -- erreicht die Suche eine
/// Seitenkachel jedes Gebaeudes und Platzes? (Docks von Fabrik und Schmelze, Zufahrt fuer ConVecs.)
fn alleErreichbar(welt: &mut dyn Welt, z: &Zustand, hin: &[Rechteck], nh: i32, ab: usize) -> bool {
    let B = z.breite.min(256); let H = z.hoehe.min(256);
    let n = (B * H).max(0) as usize;
    let mut zu = vec![false; n];
    let frei = |welt: &mut dyn Welt, x: i32, y: i32| -> bool {
        if x < 0 || y < 0 || x >= B || y >= H || !aufKarte(welt, z, x, y, 1) { return false; }
        let c = zelle(welt, z, x, y);
        zelleBefahrbar(welt, c) && !super::bau::belegt(welt, hin, nh, x, y)
    };
    // Start: freie Kacheln rings um die Kommandozentrale (die Mitte selbst ist Gebaeude)
    let mut q: Vec<(i32, i32)> = Vec::new();
    for dy in -4..=4 { for dx in -4..=4 {
        let (x, y) = (z.bx + dx, z.by + dy);
        if frei(welt, x, y) && !zu[(y * B + x) as usize] { zu[(y * B + x) as usize] = true; q.push((x, y)); }
    } }
    let mut k = 0;
    while k < q.len() {
        let (x, y) = q[k]; k += 1;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= B || ny >= H { continue; }
            let i = (ny * B + nx) as usize;
            if zu[i] || !frei(welt, nx, ny) { continue; }
            zu[i] = true; q.push((nx, ny));
        }
    }
    for j in ab..nh as usize {
        let r = &hin[j];
        if r.w * r.h <= 1 { continue; } // Tuerme, Lager-Stellplaetze u.ae.
        let mut ok = false;
        for y in (r.y0 - 1)..=(r.y0 + r.h) { for x in (r.x0 - 1)..=(r.x0 + r.w) {
            if !ok && x >= 0 && y >= 0 && x < B && y < H && super::bau::anGrundflaeche(welt, r, x, y) && zu[(y * B + x) as usize] { ok = true; }
        } }
        if !ok { return false; }
    }
    true
}

/// Dichte: belegte Kacheln im Ring zwei Kacheln um die Grundflaeche (hinter der freien Randkachel) -- je mehr, desto
/// weniger Restflaeche bleibt ungenutzt.
fn dichte(welt: &mut dyn Welt, hin: &[Rechteck], nh: i32, x0: i32, y0: i32, w: i32, h: i32) -> i32 {
    let mut n = 0;
    for y in (y0 - 2)..=(y0 + h + 1) { for x in (x0 - 2)..=(x0 + w + 1) {
        let ring = x == x0 - 2 || x == x0 + w + 1 || y == y0 - 2 || y == y0 + h + 1;
        if ring && super::bau::belegt(welt, hin, nh, x, y) { n += 1; }
    } }
    n
}

/// Plan neu aufstellen (alle 2000 Ticks; der Plan ist billig, nichts ist gebaut).
/// Bebauungsplan (alle 2000 Ticks): fuer jeden Bedarf den besten Platz, der alle Zufahrten offen laesst (kurze Roehre,
/// nah an der Mitte, dicht, Explosionsabstand). Rust 2026-11-18 aufgeteilt (Verhalten gleich).
pub fn planen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    let sp = (z.spieler & 7) as usize;
    let t = welt.Tick();
    if t < PLAN.with(|p| p.borrow()[sp].naechster) { return; }
    PLAN.with(|p| { let mut p = p.borrow_mut(); p[sp].naechster = t + 2000; p[sp].plaetze.clear(); });
    let pp = PAR(z.spieler);
    let r = pp.vorplanRadius as i32;
    let mut hin = [Rechteck::default(); 160];
    let mut nh = hindernisse(welt, z, &mut hin, 150);    // ohne Plan (gerade geleert)
    let abst = super::bau::netzAbstand(welt, z, &hin, nh);
    // Ist schon heute ein Gebaeude zugebaut, pruefen die neuen Plaetze nur sich und die anderen neuen.
    let ab = if alleErreichbar(welt, z, &hin, nh, 0) { 0 } else { nh as usize };
    let bb = z.breite.min(256);
    let ziele = netzZiele(welt, z);
    let mut neu: Vec<PlanPlatz> = Vec::new();
    let mut fehlt: Vec<i32> = Vec::new();
    for (typ, anzahl) in bedarf(welt, z, l) {
        let g = grundflaeche(welt, typ);
        for _ in 0..anzahl {
            if nh as usize >= hin.len() { break; }
            // Kandidaten nach Wertung, der beste, der alle Zufahrten offen laesst
            let mut kand = kandidaten(welt, z, typ, g, r, bb, &abst, &hin, nh, &neu);
            if hatStutzen(welt, typ) && typ != mapCommandCenter && !ziele.is_empty() { kand = roehreGenau(welt, z, typ, g, kand, &ziele, &hin, nh); }
            let mut gesetzt = false;
            for &(_, x0, y0) in kand.iter().take(12) {
                hin[nh as usize] = Rechteck { x0, y0, w: g.w, h: g.h, stutzen: false };
                if alleErreichbar(welt, z, &hin, nh + 1, ab) {
                    nh += 1;
                    neu.push(PlanPlatz { typ, x0, y0, w: g.w, h: g.h });
                    gesetzt = true;
                    break;
                }
            }
            if !gesetzt { fehlt.push(typ); break; }
        }
    }
    planMelden(welt, z, sp, t, &neu, &fehlt);
    PLAN.with(|p| p.borrow_mut()[sp].plaetze = neu);
}

/// Ziele der Roehre: Netzgebaeude (wie anschluss())
fn netzZiele(welt: &mut dyn Welt, z: &mut Zustand) -> Vec<Rechteck> {
    let netz = super::bau::netzKacheln(welt, z);
    let mut ziele: Vec<Rechteck> = Vec::new();
    for &ty in GEBAEUDE.iter() {
        if !hatStutzen(welt, ty) { continue; }
        for u in welt.PlayerBuildingEnum(z.spieler, ty) {
            let lo = welt.Location(u);
            let (ex, ey) = (edX(welt, z, lo), edY(welt, z, lo));
            let r = rechteckUm(welt, ty, ex, ey);
            if ty == mapCommandCenter || super::bau::imNetz(z, &netz, &r) { if ziele.len() < 64 { ziele.push(r); } }
        }
    }
    ziele
}

/// Freie Plaetze im Radius r nach Wertung: Abstand zur Mitte + geschaetzte Roehre - Dichte + Explosionsabstand
#[allow(clippy::too_many_arguments)]
fn kandidaten(welt: &mut dyn Welt, z: &mut Zustand, typ: i32, g: Groesse, r: i32, bb: i32, abst: &[i32], hin: &[Rechteck], nh: i32, neu: &[PlanPlatz]) -> Vec<(i32, i32, i32)> {
    let pp = PAR(z.spieler);
    let (kachel, dichteW) = (pp.roehreKachel, pp.vorplanDicht);
    let mut kand: Vec<(i32, i32, i32)> = Vec::new();
    for dy in -r..=r { for dx in -r..=r {
        let d2 = dx * dx + dy * dy;
        if d2 > r * r { continue; }
        let (x0, y0) = (z.bx + dx - g.w / 2, z.by + dy - g.h / 2);
        let mut gr: &'static str = "";
        if !platzFrei(welt, z, x0, y0, g.w, g.h, hin, nh, &mut gr) { continue; }
        // Roehre: kuerzester Anschluss ueber eine Seitenkachel (am Stutzen eines Netzgebaeudes: 1)
        let mut roehre = 0;
        if hatStutzen(welt, typ) && typ != mapCommandCenter {
            let rr = Rechteck { x0, y0, w: g.w, h: g.h, stutzen: true };
            let mut m = i32::MAX;
            for y in (y0 - 1)..=(y0 + g.h) { for x in (x0 - 1)..=(x0 + g.w) {
                if x < 0 || y < 0 || x >= bb || (y * bb + x) as usize >= abst.len() { continue; }
                if !super::bau::anGrundflaeche(welt, &rr, x, y) { continue; }
                let v = abst[(y * bb + x) as usize];
                if v >= 0 && v < m { m = v; }
            } }
            if m == i32::MAX { continue; }
            roehre = m;
        }
        let mut w = d2 + (roehre as f64 * kachel) as i32 - (dichte(welt, hin, nh, x0, y0, g.w, g.h) as f64 * dichteW) as i32;
        w += explosionsStrafe(welt, z, typ, x0, y0, g.w, g.h);
        // geplante Tokamaks / Grosslabore untereinander: auch 3 Felder Luft
        let expl = |t: i32| t == mapTokamak || t == mapAdvancedLab;
        for q in neu.iter() {
            if !expl(typ) && !expl(q.typ) { continue; }
            let lx = (q.x0 - (x0 + g.w)).max(x0 - (q.x0 + q.w)).max(0);
            let ly = (q.y0 - (y0 + g.h)).max(y0 - (q.y0 + q.h)).max(0);
            let lu = lx.max(ly);
            if lu < 3 { w += (3 - lu) * 300; }
        }
        kand.push((w, x0, y0));
    } }
    kand.sort_unstable();
    kand
}

/// Echte Roehre (Wegsuche wie anschluss(), neue Kacheln) fuer die besten 16 Kandidaten statt der Schaetzung
#[allow(clippy::too_many_arguments)]
fn roehreGenau(welt: &mut dyn Welt, z: &mut Zustand, typ: i32, g: Groesse, mut kand: Vec<(i32, i32, i32)>, ziele: &[Rechteck], hin: &[Rechteck], nh: i32) -> Vec<(i32, i32, i32)> {
    let pp = PAR(z.spieler);
    let (kachel, dichteW) = (pp.roehreKachel, pp.vorplanDicht);
    let mut wx = [0i16; MAX_WEG as usize];
    let mut wy = [0i16; MAX_WEG as usize];
    let mut genau: Vec<(i32, i32, i32)> = Vec::new();
    for &(w, x0, y0) in kand.iter().take(16) {
        let rr = Rechteck { x0, y0, w: g.w, h: g.h, stutzen: true };
        let n = super::bau::wegSuchen(welt, z, &rr, ziele, ziele.len() as i32, hin, nh, &mut wx, &mut wy, false);
        if n <= 0 { continue; }
        let mut neuK = 0;
        for i in 0..n as usize { let c = zelle(welt, z, wx[i] as i32, wy[i] as i32); if !zelleRoehre(welt, c) { neuK += 1; } }
        let _ = w;   // Wertung neu: Abstand, Dichte, Explosionsabstand, echte Roehre (die Luecke am Stutzen zaehlt nicht)
        let d2 = (x0 + g.w / 2 - z.bx).pow(2) + (y0 + g.h / 2 - z.by).pow(2);
        let wg = d2 - (dichte(welt, hin, nh, x0, y0, g.w, g.h) as f64 * dichteW) as i32
            + explosionsStrafe(welt, z, typ, x0, y0, g.w, g.h) + (neuK.max(1) as f64 * kachel) as i32 - kachel as i32;
        genau.push((wg, x0, y0));
    }
    if !genau.is_empty() { genau.sort_unstable(); kand = genau; }
    kand
}

/// Plan in die Spur (alle 10000 Ticks oder wenn ein Platz fehlt)
fn planMelden(welt: &mut dyn Welt, z: &mut Zustand, sp: usize, t: i32, neu: &[PlanPlatz], fehlt: &[i32]) {
    let gemeldet = PLAN.with(|p| p.borrow()[sp].gemeldet);
    if gemeldet == 0 || t - gemeldet >= 10000 || !fehlt.is_empty() && selten(welt, z, 27, 4000) {
        let mut txt = String::new();
        for q in neu.iter() { txt.push_str(&format!("{} {},{}; ", name(welt, q.typ), q.x0, q.y0)); }
        let mut f = String::new();
        for &ft in fehlt.iter() { f.push_str(&format!("{} ", name(welt, ft))); }
        melde(welt, z, "plan", mapNone, z.bx, z.by, &snp(format!("Bebauungsplan: {} Plaetze reserviert ({}){}", neu.len(), txt,
              if f.is_empty() { String::new() } else { format!(" -- kein Platz fuer: {}", f) }), 400));
        PLAN.with(|p| p.borrow_mut()[sp].gemeldet = t);
    }
}

/// Platz aus dem Plan fuer typ: zuerst einer fuer diesen Typ, sonst einer gleicher Grundflaeche. Prueft ihn wie jeden
/// Platz (ohne sich selbst als Hindernis) und gibt ihn frei. Liefert (x0, y0).
pub fn ausPlan(welt: &mut dyn Welt, z: &Zustand, typ: i32) -> Option<(i32, i32)> {
    let sp = (z.spieler & 7) as usize;
    let g = grundflaeche(welt, typ);
    let liste = PLAN.with(|p| p.borrow()[sp].plaetze.clone());
    let mut wahl: Vec<(i32, usize)> = Vec::new();
    for (i, q) in liste.iter().enumerate() {
        if q.w != g.w || q.h != g.h { continue; }
        // grosse Plaetze (Fabrik, Schmelze, Grosslabor) nur fuer ihren Typ -- kleine teilen sich gleiche Grundflaechen
        if q.typ != typ && q.w * q.h > 6 { continue; }
        let d = (q.x0 + q.w / 2 - z.bx).pow(2) + (q.y0 + q.h / 2 - z.by).pow(2);
        wahl.push((if q.typ == typ { 0 } else { 1 << 20 } + d, i));
    }
    wahl.sort_unstable();
    let mut hin = [Rechteck::default(); 160];
    for &(_, i) in wahl.iter() {
        let q = liste[i];
        // Hindernisse ohne diesen Platz
        let nh = hindernisse(welt, z, &mut hin, 160);
        let mut k = 0usize;
        for j in 0..nh as usize {
            let r = hin[j];
            if r.x0 == q.x0 && r.y0 == q.y0 && r.w == q.w && r.h == q.h { continue; }
            hin[k] = r; k += 1;
        }
        let mut gr: &'static str = "";
        if !platzFrei(welt, z, q.x0, q.y0, g.w, g.h, &hin, k as i32, &mut gr) { continue; }
        PLAN.with(|p| { p.borrow_mut()[sp].plaetze.remove(i); });
        return Some((q.x0, q.y0));
    }
    None
}

/// Plan beim naechsten Schritt neu aufstellen (ein Gebaeude hat sich ausserhalb des Plans einen Platz genommen).
pub fn neuPlanen(z: &Zustand) { PLAN.with(|p| p.borrow_mut()[(z.spieler & 7) as usize].naechster = 0); }
