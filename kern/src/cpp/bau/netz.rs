//! Roehrennetz: Wegsuche, Netzkacheln, Abstand zum Netz, Anschluss unverbundener Gebaeude (anschluss).
use super::*;
use super::super::*;
use super::{GEBAEUDE, TECH_RAUMFAHRT, B_ZIEL};

// ---------------------------------------------------------------------------------------------------
// Anschluss / connection: Gebaeude ohne Verbindung zur Kommandozentrale per Roehre anschliessen.

/// C++: bool angeschlossen(UnitEx u)  (op2_ki.hpp:3058)
pub fn angeschlossen(welt: &mut dyn Welt, u: Id) -> bool { welt.HasPower(u) }

// Belegte Kacheln: Grundflaechen aller eigenen Gebaeude und geplanten Plaetze
/// C++: bool belegt(const Rechteck* hin, int nh, int x, int y)  (op2_ki.hpp:3061)
pub fn belegt(welt: &mut dyn Welt, hin: &[Rechteck], nh: i32, x: i32, y: i32) -> bool {
    for i in 0..nh as usize {
        if x >= hin[i].x0 && x < hin[i].x0 + hin[i].w && y >= hin[i].y0 && y < hin[i].y0 + hin[i].h { return true; }
    }
    false
}

// Kachel direkt an einer Seite der Grundflaeche (ohne Ecken). Tile directly beside a side (no corners).
/// C++: bool anGrundflaeche(const Rechteck& r, int x, int y)  (op2_ki.hpp:3068)
pub fn anGrundflaeche(welt: &mut dyn Welt, r: &Rechteck, x: i32, y: i32) -> bool {
    let innenX = x >= r.x0 && x < r.x0 + r.w;
    let innenY = y >= r.y0 && y < r.y0 + r.h;
    (innenX && (y == r.y0 - 1 || y == r.y0 + r.h)) || (innenY && (x == r.x0 - 1 || x == r.x0 + r.w))
}

thread_local! {
    /// PAR roehreAnNetz (Missionsmodus): Netzkacheln, deren Roehren auch als Ziel gelten (gesetzt nur waehrend anschluss)
    static NETZ_ZIEL: std::cell::RefCell<Option<std::rc::Rc<Vec<bool>>>> = const { std::cell::RefCell::new(None) };
    /// dazu die aufgezeichneten, noch nicht gebauten Roehren (auch aus derselben Runde): sonst lief der zweite Turm-Anschluss
    /// parallel neben dem ersten her (cm03 West/Ost, Nutzer 2026-11-22)
    static OFFEN_ZIEL: std::cell::RefCell<Vec<(i16, i16)>> = const { std::cell::RefCell::new(Vec::new()) };
    /// Dock-Kacheln eigener Gebaeude: dort keine Roehre (Missionsmodus; im Original blieben solche Roehren unbebaut liegen)
    static DOCKS: std::cell::RefCell<Vec<(i32, i32)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Dock-Kacheln aller eigenen Gebaeude mit Dock (Fabriken, Schmelzen, Lager, Garage, Raumhafen)
pub fn dockKacheln(welt: &mut dyn Welt, z: &Zustand) -> Vec<(i32, i32)> {
    let mut v = Vec::new();
    for typ in [mapStructureFactory, mapVehicleFactory, mapArachnidFactory, mapConsumerFactory, mapCommonOreSmelter, mapRareOreSmelter,
                mapCommonStorage, mapRareStorage, mapGarage, mapSpaceport] {
        for u in welt.PlayerBuildingEnum(z.spieler, typ) { let d = welt.GetDockLocation(u); if d.x != 0 || d.y != 0 { v.push((edX(welt, z, d), edY(welt, z, d))); } }
    }
    v
}
/// Ist (x, y) gerade als Dock gesperrt (nur waehrend anschluss im Missionsmodus gesetzt)?
pub fn dockGesperrt(x: i32, y: i32) -> bool { DOCKS.with(|d| d.borrow().iter().any(|&(dx, dy)| dx == x && dy == y)) }

thread_local! {
}

// Kuerzester Weg (Breitensuche) vom Rand von `von` zum Rand eines Ziel-Rechtecks. Liefert die Laenge,
// die Kacheln stehen in wx/wy (vom Start zum Ziel). 0 = kein Weg.
/// C++: int wegSuchen(const Zustand& z, const Rechteck& von, const Rechteck* ziele, int nz, const Rechteck* hin, int nh, short* wx, short* wy, bool ohneRoehre  (op2_ki.hpp:3075)
pub fn wegSuchen(welt: &mut dyn Welt, z: &Zustand, von: &Rechteck, ziele: &[Rechteck], nz: i32, hin: &[Rechteck], nh: i32, wx: &mut [i16], wy: &mut [i16], ohneRoehren: bool) -> i32 {
    // static int vor[256*256]; static short qx[], qy[] -> lokale Puffer
    let mut vor = vec![0i32; 256 * 256];
    let mut qx = vec![0i16; 256 * 256];
    let mut qy = vec![0i16; 256 * 256];
    let B = if z.breite < 256 { z.breite } else { 256 };
    let H = if z.hoehe < 256 { z.hoehe } else { 256 };
    for i in 0..(B * H).max(0) as usize { vor[i] = -2; }
    let (mut kopf, mut ende) = (0usize, 0usize);
    let frei = |welt: &mut dyn Welt, x: i32, y: i32| -> bool {
        if !aufKarte(welt, z, x, y, 1) { return false; }
        let c = zelle(welt, z, x, y);
        if ohneRoehren && zelleRoehre(welt, c) { return false; }
        (zelleBebaubar(welt, c) || zelleRoehre(welt, c)) && !belegt(welt, hin, nh, x, y) && !super::super::mauer::geplantIn(z, x, y, 1, 1)
            && !dockGesperrt(x, y)
    };
    for y in (von.y0 - 1)..=(von.y0 + von.h) {
        for x in (von.x0 - 1)..=(von.x0 + von.w) {
            if anGrundflaeche(welt, von, x, y) && frei(welt, x, y) && vor[(y * B + x) as usize] == -2 {
                vor[(y * B + x) as usize] = -1; qx[ende] = x as i16; qy[ende] = y as i16; ende += 1;
            }
        }
    }
    const DX: [i32; 4] = [1, -1, 0, 0];
    const DY: [i32; 4] = [0, 0, 1, -1];
    let netzZiel = NETZ_ZIEL.with(|n| n.borrow().clone());
    while kopf < ende {
        let x = qx[kopf] as i32;
        let y = qy[kopf] as i32;
        kopf += 1;
        // PAR roehreAnNetz: eine Roehre im Netz ist schon das Ziel -- der Weg endet an der naechsten vorhandenen Roehre statt
        // daneben einen eigenen Strang zum Gebaeude zu ziehen (doppelte Roehren, Nutzer 2026-11-22)
        if let Some(nz2) = &netzZiel {
            let i = (y * B + x) as usize;
            let offen = OFFEN_ZIEL.with(|o| o.borrow().iter().any(|&(ox, oy)| ox as i32 == x && oy as i32 == y));
            if !ohneRoehren && (offen || (i < nz2.len() && nz2[i] && { let c = zelle(welt, z, x, y); zelleRoehre(welt, c) })) {
                let (mut n, mut cx, mut cy) = (0i32, x, y);
                while n < MAX_WEG {
                    wx[n as usize] = cx as i16; wy[n as usize] = cy as i16; n += 1;
                    let v = vor[(cy * B + cx) as usize];
                    if v < 0 { break; }
                    cx = v % B; cy = v / B;
                }
                return n;
            }
        }
        for k in 0..nz as usize {
            if anGrundflaeche(welt, &ziele[k], x, y) {
                let (mut n, mut cx, mut cy) = (0i32, x, y);
                while n < MAX_WEG {
                    wx[n as usize] = cx as i16; wy[n as usize] = cy as i16; n += 1;
                    let v = vor[(cy * B + cx) as usize];
                    if v < 0 { break; }
                    cx = v % B; cy = v / B;
                }
                return n;
            }
        }
        for d in 0..4 {
            let nx = x + DX[d];
            let ny = y + DY[d];
            if nx < 0 || ny < 0 || nx >= B || ny >= H || vor[(ny * B + nx) as usize] != -2 || !frei(welt, nx, ny) { continue; }
            vor[(ny * B + nx) as usize] = y * B + x;
            qx[ende] = nx as i16; qy[ende] = ny as i16; ende += 1;
        }
    }
    0
}

// Rust-Erweiterung: Roehrennetz wie der HiveManager (Flutfuellung 0x42EE90 ueber Roehren und eigene Gebaeudeflaechen,
// 4er-Nachbarschaft). Strom verteilt die Engine ohne Anschluss, Arbeiter und Wissenschaftler nur im Netz einer
// Kommandozentrale -- HasPower allein zeigt den Anschluss darum nicht.
/// Kacheln im Netz einer eigenen Kommandozentrale (Index y*B+x, B = min(breite,256)).
pub fn netzKacheln(welt: &mut dyn Welt, z: &Zustand) -> Vec<bool> {
    let B = if z.breite < 256 { z.breite } else { 256 };
    let H = if z.hoehe < 256 { z.hoehe } else { 256 };
    let n = (B * H).max(0) as usize;
    let mut gebaeude = vec![false; n];
    let mut netz = vec![false; n];
    let mut q: Vec<(i32, i32)> = Vec::new();
    for &typ in GEBAEUDE.iter() {
        for u in welt.PlayerBuildingEnum(z.spieler, typ) {
            if !welt.IsLive(u) { continue; }
            let lo = welt.Location(u);
            let (ex, ey) = (edX(welt, z, lo), edY(welt, z, lo));
            let r = rechteckUm(welt, typ, ex, ey);
            for y in r.y0..r.y0 + r.h { for x in r.x0..r.x0 + r.w {
                if x < 0 || y < 0 || x >= B || y >= H { continue; }
                let i = (y * B + x) as usize;
                gebaeude[i] = true;
                if typ == mapCommandCenter && !netz[i] { netz[i] = true; q.push((x, y)); }
            } }
        }
    }
    let mut k = 0;
    while k < q.len() {
        let (x, y) = q[k]; k += 1;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= B || ny >= H { continue; }
            let i = (ny * B + nx) as usize;
            if netz[i] { continue; }
            if !gebaeude[i] { let c = zelle(welt, z, nx, ny); if !zelleRoehre(welt, c) { continue; } }
            netz[i] = true; q.push((nx, ny));
        }
    }
    netz
}

/// Roehrenkacheln von jeder Kachel bis ans Netz (0 = liegt am Netz an oder ist Roehre im Netz, -1 = nicht erreichbar).
/// Breitensuche ueber Kacheln, auf denen eine Roehre liegen kann (bebaubar oder Roehre, nicht unter Gebaeude/Plan),
/// Start an den Seiten der Gebaeude im Netz und auf den Roehren im Netz.
pub fn netzAbstand(welt: &mut dyn Welt, z: &Zustand, hin: &[Rechteck], nh: i32) -> Vec<i32> {
    let netz = netzKacheln(welt, z);
    let B = if z.breite < 256 { z.breite } else { 256 };
    let H = if z.hoehe < 256 { z.hoehe } else { 256 };
    let mut d = vec![-1i32; (B * H).max(0) as usize];
    let mut q: Vec<(i32, i32)> = Vec::new();
    let frei = |welt: &mut dyn Welt, x: i32, y: i32| -> bool {
        if !aufKarte(welt, z, x, y, 1) { return false; }
        let c = zelle(welt, z, x, y);
        (zelleBebaubar(welt, c) || zelleRoehre(welt, c)) && !belegt(welt, hin, nh, x, y)
    };
    for y in 0..H { for x in 0..B {
        let i = (y * B + x) as usize;
        if !netz[i] { continue; }
        let c = zelle(welt, z, x, y);
        if zelleRoehre(welt, c) { if d[i] < 0 { d[i] = 0; q.push((x, y)); } continue; }
        // Gebaeudekachel im Netz: freie Nachbarn sind Anschlusskacheln (eine Roehre dort haengt am Netz)
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= B || ny >= H { continue; }
            let j = (ny * B + nx) as usize;
            if netz[j] || d[j] >= 0 || !frei(welt, nx, ny) { continue; }
            d[j] = 1; q.push((nx, ny));
        }
    }}
    let mut k = 0;
    while k < q.len() {
        let (x, y) = q[k]; k += 1;
        let i = (y * B + x) as usize;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= B || ny >= H { continue; }
            let j = (ny * B + nx) as usize;
            if d[j] >= 0 || !frei(welt, nx, ny) { continue; }
            let c = zelle(welt, z, nx, ny);
            d[j] = d[i] + if zelleRoehre(welt, c) { 0 } else { 1 };
            q.push((nx, ny));
        }
    }
    d
}

/// Liegt eine Gebaeudeflaeche im Netz (siehe netzKacheln)?
pub fn imNetz(z: &Zustand, netz: &[bool], r: &Rechteck) -> bool {
    let B = if z.breite < 256 { z.breite } else { 256 };
    let H = if z.hoehe < 256 { z.hoehe } else { 256 };
    for y in r.y0..r.y0 + r.h { for x in r.x0..r.x0 + r.w {
        if x >= 0 && y >= 0 && x < B && y < H && netz[(y * B + x) as usize] { return true; }
    } }
    false
}

/// C++: void anschluss(Zustand& z, const Lage& l)  (op2_ki.hpp:3115)
/// Unverbundene Gebaeude und geplante Plaetze samt Hindernissen und Netzzielen (fuer die Wegsuche der Roehren)
pub(super) struct Lose { hin: [Rechteck; 160], nh: i32, ziele: [Rechteck; 64], nz: usize, lose: [Id; 8], loseR: [Rechteck; 8], nl: usize, netz: Option<Vec<bool>> }

/// C++: void anschluss(Zustand& z, const Lage& l). Roehren zu unverbundenen Gebaeuden. Rust 2026-11-18 aufgeteilt (Verhalten gleich).
pub fn anschluss(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    let t = welt.Tick();
    if t < z.naechsterAnschluss || z.bauAn == 0 { return; }
    z.naechsterAnschluss = t + 200;
    let offen = roehrenPruefen(welt, z, t);
    if offen != 0 && std::env::var("OP2KI_ROEHRE_DIAG").is_ok() && t % 4000 < 200 {
        let v: Vec<String> = (0..z.anzahlRoehren.max(0) as usize).filter(|&i| z.roehren[i].zustand == 1).map(|i| { let r = z.roehren[i];
            let lo = engine(welt, z, r.x1 as i32, r.y1 as i32); let n = welt.LocationEnumerator(lo).len();
            format!("{},{} seit {} Einheiten {}", r.x1, r.y1, t - r.seit, n) }).collect();
        melde(welt, z, "diagnose", mapTube, z.bx, z.by, &format!("offene Roehren: {}", v.join("; ")));
    }
    if offen != 0 {
        if selten(welt, z, 19, 3000) {
            melde(welt, z, "warte", mapEarthworker, z.bx, z.by, &format!("{} Roehrenkacheln aufgezeichnet, noch nicht gebaut (Erz {})", offen, l.erz));
        }
        return;
    }
    let lo = loseErfassen(welt, z, l);
    // PAR bauReihe: nichts offen, nichts anzuschliessen -- Luft fuer die wartenden Mauern
    if lo.nl == 0 && PAR(z.spieler).bauReihe >= 0.5 { super::super::mauer::freigeben(welt, z); }
    if lo.nl == 0 || lo.nz == 0 { return; }
    let anNetz = PAR(z.spieler).roehreAnNetz > 0.0;
    if anNetz {
        let nk = match &lo.netz { Some(v) => v.clone(), None => netzKacheln(welt, z) }; NETZ_ZIEL.with(|n| *n.borrow_mut() = Some(std::rc::Rc::new(nk)));
        let d = dockKacheln(welt, z); DOCKS.with(|x| *x.borrow_mut() = d);
    }
    for j in anschlussReihe(welt, z, &lo) {
        if anNetz {
            let v: Vec<(i16, i16)> = (0..z.anzahlRoehren.max(0) as usize).filter(|&i| z.roehren[i].zustand == 1).map(|i| (z.roehren[i].x1, z.roehren[i].y1)).collect();
            OFFEN_ZIEL.with(|o| *o.borrow_mut() = v);
        }
        anschliessen(welt, z, l, t, &lo, j);
    }
    if anNetz { NETZ_ZIEL.with(|n| *n.borrow_mut() = None); OFFEN_ZIEL.with(|o| o.borrow_mut().clear()); DOCKS.with(|x| x.borrow_mut().clear()); }
    // Erledigte Eintraege vorn entfernen, damit die Liste nicht voll laeuft
    let mut k = 0usize;
    for i in 0..z.anzahlRoehren as usize { if z.roehren[i].zustand != 2 { z.roehren[k] = z.roehren[i]; k += 1; } }
    z.anzahlRoehren = k as i32;
}

/// Laufende Roehrenauftraege: fertig, sobald die Kachel eine Roehre ist. Gebaut wird nur ueber die BuildingGroup (RecordTube):
/// der fruehere Rueckfall -- DoBuildWall mit einem 1x1-Rechteck direkt an den Earthworker -- pfluegte im Original die ganze
/// Spalte bis zum Kartenrand, und dort hing die Wegsuche des Spiels ~143 s (Harness on4_03 Seed 1, Nutzer 2026-11-20: entfernt)
pub(super) fn roehrenPruefen(welt: &mut dyn Welt, z: &mut Zustand, t: i32) -> i32 {
    // Missionsmodus: im Original blieben ganze Gruppen aufgezeichneter Roehren ueber 100 000 Ticks liegen (die Earthworker standen
    // daneben); weil anschluss auf offene Roehren wartet, blieben dahinter alle neuen Gebaeude ohne Personal (cm03, Bau-Stillstand
    // ab ~200 000). Nach 12 000 Ticks freigeben: anschluss plant sie neu und zeichnet sie erneut auf.
    if super::super::modus::mission(z) {
        let mut weg = 0;
        for i in 0..z.anzahlRoehren.max(0) as usize {
            if z.roehren[i].zustand == 1 && t - z.roehren[i].seit > 12000 {
                let c = zelle(welt, z, z.roehren[i].x1 as i32, z.roehren[i].y1 as i32);
                if !zelleRoehre(welt, c) { z.roehren[i].zustand = 2; weg += 1; }
            }
        }
        if weg > 0 { melde(welt, z, "warte", mapEarthworker, z.bx, z.by, &format!("{} Roehrenkacheln seit ueber 12000 Ticks offen -- neu geplant", weg)); }
    }
    let mut offen = 0;
    for i in 0..z.anzahlRoehren as usize {
        let mut r = z.roehren[i];
        'b: {
            if r.zustand != 1 { break 'b; }
            let c = zelle(welt, z, r.x1 as i32, r.y1 as i32);
            if zelleRoehre(welt, c) {
                r.zustand = 2;
                melde(welt, z, "fertig", mapEarthworker, r.x1 as i32, r.y1 as i32,
                      &format!("Roehre gebaut (BuildingGroup, nach {} Ticks)", t - r.seit));
                break 'b;
            }
            offen += 1;
        }
        z.roehren[i] = r;
    }
    offen
}

/// Unverbundene fertige Gebaeude (hoechstens 8) und, im neuen Modell, geplante Plaetze ohne Anschluss
pub(super) fn loseErfassen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) -> Lose {
    let mut hin = [Rechteck::default(); 160];
    let nh = hindernisse(welt, z, &mut hin, 160);
    let mut ziele = [Rechteck::default(); 64];
    let mut nz = 0usize;
    let mut lose = [0 as Id; 8];
    let mut loseR = [Rechteck::default(); 8];
    let mut nl = 0usize;
    // Neues Modell: Anschluss aus dem Roehrennetz statt aus HasPower (ein Gebaeude ohne Roehre hat trotzdem Strom).
    let netz = if kampfModellNeu() { Some(netzKacheln(welt, z)) } else { None };
    let mut tuerme: Vec<(Id, Rechteck)> = Vec::new();
    for &typ in GEBAEUDE.iter() {
        for u in welt.PlayerBuildingEnum(z.spieler, typ) {
            let lo = welt.Location(u);
            let (ex, ey) = (edX(welt, z, lo), edY(welt, z, lo));
            let r = rechteckUm(welt, typ, ex, ey);
            // Rust (PAR turmRoehre): Wachtuerme haben keinen Stutzen, brauchen aber die Roehre zur Kommandozentrale (ohne sie
            // weniger Schaden, siehe turmNetz) -- jeder unverbundene fertige Turm wird wie ein loses Gebaeude angeschlossen.
            if typ == mapGuardPost && PAR(z.spieler).turmRoehre >= 0.5 {
                if welt.GetCurAction(u) != moDone || welt.IsBusy(u) || turmAngeschlossen(welt, u) { continue; }
                // PAR bauReihe: Tuerme erst, wenn kein Gebaeude mit Personal mehr auf seinen Anschluss wartet (unten)
                if PAR(z.spieler).bauReihe >= 0.5 { if tuerme.len() < 8 { tuerme.push((u, r)); } continue; }
                if nl >= 8 { continue; }
                lose[nl] = u; loseR[nl] = r; nl += 1; continue;
            }
            let verbunden = match &netz { Some(n) => imNetz(z, n, &r), None => angeschlossen(welt, u) };
            if typ == mapCommandCenter || (hatStutzen(welt, typ) && verbunden) { if nz < 64 { ziele[nz] = r; nz += 1; } continue; }
            if !hatStutzen(welt, typ) || nl >= 8 { continue; }
            // Nur fertige, ruhige Gebaeude: im Bau stehen sie auf Aktion 6, beschaeftigt und ohne Strom -- KEIN fehlender Anschluss.
            if welt.GetCurAction(u) != moDone || welt.IsBusy(u) { continue; }
            if netz.is_none() {
                let bedarfStrom = welt.GetPowerRequired(typ, z.spieler);
                if bedarfStrom <= 0 { continue; } // Lager u.ae.: nie Strom, nicht erkennbar
                if l.stromFrei < bedarfStrom { continue; } // fehlt wirklich Strom
            }
            lose[nl] = u; loseR[nl] = r; nl += 1;
        }
    }
    // Neues Modell: auch geplante Plaetze ohne Anschluss (platzFern) -- die Roehre wird gleich mit dem Platz aufgezeichnet,
    // nicht erst, wenn das Gebaeude fertig ist (sonst steht es nach dem Aufbau noch tausende Ticks ohne Personal).
    if let Some(n) = &netz {
        for i in 0..z.anzahlPlaetze as usize {
            if nl >= 8 { break; }
            let q = z.plaetze[i];
            if q.zustand != P_FAHRT && q.zustand != P_BAUT { continue; }
            let typ = q.typ as i32;
            if !hatStutzen(welt, typ) || typ == mapCommandCenter { continue; }
            let g = grundflaeche(welt, typ);
            let r = Rechteck { x0: q.x0 as i32, y0: q.y0 as i32, w: g.w, h: g.h, stutzen: true };
            if imNetz(z, n, &r) { continue; }
            // liegt schon ein Netzgebaeude daneben (Stutzen an Stutzen), schliesst es sich beim Bau selbst an
            let mut nachbar = false;
            for y in (r.y0 - 1)..=(r.y0 + r.h) { for x in (r.x0 - 1)..=(r.x0 + r.w) {
                if anGrundflaeche(welt, &r, x, y) && x >= 0 && y >= 0 && x < z.breite.min(256) && y < z.hoehe.min(256)
                    && n[(y * z.breite.min(256) + x) as usize] { nachbar = true; }
            } }
            if nachbar { continue; }
            lose[nl] = 0; loseR[nl] = r; nl += 1;
        }
    }
    if nl == 0 {
        for (u, r) in tuerme { if nl >= 8 { break; } lose[nl] = u; loseR[nl] = r; nl += 1; }
    }
    Lose { hin, nh, ziele, nz, lose, loseR, nl, netz }
}

/// Alle unverbundenen in einer Runde; neues Modell: kuerzeste Anschluesse zuerst (die BuildingGroup legt die Kacheln in
/// Aufzeichnungsreihenfolge, ein langer Weg haelt sonst alle kurzen auf)
pub(super) fn anschlussReihe(welt: &mut dyn Welt, z: &mut Zustand, lo: &Lose) -> Vec<usize> {
    let Lose { hin, nh, ziele, nz, loseR, nl, netz, .. } = lo;
    let (nh, nz, nl) = (*nh, *nz, *nl);
    let mut reihe: Vec<usize> = (0..nl).collect();
    if netz.is_some() {
        let mut laenge = [0i32; 8];
        for j in 0..nl {
            let mut wx = [0i16; MAX_WEG as usize];
            let mut wy = [0i16; MAX_WEG as usize];
            let n = wegSuchen(welt, z, &loseR[j], ziele, nz as i32, hin, nh, &mut wx, &mut wy, false);
            laenge[j] = if n > 0 { n } else { i32::MAX };
        }
        reihe.sort_by_key(|&j| laenge[j]);
    }
    reihe
}

/// Einen Anschluss suchen und die Roehrenkacheln aufzeichnen
pub(super) fn anschliessen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32, lo: &Lose, j: usize) {
    let Lose { hin, nh, ziele, nz, lose, loseR, .. } = lo;
    let (nh, nz) = (*nh, *nz);
    let mut wx = [0i16; MAX_WEG as usize];
    let mut wy = [0i16; MAX_WEG as usize];
    let mut n = wegSuchen(welt, z, &loseR[j], ziele, nz as i32, hin, nh, &mut wx, &mut wy, false);
    // Liegt der Weg schon ganz auf Roehrenkacheln und das Gebaeude hat trotzdem keinen Strom, einen
    // zweiten Weg ueber freie Kacheln suchen (Rueckfall, falls eine Roehre selbst nicht am Netz haengt).
    let mut alleRoehre = n > 0;
    let mut i = 0;
    while i < n && alleRoehre {
        let c = zelle(welt, z, wx[i as usize] as i32, wy[i as usize] as i32);
        if !zelleRoehre(welt, c) { alleRoehre = false; }
        i += 1;
    }
    if alleRoehre && PAR(z.spieler).roehreAnNetz > 0.0 {
        // PAR roehreAnNetz: liegt der Weg schon ganz auf Roehren, keinen zweiten Strang daneben ziehen (das ergab die doppelten
        // Roehren, vor allem zu Tuermen, deren Anschlusspruefung anders zaehlt als das Netz)
        return;
    }
    if alleRoehre {
        let n2 = wegSuchen(welt, z, &loseR[j], ziele, nz as i32, hin, nh, &mut wx, &mut wy, true);
        if n2 > 0 { n = n2; } else { n = 0; }
    }
    let typ = if lose[j] != 0 { welt.GetType(lose[j]) } else { mapTube };
    // geplanter Platz am Stutzen eines Netzgebaeudes: die eine Luecke fuellt der Stutzen beim Bau selbst
    if lose[j] == 0 && n <= 1 { return; }
    if n == 0 {
        if selten(welt, z, 20, 3000) { melde(welt, z, "fehler", typ, loseR[j].x0, loseR[j].y0, "nicht angeschlossen, kein freier Weg fuer eine Roehre"); }
        return;
    }
    let mut neu = 0;
    for i in 0..n as usize {
        let c = zelle(welt, z, wx[i] as i32, wy[i] as i32);
        if zelleRoehre(welt, c) { continue; }
        let mut schon = false;
        for k in 0..z.anzahlRoehren as usize {
            if z.roehren[k].zustand == 1 && z.roehren[k].x1 == wx[i] && z.roehren[k].y1 == wy[i] { schon = true; }
        }
        if schon { continue; }
        let ort = engine(welt, z, wx[i] as i32, wy[i] as i32);
        welt.RecordTube(super::super::zweitbasis::gruppeAm(z, wx[i] as i32, wy[i] as i32), ort);
        if (z.anzahlRoehren as usize) < MAX_ROEHREN {
            let idx = z.anzahlRoehren as usize;
            z.anzahlRoehren += 1;
            let r = &mut z.roehren[idx];
            r.x1 = wx[i]; r.x2 = wx[i]; r.y1 = wy[i]; r.y2 = wy[i]; r.zustand = 1; r.versuche = 0; r.seit = t;
        }
        neu += 1;
    }
    let nn = (n - 1) as usize;
    if neu == 0 {
        // Weg liegt schon ganz auf Roehren und das Gebaeude hat trotzdem keinen Strom -- die Roehre haengt selbst nicht am Netz.
        if selten(welt, z, 21, 4000) {
            melde(welt, z, "warte", typ, loseR[j].x0, loseR[j].y0,
                  &format!("ohne Strom, obwohl der Weg {},{} -> {},{} schon Roehre ist", wx[0], wy[0], wx[nn], wy[nn]));
        }
        return;
    }
    melde(welt, z, "roehre", typ, loseR[j].x0, loseR[j].y0,
          &format!("nicht angeschlossen (kein Strom bei {} frei) -- {} Roehrenkacheln aufgezeichnet, Weg {},{} -> {},{}",
                   l.stromFrei, neu, wx[0], wy[0], wx[nn], wy[nn]));
}
