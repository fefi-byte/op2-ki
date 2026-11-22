//! Mauern (Rust): aufgezeichnete, noch nicht gebaute Mauerkacheln merken und Mauerlinien (PAR mauerLinie).
//! Die BuildingGroup baut eine aufgezeichnete Mauer erst spaeter; ohne Merkliste plante die KI dort einen Wachturm, und
//! der Bau raeumte die Mauer wieder ab (Nutzer 2026-11-19). Mit mauerLinie stehen die Tuerme einer Front in einer Reihe
//! (waagerecht oder senkrecht, nach der Hauptachse der Feindrichtung), dazwischen und daneben eine durchgehende Mauer,
//! und je Abschnitt bleibt neben einem Turm ein Tor von torBreite Kacheln fuer die eigenen Fahrzeuge offen.
//! Walls: remember recorded walls; with mauerLinie towers stand in an axis-aligned row joined by straight walls with a gate.
use std::cell::RefCell;

use super::*;

thread_local! {
    /// je Spieler: aufgezeichnete Mauerkacheln (x, y, Tick)
    static GEPLANT: RefCell<[Vec<(i32, i32, i32)>; 8]> = RefCell::new(Default::default());
    /// je Spieler: Tore als Kachelrechteck (x0, y0, w, h)
    static TORE: RefCell<[Vec<(i32, i32, i32, i32)>; 8]> = RefCell::new(Default::default());
    /// Richtung, mit der der zuletzt gewaehlte Turmplatz bestimmt wurde (Spieler, x, y, ux, uy): die Mauer folgt der Reihe
    static RICHTUNG: RefCell<Option<(i32, i32, i32, f64, f64)>> = const { RefCell::new(None) };
}

/// Turmplatz merkt sich die (gerundete) Richtung, nach der seine Reihe liegt.
pub fn merkeRichtung(z: &Zustand, x: i32, y: i32, ux: f64, uy: f64) { RICHTUNG.with(|r| *r.borrow_mut() = Some((z.spieler, x, y, ux, uy))); }
/// Richtung der Reihe fuer den Turm bei x,y, falls sein Platz sie gemerkt hat.
pub fn richtungFuer(z: &Zustand, x: i32, y: i32) -> Option<(f64, f64)> {
    RICHTUNG.with(|r| r.borrow().filter(|e| e.0 == z.spieler && e.1 == x && e.2 == y).map(|e| (e.3, e.4)))
}
pub fn zuruecksetzen_mauer() {
    GEPLANT.with(|g| *g.borrow_mut() = Default::default());
    TORE.with(|t| *t.borrow_mut() = Default::default());
    RICHTUNG.with(|r| *r.borrow_mut() = None);
    zuruecksetzen_zugang();
}

fn sp(z: &Zustand) -> usize { (z.spieler.max(0) as usize).min(7) }

/// Liegt eine aufgezeichnete Mauerkachel im Rechteck?
pub fn geplantIn(z: &Zustand, x0: i32, y0: i32, w: i32, h: i32) -> bool {
    GEPLANT.with(|g| g.borrow()[sp(z)].iter().any(|&(x, y, _)| x >= x0 && x < x0 + w && y >= y0 && y < y0 + h))
}

/// Liegt eine Torkachel im Rechteck?
pub fn torIn(z: &Zustand, x0: i32, y0: i32, w: i32, h: i32) -> bool {
    TORE.with(|t| t.borrow()[sp(z)].iter().any(|&(tx, ty, tw, th)| x0 < tx + tw && tx < x0 + w && y0 < ty + th && ty < y0 + h))
}

/// Mauer aufzeichnen und merken. Nach 30000 Ticks vergessen, wenn dort nie eine Mauer stand oder sie zerstoert ist
/// (Kachel wieder bebaubar).
pub fn aufzeichnen(welt: &mut dyn Welt, z: &mut Zustand, x: i32, y: i32) {
    let t = welt.Tick();
    let alt: Vec<(i32, i32, i32)> = GEPLANT.with(|g| g.borrow()[sp(z)].clone());
    let behalten: Vec<(i32, i32, i32)> = alt.into_iter().filter(|&(ax, ay, at)| {
        t - at < 30000 || { let c = zelle(welt, z, ax, ay); !zelleBebaubar(welt, c) }
    }).collect();
    GEPLANT.with(|g| g.borrow_mut()[sp(z)] = behalten);
    if geplantIn(z, x, y, 1, 1) { return; }
    let l = engine(welt, z, x, y);
    welt.RecordWall(super::zweitbasis::gruppeAm(z, x, y), l, mapWall);
    GEPLANT.with(|g| g.borrow_mut()[sp(z)].push((x, y, t)));
}

/// Feindrichtung auf die Hauptachse runden (mauerLinie): (+-1, 0) oder (0, +-1).
pub fn achse(ux: f64, uy: f64) -> (f64, f64) {
    if ux.abs() >= uy.abs() { (if ux < 0.0 { -1.0 } else { 1.0 }, 0.0) } else { (0.0, if uy < 0.0 { -1.0 } else { 1.0 }) }
}

/// Wachturm (gebaut oder geplant) auf der Kachel?
fn turmAuf(welt: &mut dyn Welt, z: &Zustand, x: i32, y: i32) -> bool {
    for b in welt.PlayerBuildingEnum(z.spieler, mapGuardPost) {
        let l = welt.Location(b);
        if edX(welt, z, l) == x && edY(welt, z, l) == y { return true; }
    }
    (0..z.anzahlPlaetze as usize).any(|i| { let q = z.plaetze[i]; q.typ as i32 == mapGuardPost && q.zustand != P_FREI && q.x0 as i32 == x && q.y0 as i32 == y })
}

/// Kachel fuer eine Linienmauer frei: bebaubar, kein Gebaeude/Plan darauf (direkt daneben ist erlaubt), kein
/// Roehrenstutzen, keine Bake, kein Tor.
fn linieFrei(welt: &mut dyn Welt, z: &Zustand, hin: &[Rechteck], nh: i32, x: i32, y: i32) -> bool {
    if !aufKarte(welt, z, x, y, 2) { return false; }
    let c = zelle(welt, z, x, y);
    if !zelleBebaubar(welt, c) { return false; }
    for i in 0..nh as usize {
        let r = &hin[i];
        if x >= r.x0 && x < r.x0 + r.w && y >= r.y0 && y < r.y0 + r.h { return false; }
        if r.stutzen && ((x == r.x0 + r.w && y == r.y0 + r.h / 2) || (x == r.x0 + r.w / 2 && y == r.y0 + r.h)) { return false; }
    }
    for i in 0..z.kbN as usize {
        if ueberlappt(welt, x, y, 1, 1, z.kbX[i] as i32 - 2, z.kbY[i] as i32 - 1, 4, 3) { return false; }
    }
    !torIn(z, x, y, 1, 1)
}

/// Mauerlinie durch einen Wachturm quer zur (gerundeten) Feindrichtung: auf beiden Seiten bis zum naechsten Turm, einem
/// Hindernis oder mauerLaenge Kacheln. Steht im Umkreis torAbstand noch kein Tor, bleibt auf der Seite zur Basis ein Tor
/// von torBreite Kacheln direkt am Turm offen (der Turm schuetzt es). Liefert die Zahl der Mauerkacheln.
pub fn linie(welt: &mut dyn Welt, z: &mut Zustand, tx: i32, ty: i32, ax: f64, ay: f64) -> i32 {
    let p = PAR(z.spieler);
    let mut hin = [Rechteck::default(); 160];
    let nh = hindernisse(welt, z, &mut hin, 160);
    let (lx, ly) = (-ay as i32, ax as i32);   // entlang der Linie
    let ta = p.torAbstand as i32;
    // schon ein Tor auf dieser Linie (gleiche Spalte bzw. Zeile) im Umkreis torAbstand?
    let torDa = TORE.with(|t| t.borrow()[sp(z)].iter().any(|&(x, y, w, h)| {
        if lx == 0 { tx >= x && tx < x + w && (y - ty).abs() <= ta } else { ty >= y && ty < y + h && (x - tx).abs() <= ta }
    }));
    let zurBasis = (z.bx - tx) * lx + (z.by - ty) * ly;
    let torSeite = if zurBasis >= 0 { 1 } else { -1 };
    let tb = (p.torBreite as i32).max(1);
    // Torseite: zur Basis, sonst die andere; die Torkacheln muessen befahrbar und frei von Tuermen sein
    let torOk = |welt: &mut dyn Welt, z: &Zustand, s: i32| (1..=tb).all(|j| {
        let (x, y) = (tx + lx * j * s, ty + ly * j * s);
        let c = zelle(welt, z, x, y);
        zelleBefahrbar(welt, c) && !turmAuf(welt, z, x, y)
    });
    let torAuf = if torDa { 0 } else if torOk(welt, z, torSeite) { torSeite } else if torOk(welt, z, -torSeite) { -torSeite } else { 0 };
    let (mut n, mut tor) = (0, false);
    let mut kacheln = String::new();
    for s in [torSeite, -torSeite] {
        let mut k = 1;
        if s == torAuf {
            let (x1, y1) = (tx + lx * s, ty + ly * s);
            let (x2, y2) = (tx + lx * tb * s, ty + ly * tb * s);
            TORE.with(|t| t.borrow_mut()[sp(z)].push((x1.min(x2), y1.min(y2), (x2 - x1).abs() + 1, (y2 - y1).abs() + 1)));
            tor = true; k = tb + 1;
        }
        while k <= p.mauerLaenge as i32 {
            let (x, y) = (tx + lx * k * s, ty + ly * k * s);
            if turmAuf(welt, z, x, y) || !linieFrei(welt, z, &hin, nh, x, y) { break; }
            vormerken(welt, z, x, y);
            kacheln.push_str(&format!(" {},{}", x, y));
            n += 1; k += 1;
        }
    }
    melde(welt, z, "kampf", mapWall, tx, ty, &format!("Mauerlinie {} am Wachturm: {} Kacheln{} --{}",
          if lx == 0 { "senkrecht" } else { "waagerecht" }, n, if tor { format!(", Tor {} breit ab {},{}", tb, tx + lx * torAuf, ty + ly * torAuf) } else { String::new() }, kacheln));
    n
}

// ---------------------------------------------------------------------------------------------------
// Bau-Reihenfolge (PAR bauReihe, Nutzer 2026-11-19): erst Gebaeude mit Personal anschliessen, dann Tuerme, zuletzt Mauern.
// Mauern werden dann nicht sofort aufgezeichnet, sondern warten, bis keine Roehre offen ist und nichts mehr anzuschliessen
// ist -- und gehen in Schueben von mauerSchub Kacheln an die BuildingGroup (der Earthworker baut sonst erst alle Mauern,
// bevor die naechste Roehre drankommt).

thread_local! {
    /// je Spieler: Mauerkacheln, die auf Luft warten (schon in GEPLANT, also fuer andere Bauten gesperrt)
    static WARTE: RefCell<[Vec<(i32, i32)>; 8]> = RefCell::new(Default::default());
}

/// Mauer vormerken (bauReihe) oder gleich aufzeichnen.
pub fn vormerken(welt: &mut dyn Welt, z: &mut Zustand, x: i32, y: i32) {
    // Missionsmodus (Nutzer 2026-11-22): zwischen Mauer und Turm immer eine Kachel Abstand
    if super::modus::mission(z) && turmNeben(welt, z, x, y) { return; }
    if PAR(z.spieler).bauReihe < 0.5 { aufzeichnen(welt, z, x, y); return; }
    if geplantIn(z, x, y, 1, 1) { return; }
    let t = welt.Tick();
    GEPLANT.with(|g| g.borrow_mut()[sp(z)].push((x, y, t)));
    WARTE.with(|w| w.borrow_mut()[sp(z)].push((x, y)));
}

/// Luft: wartende Mauern schubweise an die BuildingGroup geben. Aufruf aus anschluss, wenn keine Roehre offen ist und
/// nichts anzuschliessen ist.
pub fn freigeben(welt: &mut dyn Welt, z: &mut Zustand) {
    let n = (PAR(z.spieler).mauerSchub as usize).max(1);
    let schub: Vec<(i32, i32)> = WARTE.with(|w| { let v = &mut w.borrow_mut()[sp(z)]; let k = n.min(v.len()); v.drain(..k).collect() });
    if schub.is_empty() { return; }
    let t = welt.Tick();
    for &(x, y) in &schub {
        let l = engine(welt, z, x, y);
        welt.RecordWall(super::zweitbasis::gruppeAm(z, x, y), l, mapWall);
        // Merkzeit erneuern (vergessen wird ab der Aufzeichnung gerechnet)
        GEPLANT.with(|g| for e in g.borrow_mut()[sp(z)].iter_mut() { if e.0 == x && e.1 == y { e.2 = t; } });
    }
    let rest = WARTE.with(|w| w.borrow()[sp(z)].len());
    melde(welt, z, "bau", mapWall, schub[0].0, schub[0].1, &format!("Luft: {} Mauerkacheln an die BuildingGroup (noch {} wartend)", schub.len(), rest));
}

// ---------------------------------------------------------------------------------------------------
// Zugangsmauern (PAR zugangMauer, Nutzer 2026-11-19): Zugaenge aus dem Gelaende statt Mauerstuecke an jedem Turm. Ein Ring
// im Wegabstand zmAbstand um den ganzen Kolonie-Raum (8er-Nachbarschaft: jeder Weg nach draussen kreuzt ihn) zerfaellt
// zwischen Felsen in zusammenhaengende Stuecke -- die Zugaenge. Nur Stuecke, durch die ein brauchbarer Feindweg fuehrt.
// Schmal (bis zmKlein Kacheln): offen lassen, eine Mauer machte ihn zu eng oder ganz zu. Breiter bis zmMax: Mauer ueber die
// ganze Breite, ein Tor von torBreite Kacheln, wo der Feindweg durchgeht (die absichtliche Engstelle), dahinter zwei Tuerme
// seitlich vom Tor (die Ausfahrt bleibt frei). Breiter als zmMax: offenes Feld, keine Mauer.

#[derive(Clone, Debug, Default)]
pub struct Zugang { pub ort: (i32, i32), pub mauer: Vec<(i32, i32)>, pub tor: Vec<(i32, i32)>, pub tuerme: Vec<(i32, i32)>, pub breite: i32, pub mauerAuf: bool }

thread_local! {
    static ZUGAENGE: RefCell<[Vec<Zugang>; 8]> = RefCell::new(Default::default());
    static ZPLAN: RefCell<[i32; 8]> = const { RefCell::new([-1_000_000; 8]) };
}
pub fn zuruecksetzen_zugang() {
    ZUGAENGE.with(|g| *g.borrow_mut() = Default::default());
    GEBIETE.with(|g| *g.borrow_mut() = Default::default());
    GPLAN.with(|g| *g.borrow_mut() = [-1_000_000; 8]);
    ZPLAN.with(|g| *g.borrow_mut() = [-1_000_000; 8]);
    WARTE.with(|w| *w.borrow_mut() = Default::default());
}

/// Steht der Turm (Kachel) an einem Zugang? Dann bekommt er keine eigenen Mauerstuecke.
pub fn istZugangTurm(z: &Zustand, x: i32, y: i32) -> bool {
    ZUGAENGE.with(|g| g.borrow()[sp(z)].iter().any(|a| a.tuerme.contains(&(x, y))))
}

/// Zugaenge planen (alle 20000 Ticks, sobald der Feind bekannt ist); neue Zugaenge nur, wo noch keiner liegt.
fn zugangPlanen(welt: &mut dyn Welt, z: &mut Zustand) {
    let p = PAR(z.spieler);
    let t = welt.Tick();
    if z.feindGeb == 0 || t - ZPLAN.with(|g| g.borrow()[sp(z)]) < 20000 { return; }
    ZPLAN.with(|g| g.borrow_mut()[sp(z)] = t);
    let (mut eigen, feindK) = super::erweiterung::kolonieRaum(welt, z);
    if eigen.is_empty() || feindK.is_empty() { return; }
    // geplante und reservierte Bauplaetze gehoeren dazu: die Basis waechst sonst ueber die Mauer hinaus
    for i in 0..z.anzahlPlaetze as usize {
        let q = z.plaetze[i];
        if q.zustand == P_FREI { continue; }
        let gf = grundflaeche(welt, q.typ as i32);
        for y in q.y0 as i32..q.y0 as i32 + gf.h { for x in q.x0 as i32..q.x0 as i32 + gf.w { eigen.push((x, y)); } }
    }
    for q in super::vorplan::reserviert(z) { for y in q.y0..q.y0 + q.h { for x in q.x0..q.x0 + q.w { eigen.push((x, y)); } } }
    let (b, h) = (z.breite, z.hoehe);
    let mut zellen = vec![0i32; (b * h) as usize];
    for y in 0..h { for x in 0..b { let l = engine(welt, z, x, y); zellen[(y * b + x) as usize] = welt.GetCellType(l); } }
    let g = super::gelaende::Gelaende::neu_quellen(&zellen, b, h, &eigen, &feindK, &[], 0.4, 1, 1, 0);
    if g.weg <= 0 { return; }
    let grenze = g.weg + (g.weg as f64 * 0.4).ceil() as i32;
    let r = (p.zmAbstand as i32).max(2);
    let mut ring: Vec<(i32, i32)> = Vec::new();
    for y in 2..h - 2 { for x in 2..b - 2 { if g.d_eigen[(y * b + x) as usize] == r { ring.push((x, y)); } } }
    ringBebauen(welt, z, &g, ring, r, grenze, true);
}

/// Zugaenge eines Rings (Kacheln im Wegabstand r) befestigen: zusammenhaengende Stuecke zwischen Felsen. nurFeindweg: nur
/// Stuecke, durch die ein brauchbarer Feindweg fuehrt (zugangMauer); sonst alle (gebietAuto), das Tor dann auf dem Feindweg
/// oder in der Mitte des Zugangs.
fn ringBebauen(welt: &mut dyn Welt, z: &mut Zustand, g: &super::gelaende::Gelaende, mut ring: Vec<(i32, i32)>, r: i32, grenze: i32, nurFeindweg: bool) {
    let p = PAR(z.spieler);
    let t = welt.Tick();
    let (b, h) = (z.breite, z.hoehe);
    let mut hin = [Rechteck::default(); 160];
    let nh = hindernisse(welt, z, &mut hin, 160);
    let (klein, gross) = (p.zmKlein as usize, p.zmMax as usize);
    let tb = (p.torBreite as usize).max(1);
    while let Some(s) = ring.pop() {
        // zusammenhaengendes Stueck (8 Nachbarn)
        let mut teil = vec![s];
        let mut i = 0;
        while i < teil.len() {
            let (x, y) = teil[i];
            let mut j = 0;
            while j < ring.len() { let (a, c) = ring[j]; if (a - x).abs() <= 1 && (c - y).abs() <= 1 { teil.push(ring.swap_remove(j)); } else { j += 1; } }
            i += 1;
        }
        let feindweg: Vec<(i32, i32)> = teil.iter().copied().filter(|&(x, y)| { let i = (y * b + x) as usize; g.d_feind[i] >= 0 && g.d_eigen[i] + g.d_feind[i] <= grenze }).collect();
        if nurFeindweg && feindweg.is_empty() { continue; }
        let hatFeindweg = !feindweg.is_empty();
        let brauchbar = if feindweg.is_empty() { teil.clone() } else { feindweg };
        let schon = ZUGAENGE.with(|zz| zz.borrow()[sp(z)].iter().any(|a| a.mauer.iter().chain(a.tor.iter()).any(|m| teil.iter().any(|k| (k.0 - m.0).abs() <= 3 && (k.1 - m.1).abs() <= 3))));
        if schon { continue; }
        let mitte = { let n = brauchbar.len() as i32; (brauchbar.iter().map(|k| k.0).sum::<i32>() / n, brauchbar.iter().map(|k| k.1).sum::<i32>() / n) };
        if teil.len() <= klein {
            melde(welt, z, "kampf", mapWall, mitte.0, mitte.1, &format!("Zugang {},{} nur {} Kacheln breit -- bleibt ohne Mauer offen", mitte.0, mitte.1, teil.len()));
            continue;
        }
        // gebietAuto: offenes Feld (breiter als drei volle Zugaenge) bleibt ohne Mauer -- dort waere eine Schildmauer ein
        // Stueck ins Leere; die Patrouille deckt es. Bis dahin eine durchgehende Mauer von Fels zu Fels mit mehreren Toren.
        if !nurFeindweg && teil.len() > 3 * gross {
            melde(welt, z, "kampf", mapWall, mitte.0, mitte.1, &format!("Zugang {},{} {} Kacheln breit -- offenes Feld, keine Mauer", mitte.0, mitte.1, teil.len()));
            continue;
        }
        if teil.len() > gross && nurFeindweg {
            // breiter Zugang: Schildmauer von zmSchild Kacheln quer zur Feindrichtung, wo die Feindwege durchgehen, und je ein
            // Turm hinter ihren Enden (sie schiessen an der Mauer vorbei)
            let c = *brauchbar.iter().min_by_key(|k| (k.0 - mitte.0).pow(2) + (k.1 - mitte.1).pow(2)).unwrap();
            let df = |x: i32, y: i32| { if x < 0 || y < 0 || x >= b || y >= h { i32::MAX } else { let v = g.d_feind[(y * b + x) as usize]; if v < 0 { i32::MAX } else { v } } };
            let (fx, fy) = { let (mut ux, mut uy) = (0.0f64, 0.0f64);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] { let d = df(c.0 + dx * 3, c.1 + dy * 3); if d != i32::MAX { let w = (df(c.0, c.1).min(100000) - d) as f64; ux += w * dx as f64; uy += w * dy as f64; } }
                achse(ux, uy) };
            let (lx, ly) = (-fy as i32, fx as i32);
            let halb = (p.zmSchild as i32 - 1) / 2;
            let mut mauer: Vec<(i32, i32)> = Vec::new();
            let mut enden = [c, c];
            for (e, sg) in [(0usize, -1i32), (1, 1)] {
                for k in 0..=(halb + if sg > 0 { (p.zmSchild as i32 - 1) % 2 } else { 0 }) {
                    if k == 0 && sg > 0 { continue; }
                    let (x, y) = (c.0 + lx * k * sg, c.1 + ly * k * sg);
                    if !linieFrei(welt, z, &hin, nh, x, y) { break; }
                    mauer.push((x, y)); enden[e] = (x, y);
                }
            }
            if (mauer.len() as f64) < p.zmSchild.min(3.0) {
                melde(welt, z, "kampf", mapWall, mitte.0, mitte.1, &format!("Zugang {},{} {} Kacheln breit -- kein Platz fuer eine Schildmauer", mitte.0, mitte.1, teil.len()));
                continue;
            }
            let zh = p.zmHinter as i32;
            let mut tuerme: Vec<(i32, i32)> = Vec::new();
            for (e, sg) in [(enden[0], -1i32), (enden[1], 1)] {
                let mut best: Option<((i32, i32), i32)> = None;
                for d in -1..=1 { for q in 0..=1 {
                    // seitlich q Kacheln ueber das Ende hinaus, zh (+d) Kacheln hinter der Mauer
                    let (x, y) = (e.0 + lx * sg * q - fx as i32 * (zh + d), e.1 + ly * sg * q - fy as i32 * (zh + d));
                    if x < 2 || y < 2 || x >= b - 2 || y >= h - 2 { continue; }
                    let mut gr: &'static str = "";
                    if !platzFrei(welt, z, x, y, 1, 1, &hin, nh, &mut gr) || mauer.contains(&(x, y)) { continue; }
                    let w = d.abs() * 2 + (1 - q);
                    if best.is_none_or(|bb| w < bb.1) { best = Some(((x, y), w)); }
                } }
                if let Some((t2, _)) = best { tuerme.push(t2); }
            }
            melde(welt, z, "kampf", mapWall, c.0, c.1, &format!("Zugang {},{} {} Kacheln breit: Schildmauer {} Kacheln {} bei {},{}, {} Tuerme dahinter",
                  mitte.0, mitte.1, teil.len(), mauer.len(), if lx == 0 { "senkrecht" } else { "waagerecht" }, c.0, c.1, tuerme.len()));
            for &(x, y) in &mauer { GEPLANT.with(|gg| gg.borrow_mut()[sp(z)].push((x, y, t))); }
            ZUGAENGE.with(|zz| zz.borrow_mut()[sp(z)].push(Zugang { ort: c, mauer, tor: Vec::new(), tuerme, breite: teil.len() as i32, mauerAuf: false }));
            continue;
        }
        // entlang der laengeren Seite ordnen
        let (x0, x1) = (teil.iter().map(|k| k.0).min().unwrap(), teil.iter().map(|k| k.0).max().unwrap());
        let (y0, y1) = (teil.iter().map(|k| k.1).min().unwrap(), teil.iter().map(|k| k.1).max().unwrap());
        if x1 - x0 >= y1 - y0 { teil.sort_by_key(|k| (k.0, k.1)); } else { teil.sort_by_key(|k| (k.1, k.0)); }
        // Tor: torBreite Kacheln um die Kachel, die der Mitte der Feindwege am naechsten liegt. gebietAuto: breite Zugaenge
        // bekommen ein Tor je angefangene zmMax Kacheln, gleichmaessig verteilt (eines davon auf dem Feindweg)
        let im = (0..teil.len()).min_by_key(|&i| (teil[i].0 - mitte.0).pow(2) + (teil[i].1 - mitte.1).pow(2)).unwrap();
        let ntore = if nurFeindweg { 1 } else { teil.len().div_ceil(gross.max(1)).max(1) };
        let mut mitten: Vec<usize> = if ntore == 1 { vec![im] } else { (0..ntore).map(|i| teil.len() * (2 * i + 1) / (2 * ntore)).collect() };
        if ntore > 1 && hatFeindweg { if let Some(j) = (0..ntore).min_by_key(|&j| (mitten[j] as i32 - im as i32).abs()) { mitten[j] = im; } }
        let anfaenge: Vec<usize> = mitten.iter().map(|&c| c.saturating_sub((tb - 1) / 2).min(teil.len() - tb)).collect();
        let a = anfaenge[0];
        let tor: Vec<(i32, i32)> = anfaenge.iter().flat_map(|&a| teil[a..a + tb].to_vec()).collect();
        let mauer: Vec<(i32, i32)> = teil.iter().copied().filter(|k| !tor.contains(k)).filter(|&(x, y)| linieFrei(welt, z, &hin, nh, x, y)).collect();
        // Tuerme: zmHinter Kacheln hinter der Mauer, seitlich neben jedem Tor (mindestens 2 Kacheln von jeder Torkachel)
        let hinter = r - p.zmHinter as i32;
        let mut tuerme: Vec<(i32, i32)> = Vec::new();
        let seiten: Vec<i32> = anfaenge.iter().flat_map(|&a| [a as i32 - 1, (a + tb) as i32]).collect();
        let _ = a;
        for seite in seiten {
            if seite < 0 || seite as usize >= teil.len() { continue; }
            let w = teil[seite as usize];
            let mut best: Option<((i32, i32), i32)> = None;
            for dy in -3..=3 { for dx in -3..=3 {
                let (x, y) = (w.0 + dx, w.1 + dy);
                if x < 2 || y < 2 || x >= b - 2 || y >= h - 2 || g.d_eigen[(y * b + x) as usize] != hinter { continue; }
                if tor.iter().any(|k| (k.0 - x).abs() < 2 && (k.1 - y).abs() < 2) || tuerme.iter().any(|q| (q.0 - x).abs() < 2 && (q.1 - y).abs() < 2) { continue; }
                let mut gr: &'static str = "";
                if !platzFrei(welt, z, x, y, 1, 1, &hin, nh, &mut gr) { continue; }
                let d = dx * dx + dy * dy;
                if best.is_none_or(|bb| d < bb.1) { best = Some(((x, y), d)); }
            } }
            if let Some((q, _)) = best { tuerme.push(q); }
        }
        melde(welt, z, "kampf", mapWall, tor[0].0, tor[0].1, &format!("Zugang {},{} {} Kacheln breit: Mauer {} Kacheln, {} Tor(e) {} breit bei {},{}, {} Tuerme dahinter",
              mitte.0, mitte.1, teil.len(), mauer.len(), ntore, tb, tor[0].0, tor[0].1, tuerme.len()));
        // Mauer und Tor sofort sperren, damit nichts anderes dort geplant wird
        for &(x, y) in &mauer { GEPLANT.with(|gg| gg.borrow_mut()[sp(z)].push((x, y, t))); }
        for &a in &anfaenge {
            let tt: &[(i32, i32)] = &teil[a..a + tb];
            let (tx0, ty0) = (tt.iter().map(|k| k.0).min().unwrap(), tt.iter().map(|k| k.1).min().unwrap());
            let (tx1, ty1) = (tt.iter().map(|k| k.0).max().unwrap(), tt.iter().map(|k| k.1).max().unwrap());
            TORE.with(|q| q.borrow_mut()[sp(z)].push((tx0, ty0, tx1 - tx0 + 1, ty1 - ty0 + 1)));
        }
        ZUGAENGE.with(|zz| zz.borrow_mut()[sp(z)].push(Zugang { ort: tor[0], mauer, tor, tuerme, breite: teil.len() as i32, mauerAuf: false }));
    }
}

thread_local! {
    static GPLAN: RefCell<[i32; 8]> = const { RefCell::new([-1_000_000; 8]) };
    /// schon geplante Gebiete je Spieler (Mitte der Basis): einmal je Basis, sonst entstanden Mauern auf mehreren Ringen
    static GEBIETE: RefCell<[Vec<(i32, i32)>; 8]> = RefCell::new(Default::default());
}

/// Gebiet selbst finden (Missionsmodus, PAR gebietAuto; Nutzer 2026-11-22: "dass die KI das Plateau, auf dem sie sitzt,
/// selber findet, und an Basis 2 die Gegebenheiten besser nutzt"). Je Basis (Hauptbasis, jede unabhaengige zweite Basis):
/// Wegabstand von ihren Gebaeuden, geplanten Plaetzen und Bezirken; der Rand ist der Abstand mit dem schmalsten Ring
/// (3..12 Kacheln) -- an einer Plateaukante bleiben nur die Durchlaesse zwischen den Felsen. Jeder Durchlass wird befestigt
/// (ringBebauen ohne Feindweg-Pflicht): Mauer mit Tor und zwei Tuermen, breite Zugaenge Schildmauer mit Tuermen, schmale
/// bleiben offen. Alle 20000 Ticks neu (die Basis waechst); schon befestigte Zugaenge bleiben.
fn gebietPlanen(welt: &mut dyn Welt, z: &mut Zustand) {
    let t = welt.Tick();
    if t < 8000 || t - GPLAN.with(|g| g.borrow()[sp(z)]) < 2000 { return; }
    GPLAN.with(|g| g.borrow_mut()[sp(z)] = t);
    let (alle, feindK) = super::erweiterung::kolonieRaum(welt, z);
    let mut plan: Vec<(i32, i32)> = Vec::new();
    for i in 0..z.anzahlPlaetze as usize {
        let q = z.plaetze[i];
        if q.zustand == P_FREI || q.zustand == P_GESPERRT { continue; }
        let gf = grundflaeche(welt, q.typ as i32);
        for y in q.y0 as i32..q.y0 as i32 + gf.h { for x in q.x0 as i32..q.x0 as i32 + gf.w { plan.push((x, y)); } }
    }
    for (x, y) in super::bezirke::ankerPunkte(z) { for dy in -4..=4 { for dx in -4..=4 { plan.push((x + dx, y + dy)); } } }
    let (b, h) = (z.breite, z.hoehe);
    let mut zellen = vec![0i32; (b * h) as usize];
    for y in 0..h { for x in 0..b { let l = engine(welt, z, x, y); zellen[(y * b + x) as usize] = welt.GetCellType(l); } }
    // Basen: Hauptbasis (Umkreis 35) und die unabhaengigen zweiten (Umkreis 30)
    let mut basen: Vec<(&str, i32, i32, i32)> = vec![("Hauptbasis", z.bx, z.by, 35)];
    for k in 0..super::zweitbasis::MAX_ZB {
        let s = super::zweitbasis::zb(z.spieler, k);
        if s.groesse == 4 && s.phase >= 5 { basen.push(("zweite Basis", s.x, s.y, super::zweitbasis::BASIS_UMKREIS)); }
    }
    for (name, cx, cy, um) in basen {
        if GEBIETE.with(|g| g.borrow()[sp(z)].contains(&(cx, cy))) { continue; }
        // die Hauptbasis erst, wenn ihre Bezirke feststehen (sie gehoeren ins Gebiet)
        if name == "Hauptbasis" && PAR(z.spieler).bezirke >= 0.5 && super::bezirke::ankerPunkte(z).is_empty() { continue; }
        GEBIETE.with(|g| g.borrow_mut()[sp(z)].push((cx, cy)));
        let nah = |&(x, y): &(i32, i32)| (x - cx).pow(2) + (y - cy).pow(2) <= um * um;
        let eigen: Vec<(i32, i32)> = alle.iter().chain(plan.iter()).copied().filter(nah).collect();
        if eigen.is_empty() { continue; }
        let g = super::gelaende::Gelaende::neu_quellen(&zellen, b, h, &eigen, &feindK, &[], 0.4, 1, 1, 0);
        let grenze = if g.weg > 0 { g.weg + (g.weg as f64 * 0.4).ceil() as i32 } else { i32::MAX / 4 };
        let weit = (um + 15) * (um + 15);
        // Ringgroesse je Abstand, im Verhaeltnis zum Umfang, den ein Ring dort ohne Felsen haette (2 pi (R0 + r), R0 =
        // Radius der Bebauung): der kleinste Anteil ist der natuerliche Rand -- dort verengen Felsen den Ring (Plateaukante)
        let mut n = [0usize; 17];
        for y in 2..h - 2 { for x in 2..b - 2 {
            let d = g.d_eigen[(y * b + x) as usize];
            if (3..=16).contains(&d) && (x - cx).pow(2) + (y - cy).pow(2) <= weit { n[d as usize] += 1; }
        } }
        let r0 = ((eigen.len() as f64) / std::f64::consts::PI).sqrt();
        let anteil = |r: usize| n[r] as f64 / (std::f64::consts::TAU * (r0 + r as f64));
        let Some(r) = (3..=16).filter(|&r| n[r] > 0).min_by(|&a, &c| anteil(a).total_cmp(&anteil(c)).then(a.cmp(&c))) else { continue; };
        let ring: Vec<(i32, i32)> = (2..h - 2).flat_map(|y| (2..b - 2).map(move |x| (x, y)))
            .filter(|&(x, y)| g.d_eigen[(y * b + x) as usize] == r as i32 && (x - cx).pow(2) + (y - cy).pow(2) <= weit).collect();
        let aw: Vec<i32> = (3..=16).map(|r| (anteil(r) * 100.0).round() as i32).collect();
        melde(welt, z, "kampf", mapWall, cx, cy, &format!("Gebiet {}: Rand im Wegabstand {} von der Bebauung, {} offene Kacheln ({:.0} % eines Rings ohne Fels; Abstand 3..16 in %: {:?})",
              name, r, ring.len(), anteil(r) * 100.0, aw));
        ringBebauen(welt, z, &g, ring, r as i32, grenze, false);
    }
}

/// Je Schritt: planen; Mauern eines Zugangs vormerken, sobald einer seiner Tuerme steht oder geplant ist (Tuerme vor Mauern).
pub fn zugangSchritt(welt: &mut dyn Welt, z: &mut Zustand) {
    if PAR(z.spieler).zugangMauer < 0.5 || z.bauAn == 0 { return; }
    if PAR(z.spieler).gebietAuto >= 0.5 { gebietPlanen(welt, z); } else { zugangPlanen(welt, z); }
    let n = ZUGAENGE.with(|zz| zz.borrow()[sp(z)].len());
    for i in 0..n {
        let a = ZUGAENGE.with(|zz| zz.borrow()[sp(z)][i].clone());
        if a.mauerAuf { continue; }
        if !a.tuerme.is_empty() && !a.tuerme.iter().any(|&(x, y)| turmAuf(welt, z, x, y)) { continue; }
        // die gesperrten Kacheln wieder freigeben, damit vormerken/aufzeichnen sie nicht als schon geplant ueberspringt
        GEPLANT.with(|gg| gg.borrow_mut()[sp(z)].retain(|e| !a.mauer.contains(&(e.0, e.1))));
        for &(x, y) in &a.mauer { vormerken(welt, z, x, y); }
        ZUGAENGE.with(|zz| zz.borrow_mut()[sp(z)][i].mauerAuf = true);
        let k: String = a.mauer.iter().map(|m| format!(" {},{}", m.0, m.1)).collect();
        let t: String = a.tor.iter().map(|m| format!(" {},{}", m.0, m.1)).collect();
        melde(welt, z, "kampf", mapWall, a.ort.0, a.ort.1, &format!("Zugangsmauer {} Kacheln vorgemerkt (Turm steht) --{} | Tor{}", a.mauer.len(), k, t));
    }
}

/// Turmplatz an einem Zugang, dessen Tuerme fehlen. -1 = keiner.
pub fn zugangTurm(welt: &mut dyn Welt, z: &mut Zustand) -> i32 {
    if PAR(z.spieler).zugangMauer < 0.5 { return -1; }
    let alle: Vec<Zugang> = ZUGAENGE.with(|zz| zz.borrow()[sp(z)].clone());
    let mut hin = [Rechteck::default(); 160];
    let nh = hindernisse(welt, z, &mut hin, 160);
    for a in &alle {
        for &(x, y) in &a.tuerme {
            if turmAuf(welt, z, x, y) { continue; }
            // Tuerme einer unabhaengigen zweiten Basis baut sie selbst (zugangTurmBei)
            if super::zweitbasis::gruppeAm(z, x, y) != z.bau { continue; }
            let mut g: &'static str = "";
            // Turm darf direkt an seiner eigenen Mauer stehen -- die Festungstuerme sind Teil der Mauer
            let _ = &mut g; if !turmPlatzFrei(welt, z, x, y, &hin, nh) { continue; }
            let idx = platzEintrag(welt, z, mapGuardPost, x, y);
            if idx >= 0 { melde(welt, z, "kampf", mapGuardPost, x, y, &format!("Wachturm am Zugang {},{} ({} Kacheln breit, {})", a.ort.0, a.ort.1, a.breite, if a.tor.is_empty() { "hinter der Schildmauer" } else { "neben dem Tor" })); }
            return idx;
        }
    }
    -1
}

/// Fehlender Turm eines Zugangs im Umkreis r um (cx, cy) -- fuer die unabhaengige zweite Basis, die ihre Zugangstuerme selbst
/// baut. Ein Platz, der gerade nicht frei ist (Fahrzeug, Bau), wird uebersprungen.
pub fn zugangTurmBei(welt: &mut dyn Welt, z: &mut Zustand, cx: i32, cy: i32, r: i32) -> Option<(i32, i32)> {
    if PAR(z.spieler).zugangMauer < 0.5 { return None; }
    let alle: Vec<Zugang> = ZUGAENGE.with(|zz| zz.borrow()[sp(z)].clone());
    let mut hin = [Rechteck::default(); 160];
    let nh = hindernisse(welt, z, &mut hin, 160);
    for a in &alle { for &(x, y) in &a.tuerme {
        if (x - cx).pow(2) + (y - cy).pow(2) > r * r || turmAuf(welt, z, x, y) { continue; }
        let mut g: &'static str = "";
        let _ = &mut g; if !turmPlatzFrei(welt, z, x, y, &hin, nh) { continue; }
        return Some((x, y));
    } }
    None
}

/// Raumhafen-Festung (Missionsmodus, Nutzer 2026-11-22: "Raumhafen soll in die Mitte und von 3 Seiten mit Mauer und Tuermen
/// verteidigt werden"): Mauerkacheln sofort sperren und als Zugang eintragen -- die Tuerme baut wachtPlatz (zugangTurm) zuerst,
/// die Mauer folgt, sobald ein Turm steht (zugangSchritt).
pub fn festungEintragen(welt: &mut dyn Welt, z: &mut Zustand, ort: (i32, i32), mut mauer: Vec<(i32, i32)>, tuerme: Vec<(i32, i32)>) {
    let t = welt.Tick();
    // eine Kachel Abstand zu den Ecktuermen
    mauer.retain(|&(x, y)| !tuerme.iter().any(|&(tx, ty)| (tx - x).abs() <= 1 && (ty - y).abs() <= 1));
    for &(x, y) in &mauer { GEPLANT.with(|gg| gg.borrow_mut()[sp(z)].push((x, y, t))); }
    melde(welt, z, "kampf", mapWall, ort.0, ort.1, &format!("Raumhafen-Festung bei {},{}: Mauer {} Kacheln an drei Seiten, {} Tuerme an den Ecken", ort.0, ort.1, mauer.len(), tuerme.len()));
    let breite = mauer.len() as i32;
    // vorn einreihen: die Festung bekommt ihre Tuerme vor den Zugaengen des Gebiets
    ZUGAENGE.with(|zz| zz.borrow_mut()[sp(z)].insert(0, Zugang { ort, mauer, tor: Vec::new(), tuerme, breite, mauerAuf: false }));
}

/// Fehlt noch ein Turm eines Zugangs oder der Festung (fuer den Anstoss im Missionsmodus)?
pub fn zugangTurmFehlt(welt: &mut dyn Welt, z: &Zustand) -> bool {
    let alle: Vec<Zugang> = ZUGAENGE.with(|zz| zz.borrow()[sp(z)].clone());
    alle.iter().any(|a| a.tuerme.iter().any(|&(x, y)| !turmAuf(welt, z, x, y)))
}

/// Platz fuer einen Zugangs- oder Festungsturm: die Kachel bebaubar, kein Gebaeude, kein Plan, kein Fahrzeug darauf -- der Rand
/// darf Mauer sein (an den Festungsecken stehen die Mauern schon, wenn die naechsten Tuerme kommen; platzFrei verlangt einen
/// befahrbaren Rand und lehnte sie ab)
fn turmPlatzFrei(welt: &mut dyn Welt, z: &Zustand, x: i32, y: i32, hin: &[Rechteck], nh: i32) -> bool {
    if x < 2 || y < 2 || x >= z.breite - 2 || y >= z.hoehe - 2 { return false; }
    let c = zelle(welt, z, x, y);
    if !zelleBebaubar(welt, c) { return false; }
    if hin[..nh as usize].iter().any(|r| x >= r.x0 && x < r.x0 + r.w && y >= r.y0 && y < r.y0 + r.h) { return false; }
    let l = engine(welt, z, x, y);
    !welt.LocationEnumerator(l).into_iter().any(|u| welt.IsVehicle(u) && welt.GetCurAction(u) == moDone)
}

/// Steht oder kommt neben (x, y) ein Turm (8 Nachbarn)? Gebaut, als Platz aufgezeichnet oder als Turm eines Zugangs vorgesehen.
pub fn turmNeben(welt: &mut dyn Welt, z: &Zustand, x: i32, y: i32) -> bool {
    for dy in -1..=1 { for dx in -1..=1 {
        if dx == 0 && dy == 0 { continue; }
        if turmAuf(welt, z, x + dx, y + dy) { return true; }
    } }
    ZUGAENGE.with(|zz| zz.borrow()[sp(z)].iter().any(|a| a.tuerme.iter().any(|&(tx, ty)| (tx - x).abs() <= 1 && (ty - y).abs() <= 1 && (tx, ty) != (x, y))))
}
