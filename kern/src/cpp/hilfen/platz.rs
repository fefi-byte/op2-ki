//! Plaetze pruefen und waehlen: Hindernisse, platzFrei, Explosionsabstand, platzWaehlen.
use super::*;
use super::super::*;

thread_local! { static WEGE: std::cell::RefCell<[Option<(i32, i32, std::rc::Rc<Vec<i32>>)>; 8]> = std::cell::RefCell::new(Default::default()); }

/// Wegabstand (8 Nachbarn, Gelaende) von der eigenen Basismitte, je Spieler alle 2000 Ticks neu; -1 = nicht erreichbar
fn wegFeld(welt: &mut dyn Welt, z: &Zustand) -> std::rc::Rc<Vec<i32>> {
    let sp = (z.spieler & 7) as usize; let t = welt.Tick();
    if let Some(f) = WEGE.with(|w| w.borrow()[sp].as_ref().filter(|e| t >= e.0 && t - e.0 < 2000 && e.1 == z.bx * 1024 + z.by).map(|e| e.2.clone())) { return f; }
    let r = super::super::routen::raster(welt, z);
    let (b, h) = (r.b, r.h);
    let mut d = vec![-1i32; (b * h) as usize];
    let mut q = std::collections::VecDeque::new();
    for dy in -2..=2 { for dx in -2..=2 {
        let (x, y) = (z.bx + dx, z.by + dy);
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
    let f = std::rc::Rc::new(d);
    WEGE.with(|w| w.borrow_mut()[sp] = Some((t, z.bx * 1024 + z.by, f.clone())));
    f
}

/// PAR platzErreichbar (2026-11-21): erreicht ein ConVec den Platz von der Basis aus ohne grossen Umweg? Gemessen ml4_08
/// Seed 3: Tokamak und Universitaet auf einem Plateau hinter einer Klippenkante, der ConVec wartete in einer Sackgasse,
/// bis der Wachhund nach 8000 Ticks abbrach -- die Eroeffnung stand so lange. Eine Randkachel muss erreichbar sein, der
/// Weg hoechstens 2,5 x Schachbrettabstand + 12.
/// Wegabstand (Kacheln, 8 Nachbarn) von der Basismitte zur Kachel; -1 = nicht erreichbar
pub fn wegAbstand(welt: &mut dyn Welt, z: &Zustand, x: i32, y: i32) -> i32 {
    let f = wegFeld(welt, z);
    let b = z.breite.clamp(1, 512);
    if x < 0 || y < 0 || x >= b || y >= z.hoehe { return -1; }
    f.get((y * b + x) as usize).copied().unwrap_or(-1)
}

pub fn erreichbar(welt: &mut dyn Welt, z: &Zustand, x0: i32, y0: i32, w: i32, h: i32) -> bool {
    if PAR(z.spieler).platzErreichbar < 0.5 || z.breite <= 0 { return true; }
    let f = wegFeld(welt, z);
    let b = z.breite.clamp(1, 512);
    let luft = (x0 + w / 2 - z.bx).abs().max((y0 + h / 2 - z.by).abs());
    let grenze = (2.5 * luft as f64) as i32 + 12;
    for y in (y0 - 1)..=(y0 + h) {
        for x in (x0 - 1)..=(x0 + w) {
            if x >= x0 && x < x0 + w && y >= y0 && y < y0 + h { continue; }
            if x < 0 || y < 0 || x >= b || y >= z.hoehe { continue; }
            let v = f[(y * b + x) as usize];
            if v >= 0 && v <= grenze { return true; }
        }
    }
    false
}

// Gebaeude eines Typs: fertig + geplant/im Bau
/// C++: int geplant(const Zustand& z, int typ)  (op2_ki.hpp:725)
pub fn geplant(welt: &mut dyn Welt, z: &Zustand, typ: i32) -> i32 {
    let mut n = 0;
    for i in 0..z.anzahlPlaetze as usize {
        if z.plaetze[i].typ as i32 == typ && (z.plaetze[i].zustand == P_FAHRT || z.plaetze[i].zustand == P_BAUT) { n += 1; }
    }
    n
}

// Bauplatz: jedes Gebaeude bringt zwei Roehrenstuecke mit (rechts, unten, Mitte der Seite im 1 Kachel breiten Rand;
// ausser Kraftwerken und Minen). Gebaeude stehen genau 1 Kachel auseinander, Stutzen trifft die Seite des anderen.

/// C++: bool hatStutzen(int typ)  (op2_ki.hpp:747)
pub fn hatStutzen(welt: &mut dyn Welt, typ: i32) -> bool {
    match typ {
        mapTokamak | mapMHDGenerator | mapSolarPowerArray | mapGeothermalPlant
        | mapCommonOreMine | mapRareOreMine | mapMagmaWell
        | mapGuardPost | mapLightTower => false,
        _ => true,
    }
}

/// C++: Rechteck rechteckUm(int typ, int cx, int cy)  (op2_ki.hpp:758)
pub fn rechteckUm(welt: &mut dyn Welt, typ: i32, cx: i32, cy: i32) -> Rechteck {        // aus der Mitte (Location)
    let g = grundflaeche(welt, typ);
    Rechteck { x0: cx - g.w / 2, y0: cy - g.h / 2, w: g.w, h: g.h, stutzen: hatStutzen(welt, typ) }
}

// Alle eigenen Gebaeude und geplanten Plaetze (gesperrte Plaetze als Hindernis ohne Stutzen).
/// C++: int hindernisse(const Zustand& z, Rechteck* out, int max)  (op2_ki.hpp:764)
pub fn hindernisse(welt: &mut dyn Welt, z: &Zustand, out: &mut [Rechteck], max: i32) -> i32 {
    let mut n: i32 = 0;
    for &t in GEBAEUDE.iter() {
        for u in welt.PlayerBuildingEnum(z.spieler, t) {
            if n >= max { break; }
            let l = welt.Location(u);
            let (ex, ey) = (edX(welt, z, l), edY(welt, z, l));
            out[n as usize] = rechteckUm(welt, t, ex, ey);
            n += 1;
        }
    }
    let mut i = 0;
    while i < z.anzahlPlaetze && n < max {
        let q = &z.plaetze[i as usize];
        i += 1;
        if q.zustand == P_FREI { continue; }
        let g = grundflaeche(welt, q.typ as i32);
        let st = q.zustand != P_GESPERRT && hatStutzen(welt, q.typ as i32);
        out[n as usize] = Rechteck { x0: q.x0 as i32, y0: q.y0 as i32, w: g.w, h: g.h, stutzen: st };
        n += 1;
    }
    // Missionsmodus: reservierter Platz des Raumhafens samt Festungsrand
    if n < max { if let Some(r) = super::super::bezirke::raumhafenReserve(welt, z) { out[n as usize] = r; n += 1; } }
    // Bebauungsplan (Rust): reservierte Plaetze sind fuer andere Gebaeude belegt (ohne Stutzen -- dort steht noch nichts)
    for q in super::vorplan::reserviert(z) {
        if n >= max { break; }
        out[n as usize] = Rechteck { x0: q.x0, y0: q.y0, w: q.w, h: q.h, stutzen: false };
        n += 1;
    }
    n
}

/// C++: bool ueberlappt(int ax, int ay, int aw, int ah, int bx, int by, int bw, int bh)  (op2_ki.hpp:783)
pub fn ueberlappt(welt: &mut dyn Welt, ax: i32, ay: i32, aw: i32, ah: i32, bx: i32, by: i32, bw: i32, bh: i32) -> bool {
    ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah
}

// Grundflaeche bebaubar, Rand befahrbar, keine Bake/kein Wrack, nichts Geplantes im Weg,
// kein Roehrenstutzen unter der Grundflaeche.
/// C++: bool platzFrei(const Zustand& z, int x0, int y0, int w, int h, const Rechteck* hin, int nh, const char** grund)  (op2_ki.hpp:789)
pub fn platzFrei(welt: &mut dyn Welt, z: &Zustand, x0: i32, y0: i32, w: i32, h: i32, hin: &[Rechteck], nh: i32, grund: &mut &'static str) -> bool {
    if !erreichbar(welt, z, x0, y0, w, h) { *grund = "unerreichbar"; return false; }
    // Rust: aufgezeichnete, noch nicht gebaute Mauern samt Rand und Tore freihalten (sonst raeumt der Bau die Mauer ab)
    if super::super::mauer::geplantIn(z, x0 - 1, y0 - 1, w + 2, h + 2) { *grund = "Mauer geplant"; return false; }
    if super::super::mauer::torIn(z, x0, y0, w, h) { *grund = "Tor"; return false; }
    platzFreiOhneMauer(welt, z, x0, y0, w, h, hin, nh, grund)
}

/// platzFrei ohne die Pruefung auf geplante Mauern (fuer Mauerkacheln selbst: sie duerfen aneinander liegen)
pub fn platzFreiOhneMauer(welt: &mut dyn Welt, z: &Zustand, x0: i32, y0: i32, w: i32, h: i32, hin: &[Rechteck], nh: i32, grund: &mut &'static str) -> bool {
    if !aufKarte(welt, z, x0 - 1, y0 - 1, 2) || !aufKarte(welt, z, x0 + w, y0 + h, 2) { *grund = "Kartenrand"; return false; }
    // Baken freihalten: Grundflaeche der Mine (2x1, Bake rechts) samt Rand
    for i in 0..z.kbN as usize {
        if ueberlappt(welt, x0 - 1, y0 - 1, w + 2, h + 2, z.kbX[i] as i32 - 2, z.kbY[i] as i32 - 1, 4, 3) { *grund = "Bake"; return false; }
    }
    for i in 0..nh as usize {
        let r = &hin[i];
        // Abstand mindestens 1 Kachel: der Rand des einen darf die Grundflaeche des anderen nicht beruehren
        if ueberlappt(welt, x0 - 1, y0 - 1, w + 2, h + 2, r.x0, r.y0, r.w, r.h) { *grund = "Gebaeude/Plan"; return false; }
        if r.stutzen {
            let (sx, sy) = (r.x0 + r.w, r.y0 + r.h / 2);        // rechter Stutzen
            let (ux, uy) = (r.x0 + r.w / 2, r.y0 + r.h);        // unterer Stutzen
            if ueberlappt(welt, x0, y0, w, h, sx, sy, 1, 1) || ueberlappt(welt, x0, y0, w, h, ux, uy, 1, 1) {
                *grund = "Roehrenstutzen"; return false;
            }
        }
    }
    for y in (y0 - 1)..=(y0 + h) {
        for x in (x0 - 1)..=(x0 + w) {
            let c = zelle(welt, z, x, y);
            let innen = x >= x0 && x < x0 + w && y >= y0 && y < y0 + h;
            if if innen { !zelleBebaubar(welt, c) } else { !zelleBefahrbar(welt, c) } {
                *grund = if innen { "Untergrund" } else { "Rand" }; return false;
            }
        }
    }
    let rr = Rect::new(engine(welt, z, x0 - 1, y0 - 1), engine(welt, z, x0 + w, y0 + h));
    for u2 in welt.InRectEnumerator(rr) {
        if welt.IsVehicle(u2) {
            // Fahrende Fahrzeuge sind gleich weg; ein stehendes in der Grundflaeche blockiert den Bau.
            let l = welt.Location(u2);
            let (vx, vy) = (edX(welt, z, l), edY(welt, z, l));
            if vx >= x0 && vx < x0 + w && vy >= y0 && vy < y0 + h && welt.GetCurAction(u2) == moDone && welt.GetType(u2) != mapConVec {
                *grund = "Fahrzeug steht im Weg"; return false;
            }
            continue;
        }
        *grund = if welt.IsBuilding(u2) { "Gebaeude" } else { "Bake/Wrack" };
        return false;
    }
    true
}

// Neuen Eintrag in z.plaetze anlegen (freigewordene wiederverwenden). -1 = voll.
/// C++: int platzEintrag(Zustand& z, int typ, int x0, int y0)  (op2_ki.hpp:833)
pub fn platzEintrag(welt: &mut dyn Welt, z: &mut Zustand, typ: i32, x0: i32, y0: i32) -> i32 {
    let mut k: i32 = -1;
    for i in 0..z.anzahlPlaetze as usize {
        if z.plaetze[i].zustand == P_FREI && z.plaetze[i].convec == 0 { k = i as i32; break; }
    }
    // Missionsmodus: Liste voll -- den aeltesten Eintrag eines stehenden Gebaeudes wiederverwenden (das Gebaeude selbst zaehlt in
    // hindernisse ueber das Spiel), sonst einen seit 30000 Ticks gesperrten Platz. Vorher blieben stehende und gesperrte Plaetze
    // fuer immer belegt: nach 48 Plaetzen baute die KI nichts mehr (cm03 ab Tick ~245 000, Hungersnot). Im Mehrspieler-Modus
    // (noch) nicht, damit er gleich bleibt.
    if k < 0 && z.anzahlPlaetze >= MAX_PLAETZE as i32 && super::super::modus::mission(z) {
        let t = welt.Tick();
        let n = z.anzahlPlaetze as usize;
        k = (0..n).filter(|&i| z.plaetze[i].zustand == P_STEHT).min_by_key(|&i| z.plaetze[i].seit).map(|i| i as i32)
            .or_else(|| (0..n).filter(|&i| z.plaetze[i].zustand == P_GESPERRT && t - z.plaetze[i].seit > 30000).min_by_key(|&i| z.plaetze[i].seit).map(|i| i as i32))
            .unwrap_or(-1);
    }
    if k < 0 {
        if z.anzahlPlaetze >= MAX_PLAETZE as i32 { return -1; }
        k = z.anzahlPlaetze;
        z.anzahlPlaetze += 1;
    }
    let t = welt.Tick();
    let p = &mut z.plaetze[k as usize];
    p.x0 = x0 as i16; p.y0 = y0 as i16; p.typ = typ as i16; p.zustand = P_FREI;
    p.convec = 0; p.seit = t; p.versuche = 0; p.raster = -1;
    k
}

// Tokamak und Grosslabor explodieren mit Flaechenschaden: moeglichst 3 Felder Luft zu anderen Gebaeuden. Strafe je
// fehlendem Feld; als Wertung, nicht als Verbot.
/// C++: int explosionsStrafe(const Zustand& z, int typ, int x0, int y0, int w, int h)  (op2_ki.hpp:850)
pub fn explosionsStrafe(welt: &mut dyn Welt, z: &Zustand, typ: i32, x0: i32, y0: i32, w: i32, h: i32) -> i32 {
    let selbst = typ == mapTokamak || typ == mapAdvancedLab;
    let mut strafe = 0;
    let luecke = |bx: i32, by: i32, bw: i32, bh: i32| -> i32 {
        let gr = |a: i32, b: i32| if a > b { a } else { b };
        let dx = gr(0, gr(bx - (x0 + w), x0 - (bx + bw)));
        let dy = gr(0, gr(by - (y0 + h), y0 - (by + bh)));
        gr(dx, dy)
    };
    for &t in GEBAEUDE.iter() {
        if t == mapGuardPost { continue; }
        let expl = t == mapTokamak || t == mapAdvancedLab;
        if !selbst && !expl { continue; }
        for u in welt.PlayerBuildingEnum(z.spieler, t) {
            if !welt.IsLive(u) { continue; }
            let (lx, ly) = (welt.Location(u), welt.Location(u));
            let (ex, ey) = (edX(welt, z, lx), edY(welt, z, ly));
            let r = rechteckUm(welt, t, ex, ey);
            let lu = luecke(r.x0, r.y0, r.w, r.h);
            if lu < 3 { strafe += (3 - lu) * 300; }
        }
    }
    for i in 0..z.anzahlPlaetze as usize {
        let q = &z.plaetze[i];
        if q.zustand == P_FREI || q.typ as i32 == mapGuardPost { continue; }
        if !selbst && q.typ as i32 != mapTokamak && q.typ as i32 != mapAdvancedLab { continue; }
        let g = grundflaeche(welt, q.typ as i32);
        let lu = luecke(q.x0 as i32, q.y0 as i32, g.w, g.h);
        if lu < 3 { strafe += (3 - lu) * 300; }
    }
    strafe
}

// Platz fuer `typ` suchen. Liefert Index in z.plaetze oder -1.
/// C++: int platzWaehlen(Zustand& z, int typ)  (op2_ki.hpp:882)
/// Bauplatz fuer `typ` waehlen: Bebauungsplan (grosse Gebaeude), am Roehrenstutzen, fern (PAR platzFern), Plan als
/// Rueckfall. Liefert den Platz-Eintrag oder -1. Rust 2026-11-18 aufgeteilt (Verhalten gleich).
thread_local! { static OHNE_PLAN: std::cell::Cell<bool> = std::cell::Cell::new(false); }
/// Aussenposten der Mission: Plaetze ohne den Bebauungsplan der Hauptbasis waehlen (sonst landet die Schmelze daheim)
pub fn ohnePlan(an: bool) { OHNE_PLAN.with(|c| c.set(an)); }

pub fn platzWaehlen(welt: &mut dyn Welt, z: &mut Zustand, typ: i32) -> i32 {
    let planErlaubt = !OHNE_PLAN.with(|c| c.get());
    // Missionsmodus (PAR bezirke): die Hauptbasis waechst in Bezirken (bezirke.rs); Kern, Tuerme und Minen wie bisher
    if PAR(z.spieler).bezirke >= 0.5 && typ != mapGuardPost && typ != mapCommonOreMine && typ != mapRareOreMine {
        if planErlaubt && typ == mapSpaceport { if let Some(i) = super::super::bezirke::raumhafenPlatz(welt, z, typ) { return i; } }
        if planErlaubt { if let Some(i) = super::super::bezirke::platz(welt, z, typ) { return i; } }
        // Kern (und die zweite Basis): luftig mit kurzen Roehren
        if PAR(z.spieler).kernLuft > 0.0 { if let Some(i) = super::super::bezirke::kernPlatz(welt, z, typ) { return i; } }
    }
    // Bebauungsplan (Rust): grosse Gebaeude zuerst auf ihren reservierten Platz; kleine nehmen weiter den besten
    // Stutzenplatz (kuerzeste Roehre, dicht an der Mitte) und weichen nur auf Planplaetze aus, wenn sonst nichts frei ist.
    let gross = { let g = grundflaeche(welt, typ); g.w * g.h > 6 };
    if planErlaubt && gross && typ != mapCommonOreMine && typ != mapRareOreMine && typ != mapGuardPost {
        if let Some((x0, y0)) = super::vorplan::ausPlan(welt, z, typ) {
            let i = platzEintrag(welt, z, typ, x0, y0);
            if i >= 0 { melde(welt, z, "plan", typ, x0, y0, &format!("Platz aus dem Bebauungsplan ({},{})", x0, y0)); }
            return i;
        }
    }
    // static Rechteck hin[160] -> lokaler Puffer
    let mut hin = [Rechteck::default(); 160];
    let nh = hindernisse(welt, z, &mut hin, 160);
    let (mut gefunden, mut bestX, mut bestY, verworfen, grund) = platzAmStutzen(welt, z, typ, &hin, nh);
    // Erweiterung (Rust, PAR.platzFern): kein Platz am Stutzen -> naechster freier Platz im Umkreis; anschluss() legt die Roehre.
    // Enge Basis (mp2_05: Ecke zwischen Klippen) bekam sonst nie Grosslabor und Fahrzeugfabrik.
    if !gefunden && PAR(z.spieler).platzFern > 0.0 && hatStutzen(welt, typ) && typ != mapCommandCenter {
        if let Some((x, y)) = platzFernSuchen(welt, z, typ, &hin, nh) { gefunden = true; bestX = x; bestY = y; }
    }
    if planErlaubt && !gefunden && !gross && super::vorplan::an(z) && !super::vorplan::ausgesetzt() && typ != mapGuardPost {
        if let Some((x0, y0)) = super::vorplan::ausPlan(welt, z, typ) {
            let i = platzEintrag(welt, z, typ, x0, y0);
            if i >= 0 { melde(welt, z, "plan", typ, x0, y0, &format!("Platz aus dem Bebauungsplan ({},{}) -- sonst keiner frei", x0, y0)); }
            return i;
        }
    }
    if planErlaubt && !gefunden && super::vorplan::an(z) && !super::vorplan::ausgesetzt() {
        // Bebauungsplan: die Reservierungen lenken nur um, sie sperren nie ganz -- ohne sie noch einmal suchen (der Plan
        // stellt sich beim naechsten Durchlauf um das neue Gebaeude herum neu auf).
        super::vorplan::aussetzen(true);
        let i = platzWaehlen(welt, z, typ);
        super::vorplan::aussetzen(false);
        if i >= 0 { super::vorplan::neuPlanen(z); }
        return i;
    }
    if !gefunden {
        if selten(welt, z, 6, 1000) {
            let (bx, by) = (z.bx, z.by);
            melde(welt, z, "fehler", typ, bx, by, &format!("kein Bauplatz mit Anschluss ({} verworfen, zuletzt: {})", verworfen, grund));
        }
        return -1;
    }
    platzEintrag(welt, z, typ, bestX, bestY)
}

/// Am Stutzen eines vorhandenen/geplanten Gebaeudes (immer 1 Kachel Abstand); Kommandozentrale nah an der Mitte,
/// Kraftwerke im Ring um die Mitte (Tokamak feindabgewandt). Liefert (gefunden, x, y, verworfen, letzter Grund).
pub fn platzAmStutzen(welt: &mut dyn Welt, z: &mut Zustand, typ: i32, hin: &[Rechteck; 160], nh: i32) -> (bool, i32, i32, i32, &'static str) {
    let g = grundflaeche(welt, typ);
    let mut grund: &'static str = "";
    let mut verworfen = 0;
    let mut bestX = 0; let mut bestY = 0;
    let mut besteWertung: i32 = 1 << 30;
    let mut gefunden = false;
    {
        let zr: &Zustand = &*z;
        let mut pruefen = |welt: &mut dyn Welt, x0: i32, y0: i32, mut wertung: i32| {
            if wertung >= besteWertung { return; }
            if !platzFrei(welt, zr, x0, y0, g.w, g.h, hin, nh, &mut grund) { verworfen += 1; return; }
            wertung += explosionsStrafe(welt, zr, typ, x0, y0, g.w, g.h);
            if wertung >= besteWertung { return; }
            besteWertung = wertung; bestX = x0; bestY = y0; gefunden = true;
        };
        let (mx, my) = (zr.bx, zr.by);
        // Kraftwerke auf die feindabgewandte Seite
        let (mut fux, mut fuy) = (0.0f64, 0.0f64);
        let hinten = typ == mapTokamak && zr.feindGemeldet != 0;
        if hinten { feindRichtung(welt, zr, mx, my, &mut fux, &mut fuy); }
        if !hatStutzen(welt, typ) || typ == mapCommandCenter {
            // Kommandozentrale: so nah wie moeglich an die Basismitte. Kraftwerke: Ring um die Mitte.
            for dy in -12..=12 {
                for dx in -12..=12 {
                    let (x0, y0) = (mx + dx - g.w / 2, my + dy - g.h / 2);
                    let d2 = dx * dx + dy * dy;
                    let mut w: i32;
                    if typ == mapCommandCenter { w = d2; }
                    else { let d = if d2 > 0 { d2 } else { 0 }; w = (d - 49) * (d - 49) / 8 + d / 4; }  // Ring um ~7
                    if hinten { w += (40.0 * (dx as f64 * fux + dy as f64 * fuy)) as i32; }             // zum Feind hin teurer
                    pruefen(welt, x0, y0, w);
                }
            }
        } else {
            // Gebaeude mit Anschluss: an einen Stutzen eines vorhandenen/geplanten Gebaeudes, oder mit dem
            // eigenen Stutzen an dessen Seite -- immer genau 1 Kachel Abstand.
            let wert = |welt: &mut dyn Welt, x0: i32, y0: i32, ausrichtung: i32| -> i32 {
                let (cx, cy) = (x0 + g.w / 2, y0 + g.h / 2);
                abstand2(welt, cx, cy, mx, my) * 4 + ausrichtung
            };
            for i in 0..nh as usize {
                let a = hin[i];
                if !a.stutzen { continue; }
                let (acx, acy) = (a.x0 + a.w / 2, a.y0 + a.h / 2);
                // rechts von A (A's rechter Stutzen trifft unsere linke Seite)
                for y0 in (acy - g.h + 1)..=acy {
                    let v = wert(welt, a.x0 + a.w + 1, y0, (y0 - a.y0).abs());
                    pruefen(welt, a.x0 + a.w + 1, y0, v);
                }
                // unter A (A's unterer Stutzen trifft unsere Oberseite)
                for x0 in (acx - g.w + 1)..=acx {
                    let v = wert(welt, x0, a.y0 + a.h + 1, (x0 - a.x0).abs());
                    pruefen(welt, x0, a.y0 + a.h + 1, v);
                }
                // links von A (unser rechter Stutzen trifft A's linke Seite)
                for y0 in (a.y0 - g.h / 2)..=(a.y0 + a.h - 1 - g.h / 2) {
                    let v = wert(welt, a.x0 - 1 - g.w, y0, (y0 - a.y0).abs());
                    pruefen(welt, a.x0 - 1 - g.w, y0, v);
                }
                // ueber A (unser unterer Stutzen trifft A's Oberseite)
                for x0 in (a.x0 - g.w / 2)..=(a.x0 + a.w - 1 - g.w / 2) {
                    let v = wert(welt, x0, a.y0 - 1 - g.h, (x0 - a.x0).abs());
                    pruefen(welt, x0, a.y0 - 1 - g.h, v);
                }
            }
        }
    }
    (gefunden, bestX, bestY, verworfen, grund)
}

/// Erweiterung (Rust, PAR platzFern): kein Platz am Stutzen -> naechster freier Platz im Umkreis; anschluss() legt die
/// Roehre. Neues Modell: jeder Platz traegt die Laenge seines Anschlusses ans Netz.
pub(super) fn platzFernSuchen(welt: &mut dyn Welt, z: &mut Zustand, typ: i32, hin: &[Rechteck; 160], nh: i32) -> Option<(i32, i32)> {
    let g = grundflaeche(welt, typ);
    let r = PAR(z.spieler).platzFern as i32;
    let (mx, my) = (z.bx, z.by);
    let mut best = (1i32 << 30, 0, 0);
    // Neues Modell: die Roehre kostet Zeit (der Earthworker legt sie Kachel fuer Kachel) -- jeder Platz traegt die
    // Laenge seines Anschlusses ans Netz, nicht nur den Abstand zur Mitte.
    let abst = if kampfModellNeu() { Some(super::bau::netzAbstand(welt, z, hin, nh)) } else { None };
    let kachel = PAR(z.spieler).roehreKachel;
    let bb = if z.breite < 256 { z.breite } else { 256 };
    for dy in -r..=r { for dx in -r..=r {
        let d2 = dx * dx + dy * dy;
        if d2 > r * r || d2 >= best.0 { continue; }
        let (x0, y0) = (mx + dx - g.w / 2, my + dy - g.h / 2);
        let mut roehre = 0;
        if let Some(a) = &abst {
            // kuerzester Anschluss ueber eine Seitenkachel der Grundflaeche
            let rr = Rechteck { x0, y0, w: g.w, h: g.h, stutzen: true };
            let mut m = i32::MAX;
            for y in (y0 - 1)..=(y0 + g.h) { for x in (x0 - 1)..=(x0 + g.w) {
                if x < 0 || y < 0 || x >= bb || y * bb + x >= a.len() as i32 { continue; }
                if !super::bau::anGrundflaeche(welt, &rr, x, y) { continue; }
                let v = a[(y * bb + x) as usize];
                if v >= 0 && v < m { m = v; }
            } }
            if m == i32::MAX { continue; } // vom Netz aus nicht zu erreichen
            roehre = m;
        }
        let vorab = d2 + (roehre as f64 * kachel) as i32;
        if vorab >= best.0 { continue; }
        let mut gr: &'static str = "";
        if !platzFrei(welt, z, x0, y0, g.w, g.h, hin, nh, &mut gr) { continue; }
        let w = vorab + explosionsStrafe(welt, z, typ, x0, y0, g.w, g.h);
        if w < best.0 { best = (w, x0, y0); }
    } }
    if best.0 < (1 << 30) {
        let (bestX, bestY) = (best.1, best.2);
            melde(welt, z, "bau", typ, bestX, bestY, &format!("kein Platz am Roehrenstutzen -- Platz {},{} ohne Anschluss, die Roehre folgt", bestX, bestY));
        return Some((bestX, bestY));
    }
    None
}
