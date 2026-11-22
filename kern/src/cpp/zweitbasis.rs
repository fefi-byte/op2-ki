//! Erweiterung der Rust-Fassung (nicht in der C++-KI), zuschaltbar mit PAR.zweitbasis: eine zweite Basis an einem
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
    /// Missionsmodus: Aussenposten per Befehl (Groesse 1-3; 0 = automatische Zweitbasis) und seine Baken
    pub groesse: i32,
    pub baken: [Id; 6],
    pub nb: usize,
    /// letzte Meldung "wartet auf Erz" (Tick)
    pub gemeldet: i32,
    /// Groesse 4 (unabhaengige Basis): eigene BuildingGroup und ihre Einheiten (Strukturfabrik, Fahrzeugfabrik, ConVecs,
    /// Earthworker, Robo-Miner), letzter Earthworker-Auftrag
    pub gruppe: Id,
    pub sf: Id,
    pub vf: Id,
    pub cv: [Id; 2],
    pub ew: Id,
    pub ewSeit: i32,
    pub rm: Id,
    pub lkwSeit: i32,
    /// Platz des gerade gebauten Gebaeudes (oben links, -1 = keiner), Index der Zweitbasis, letzter Bausatz-Auftrag
    pub px: i32,
    pub py: i32,
    pub k: usize,
    pub kitSeit: [i32; 2],
    /// eigene Fahrzeuge der Basis, daheim neu gebaut: hoechste Einheitennummer beim Auftrag (0 = keiner offen)
    pub cvWarte: [i32; 2],
    pub ewWarte: i32,
    /// Tuerme in der Basis (zusaetzlich zu denen an den Zugaengen)
    pub innen: i32,
    /// ihre Spinne (Reparatur; Plymouth, PAR spinnen)
    pub spinne: Id,
    /// Erz, auf das ihr naechster Bau wartet (vorgemerkt: Vorrang vor der Hauptbasis), Tick der Meldung
    pub bedarf: i32,
    pub bedarfSeit: i32,
    /// eigene Wache (FightGroup) aus der eigenen Fahrzeugfabrik, letzter Auftrag
    pub wache: Gruppe,
    pub wacheSeit: i32,
}

/// Umkreis (Kacheln) einer unabhaengigen Basis: Auftraege darin baut ihre eigene BuildingGroup
pub const BASIS_UMKREIS: i32 = 30;

/// Missionsmodus, unabhaengige zweite Basis: die BuildingGroup fuer einen Auftrag bei (x, y) -- die eigene Gruppe der Basis,
/// in deren Umkreis er liegt (sobald sie selbststaendig ist), sonst die der Hauptbasis. Ohne solche Basis immer z.bau.
pub fn gruppeAm(z: &Zustand, x: i32, y: i32) -> Id {
    for k in 0..MAX_ZB {
        let s = zb(z.spieler, k);
        // bis zum Rand ihres Gebiets (gebietAuto: Zugaenge bis 16 Kacheln hinter der Bebauung)
        let r = BASIS_UMKREIS + 16;
        if s.groesse == 4 && s.phase >= 1 && s.gruppe != 0 && (x - s.x).pow(2) + (y - s.y).pow(2) <= r * r { return s.gruppe; }
    }
    z.bau
}

/// Fuer eine unabhaengige Basis bestellt, aber noch nicht zugeordnet (ConVec oder Earthworker mit hoeherer Nummer als beim
/// Auftrag): die Hauptbasis nimmt sie nicht in ihre Gruppe. Einheiten aus der Gruppe der Hauptbasis zu holen liess das
/// Original abstuerzen (0x42A13F im Gruppenbefehl, cm03 zweimal beim Bau-ConVec der Gruppe) -- die Basis baut ihre eigenen.
pub fn reserviert(welt: &mut dyn Welt, z: &Zustand, u: Id) -> bool {
    let ty = welt.GetType(u);
    (0..MAX_ZB).any(|k| { let s = zb(z.spieler, k); s.groesse == 4 && (
        (ty == mapConVec && (0..2).any(|j| s.cv[j] == 0 && s.cvWarte[j] > 0 && u > s.cvWarte[j]))
        || (ty == mapEarthworker && s.ew == 0 && s.ewWarte > 0 && u > s.ewWarte)) })
}

/// Steht die Einheit im Umkreis einer unabhaengigen Basis (ab dem Konvoi)? Ihre Strukturfabrik nimmt die Hauptbasis nie auf,
/// auch nicht kurz, bevor die Basis sie zuordnet.
pub fn amPosten(welt: &mut dyn Welt, z: &Zustand, u: Id) -> bool {
    let lo = welt.Location(u); let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
    (0..MAX_ZB).any(|k| { let s = zb(z.spieler, k); s.groesse == 4 && s.phase >= 2 && (x - s.x).pow(2) + (y - s.y).pow(2) <= BASIS_UMKREIS * BASIS_UMKREIS })
}

/// Gehoert die Einheit einer unabhaengigen Basis? Dann nimmt die Hauptbasis sie nicht in ihre Gruppe und nutzt sie nicht.
pub fn gehoertBasis(z: &Zustand, u: Id) -> bool {
    if u == 0 { return false; }
    (0..MAX_ZB).any(|k| { let s = zb(z.spieler, k); s.groesse == 4 && (s.sf == u || s.vf == u || s.ew == u || s.rm == u || s.spinne == u || s.cv.contains(&u)) })
}

/// Hoechstzahl Zweitbasen je Spieler (PAR zbAnzahl wird darauf begrenzt)
pub const MAX_ZB: usize = 4;

thread_local! {
    pub static ZB: RefCell<[[Zb; MAX_ZB]; 8]> = RefCell::new([[Zb::default(); MAX_ZB]; 8]);
    /// Bakenfelder ohne Platz fuer die Kommandozentrale: (Bake, gesperrt bis Tick) je Spieler
    static SPERRE: RefCell<[Vec<(Id, i32)>; 8]> = RefCell::new(Default::default());
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
    welt.RecordBuilding(gruppeAm(z, px0, py0), ort, typ, mapNone);
    let t = welt.Tick();
    { let p = &mut z.plaetze[iu]; p.zustand = P_FAHRT; p.seit = t; p.convec = 0; }
    melde(welt, z, "bau", typ, px0, py0, &format!("Zweitbasis: {} (Platz {}x{}, Mitte {},{})", warum, g.w, g.h, px0 + g.w / 2, py0 + g.h / 2));
    true
}

/// Aussenposten der Mission: wie aufzeichnenBei, aber ohne den Bebauungsplan der Hauptbasis -- grosse Gebaeude (Schmelze,
/// Fahrzeugfabrik) nahm platzWaehlen sonst aus dem Plan daheim (ml4_05: Schmelze des Postens bei 34,120 statt am Posten)
fn postenBei(welt: &mut dyn Welt, z: &mut Zustand, typ: i32, sx: i32, sy: i32, warum: &str) -> bool {
    super::hilfen::ohnePlan(true);
    let ok = aufzeichnenBei(welt, z, typ, sx, sy, warum);
    super::hilfen::ohnePlan(false);
    ok
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
            welt.RecordBuilding(gruppeAm(z, x, y), ort, mapGuardPost, waffe);
            // Missionsmodus: keine Mauerstuecke an jedem Turm -- die Mauern kommen an die Zugaenge (mauer.rs)
            if !super::modus::mission(z) { mauerNeben(welt, z, x, y); }
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
    let feind = super::erweiterung::feindBasis(welt, z)?;
    let (b, h) = (z.breite, z.hoehe);
    let mut zellen = vec![0i32; (b * h) as usize];
    for y in 0..h { for x in 0..b { let lo = engine(welt, z, x, y); zellen[(y * b + x) as usize] = welt.GetCellType(lo); } }
    let g = Gelaende::neu(&zellen, b, h, (z.bx, z.by), feind, 0.4, 1, 0, 0);
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
    // Missionsmodus (Nutzer 2026-11-22): die zweite Basis kommt per Trigger an einen vorgegebenen Ort; die automatische
    // Zweitbasis ist dort aus
    if let Some(po) = super::modus::postenNehmen(welt, z) { postenBeginnen(welt, z, &po); }
    let auto = p.zweitbasis > 0.0 && !super::modus::mission(z);
    let anzahl = (p.zbAnzahl.max(0.0) as usize).min(MAX_ZB);
    let aktiv = (0..MAX_ZB).filter(|&k| zb(z.spieler, k).phase > 0 || zb(z.spieler, k).groesse > 0).count();
    for k in 0..MAX_ZB {
        let s = zb(z.spieler, k);
        if s.phase > 0 || s.groesse > 0 { platzSchritt(welt, z, l, k); }
    }
    if auto && aktiv < anzahl {
        if let Some(k) = (0..MAX_ZB).find(|&k| zb(z.spieler, k).phase == 0 && zb(z.spieler, k).groesse == 0) { platzSchritt(welt, z, l, k); }
    }
}

/// Aussenposten-Auftrag der Mission beginnen: Kommandozentrale und Schmelze am Ort aufzeichnen, Baken zuordnen (vorgegebene:
/// die Bake hoechstens 3 Kacheln vom Punkt; keine vorgegeben: alle im Umkreis von 8 Kacheln)
fn postenBeginnen(welt: &mut dyn Welt, z: &mut Zustand, po: &super::modus::Posten) {
    let Some(k) = (0..MAX_ZB).find(|&k| zb(z.spieler, k).phase == 0 && zb(z.spieler, k).groesse == 0) else {
        melde(welt, z, "zweitbasis", mapCommandCenter, po.x, po.y, "Aussenposten: alle Plaetze belegt -- Auftrag verworfen"); return;
    };
    let mut baken = [0 as Id; 6]; let mut nb = 0usize;
    let bis = hoechsteEinheit(welt);
    let mut alle: Vec<(Id, i32, i32)> = Vec::new();
    for id in 1..=bis {
        let u: Id = id;
        if !welt.IsLive(u) || welt.GetType(u) != mapMiningBeacon { continue; }
        let lo = welt.Location(u); alle.push((u, edX(welt, z, lo), edY(welt, z, lo)));
    }
    if po.baken.is_empty() {
        for &(u, x, y) in alle.iter() { if nb < 6 && (x - po.x).pow(2) + (y - po.y).pow(2) <= 64 { baken[nb] = u; nb += 1; } }
    } else {
        for &(bx, by) in po.baken.iter() {
            if let Some(&(u, _, _)) = alle.iter().filter(|b| (b.1 - bx).abs() <= 3 && (b.2 - by).abs() <= 3).min_by_key(|b| (b.1 - bx).pow(2) + (b.2 - by).pow(2)) {
                if nb < 6 { baken[nb] = u; nb += 1; }
            } else { melde(welt, z, "zweitbasis", mapMiningBeacon, bx, by, "Aussenposten: keine Bake an diesem Punkt"); }
        }
    }
    // Phase 0 mit Groesse: der Platz ist dem Auftrag reserviert; aufgezeichnet wird, sobald das Erz fuer Kommandozentrale und
    // Schmelze da ist (sonst entsteht der Bausatz nicht, und der Wachhund bricht den Auftrag nach bauAbbruch ab)
    let t = welt.Tick();
    let s = Zb { phase: 0, x: po.x, y: po.y, bake: baken[0], seit: t, naechste: t, groesse: po.groesse, baken, nb, k, px: -1, py: -1, ..Zb::default() };
    setze(z.spieler, k, s);
    melde(welt, z, "zweitbasis", mapCommandCenter, po.x, po.y, &format!("Aussenposten {} der Mission angenommen bei {},{} (Groesse {}, {} Baken)", k + 1, po.x, po.y, po.groesse, nb));
}

/// Aussenposten der Mission aufzeichnen, sobald das Erz reicht. true = aufgezeichnet.
fn postenAufzeichnen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, s: &mut Zb) -> bool {
    let seltenErz = s.nb > 0 && welt.GetOreType(s.baken[0]) == 1 && kitMoeglich(welt, z, mapRareOreSmelter);
    let schmelze = if seltenErz { mapRareOreSmelter } else { mapCommonOreSmelter };
    let kosten = welt.GetOreCost(mapCommandCenter, z.spieler) + if s.nb > 0 { welt.GetOreCost(schmelze, z.spieler) } else { 0 };
    // waehrend eines Angriffs auf die Basis nicht aufzeichnen: die Abwehr darf das vorgemerkte Erz nehmen, der Bausatz
    // entstuende nicht und der Wachhund braeche den Auftrag ab (ml4_05 Seed 1 bei 37 403)
    if angriffLaeuft(welt, z) {
        let t = welt.Tick();
        if t - s.gemeldet >= 6000 { s.gemeldet = t; melde(welt, z, "zweitbasis", mapCommandCenter, s.x, s.y, &format!("Aussenposten der Mission wartet: Angriff auf die Basis (Bedrohung {})", z.bedrohung)); }
        return false;
    }
    if l.erz < kosten {
        let t = welt.Tick();
        if t - s.gemeldet >= 6000 { s.gemeldet = t; melde(welt, z, "zweitbasis", mapCommandCenter, s.x, s.y, &format!("Aussenposten der Mission wartet auf Erz: {} von {}", l.erz, kosten)); }
        return false;
    }
    if !aufzeichnenBei(welt, z, mapCommandCenter, s.x, s.y, "Kommandozentrale (Aussenposten der Mission)") {
        melde(welt, z, "zweitbasis", mapCommandCenter, s.x, s.y, "Aussenposten: kein Platz fuer die Kommandozentrale am Ort"); return false;
    }
    // die Schmelze erst, wenn die Kommandozentrale steht (Phase 2): die BuildingGroup baut nacheinander, und bei weitem Weg
    // brach der Wachhund die wartende Schmelze nach bauAbbruch ab (ml4_05: Kommandozentrale brauchte 7500 Ticks)
    s.phase = 1; s.seit = welt.Tick();
    melde(welt, z, "zweitbasis", mapCommandCenter, s.x, s.y, "Aussenposten der Mission: Kommandozentrale aufgezeichnet (Schmelze folgt, Erz vorgemerkt)");
    true
}

/// Missionsmodus, Vorrang des Aussenpostens (Nutzer 2026-11-22): wartet ein befohlener Posten auf sein Erz, die Kosten
/// (Kommandozentrale + Schmelze) -- sie werden vorgemerkt, und neue Kampffahrzeuge warten ohne akute Bedrohung
/// Auch nach dem Aufzeichnen (Phase 1), bis die Bausaetze bezahlt sind: sonst nahm die Armee das Erz, die Strukturfabrik
/// konnte den Bausatz nicht bauen und der Wachhund brach ab (ml4_05: Erz 247 nach 6000 Ticks).
pub fn postenWartet(welt: &mut dyn Welt, z: &Zustand, l: &Lage) -> Option<i32> {
    let mut summe = 0;
    for k in 0..MAX_ZB {
        let s = zb(z.spieler, k);
        if s.groesse == 4 {
            let cc = welt.GetOreCost(mapCommandCenter, z.spieler); let sf = welt.GetOreCost(mapStructureFactory, z.spieler);
            match s.phase {
                0 => summe += cc + sf,
                1 => summe += sf + if l.kitsImSpiel[mapCommandCenter as usize] == 0 && !nahDa(welt, z, &s, mapCommandCenter) { cc } else { 0 },
                2 => if l.kitsImSpiel[mapStructureFactory as usize] == 0 && !nahDa(welt, z, &s, mapStructureFactory) { summe += sf },
                // selbststaendig (Nutzer 2026-11-22: "die Basis stand lange ohne etwas zu machen -- Erzmangel, das haette ich gern
                // als Vorrang"): was ihr naechster Bau braucht, bleibt der Hauptbasis vorenthalten (frisch gemeldet, 3000 Ticks)
                _ => if s.bedarf > 0 && (welt.Tick() - s.bedarfSeit) < PAR(z.spieler).zbVorrang as i32 { summe += s.bedarf },
            }
            continue;
        }
        if s.groesse <= 0 || s.phase > 1 { continue; }
        let schmelze = if s.nb > 0 && welt.GetOreType(s.baken[0]) == 1 && kitMoeglich(welt, z, mapRareOreSmelter) { mapRareOreSmelter } else { mapCommonOreSmelter };
        if s.phase == 0 {
            summe += welt.GetOreCost(mapCommandCenter, z.spieler) + if s.nb > 0 { welt.GetOreCost(schmelze, z.spieler) } else { 0 };
            continue;
        }
        let nah = |welt: &mut dyn Welt, typ: i32| welt.PlayerBuildingEnum(z.spieler, typ).into_iter().any(|u| {
            let lo = welt.Location(u); (edX(welt, z, lo) - s.x).pow(2) + (edY(welt, z, lo) - s.y).pow(2) <= 144 });
        if l.kitsImSpiel[mapCommandCenter as usize] == 0 && !nah(welt, mapCommandCenter) { summe += welt.GetOreCost(mapCommandCenter, z.spieler); }
        if s.nb > 0 && l.kitsImSpiel[schmelze as usize] == 0 && !nah(welt, schmelze) { summe += welt.GetOreCost(schmelze, z.spieler); }
    }
    if summe > 0 { Some(summe) } else { None }
}

/// Bereit fuer eine neue Zweitbasis? None = ja, sonst der Grund. Die Hauptbasis steht (Fahrzeugfabrik, Armee hat ihr
/// Soll, kein Bauplatz offen), Erz ist uebrig und genug Arbeiter sind frei (Nutzer 2026-11-19).
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
    if s.groesse == 4 { let s2 = basisSchritt(welt, z, l, s); setze(z.spieler, k, s2); return; }
    match s.phase {
        0 if s.groesse > 0 => { postenAufzeichnen(welt, z, l, &mut s); }
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
            let mut imBau = false;
            for u in welt.PlayerBuildingEnum(z.spieler, mapCommandCenter) {
                let lo = welt.Location(u); let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
                if (x - s.x).pow(2) + (y - s.y).pow(2) <= 36 { if welt.GetCurAction(u) != moObjWait { cc = true; } else { imBau = true; } }
            }
            // Aussenposten der Mission: hat der Wachhund den Auftrag abgebrochen (kein aufgezeichneter Platz mehr am Ort), nach
            // 1000 Ticks neu aufzeichnen -- der gesperrte Platz faellt weg, die BuildingGroup waehlt einen anderen
            let aufgezeichnet = (0..z.anzahlPlaetze.max(0) as usize).any(|i| { let q = z.plaetze[i];
                q.typ as i32 == mapCommandCenter && (q.zustand == P_FAHRT || q.zustand == P_BAUT) && (q.x0 as i32 - s.x).pow(2) + (q.y0 as i32 - s.y).pow(2) <= 100 });
            if cc { s.phase = 2; melde(welt, z, "zweitbasis", mapCommandCenter, s.x, s.y, "Kommandozentrale der Zweitbasis steht"); }
            else if s.groesse > 0 && !aufgezeichnet && !imBau && t - s.seit > 1000 {
                // abgebrochen (Wachhund): zurueck in Phase 0, neu aufgezeichnet wird bei genug Erz an einem anderen Platz
                s.phase = 0;
                melde(welt, z, "zweitbasis", mapCommandCenter, s.x, s.y, "Aussenposten der Mission: Bauauftrag abgebrochen -- neu, sobald das Erz reicht");
            }
            else if t - s.seit > 30000 {
                if s.groesse > 0 {
                    // Auftrag der Mission: nicht aufgeben -- die Kommandozentrale noch einmal aufzeichnen
                    s.seit = t; aufzeichnenBei(welt, z, mapCommandCenter, s.x, s.y, "Kommandozentrale (Aussenposten der Mission, neu)");
                    melde(welt, z, "zweitbasis", mapCommandCenter, s.x, s.y, "Aussenposten der Mission: Kommandozentrale nach 30000 Ticks noch nicht da -- neu aufgezeichnet");
                } else { s.phase = 0; melde(welt, z, "zweitbasis", mapCommandCenter, s.x, s.y, "Zweitbasis kam nicht zustande -- neu pruefen"); }
            }
        }
        _ => {
            // Tuerme, Fabriken, Mine -- je Schritt hoechstens ein neuer Auftrag. Aussenposten der Mission nach Groesse:
            // 1 nur Mine und Schmelze, 2 dazu Tuerme, 3 dazu die Fahrzeugfabrik
            let (soll_tuerme, mit_fabrik, mit_struktur) = if s.groesse > 0 {
                (if s.groesse >= 2 { p.zbTuerme.max(3.0) } else { 0.0 }, s.groesse >= 3, false)
            } else { (p.zbTuerme, p.zbFabrik > 0.0, p.zbStruktur > 0.0) };
            // Aussenposten der Mission: zuerst die Schmelze (nach der Kommandozentrale aufgezeichnet)
            if s.groesse > 0 && s.nb > 0 {
                let seltenErz = welt.GetOreType(s.baken[0]) == 1 && kitMoeglich(welt, z, mapRareOreSmelter);
                let typ = if seltenErz { mapRareOreSmelter } else { mapCommonOreSmelter };
                let steht = welt.PlayerBuildingEnum(z.spieler, typ).into_iter().any(|u| {
                    let lo = welt.Location(u); (edX(welt, z, lo) - s.x).pow(2) + (edY(welt, z, lo) - s.y).pow(2) <= 144 });
                let offen = (0..z.anzahlPlaetze.max(0) as usize).any(|i| { let q = z.plaetze[i];
                    q.typ as i32 == typ && (q.zustand == P_FAHRT || q.zustand == P_BAUT) && (q.x0 as i32 - s.x).pow(2) + (q.y0 as i32 - s.y).pow(2) <= 900 });
                if !steht && !offen {
                    if l.erz >= welt.GetOreCost(typ, z.spieler) {
                        postenBei(welt, z, typ, s.x, s.y, if seltenErz { "Seltenerz-Schmelze (Aussenposten der Mission)" } else { "Schmelze (Aussenposten der Mission)" });
                    }
                    setze(z.spieler, k, s); return;
                }
            }
            if (s.tuerme as f64) < soll_tuerme && l.erz >= 700 {
                if turmBei(welt, z, s.x, s.y) { s.tuerme += 1; setze(z.spieler, k, s); return; }
            }
            if mit_fabrik && s.fabrik == 0 && kitMoeglich(welt, z, mapVehicleFactory) && l.erz >= welt.GetOreCost(mapVehicleFactory, z.spieler) {
                if (if s.groesse > 0 { postenBei } else { aufzeichnenBei })(welt, z, mapVehicleFactory, s.x, s.y, "Fahrzeugfabrik") { s.fabrik = 1; setze(z.spieler, k, s); return; }
            }
            if mit_struktur && s.struktur == 0 && l.erz >= welt.GetOreCost(mapStructureFactory, z.spieler) {
                if aufzeichnenBei(welt, z, mapStructureFactory, s.x, s.y, "Strukturfabrik") { s.struktur = 1; setze(z.spieler, k, s); return; }
            }
            // Mine: freier Robo-Miner an die Bake, sonst einen in einer Fahrzeugfabrik bauen lassen. Aussenposten der Mission:
            // die vorgegebenen Baken nacheinander (die erste ohne Mine)
            if s.groesse > 0 && s.nb > 0 {
                if let Some(&b) = s.baken[..s.nb].iter().find(|&&b| { let lo = welt.Location(b); welt.IsLive(b) && !mineAuf(welt, lo) }) { s.bake = b; }
            }
            let bl = welt.Location(s.bake);
            if s.bake != 0 && welt.IsLive(s.bake) && !mineAuf(welt, bl) {
                let typ = if welt.GetOreType(s.bake) == 1 { mapRareOreMine } else { mapCommonOreMine };
                let miner = welt.PlayerVehicleEnum(z.spieler).into_iter().find(|&u| welt.GetType(u) == mapRoboMiner && !welt.IsBusy(u) && welt.GetCurAction(u) == moDone);
                if let Some(m) = miner {
                    if t - s.minerSeit > 3000 {
                        let ecke = super::erz::minenEcke(welt, z, bl); welt.DoBuild(m, typ, ecke);
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

/// Steht (oder entsteht) ein Gebaeude der Sorte im Umkreis der Basis?
fn nahDa(welt: &mut dyn Welt, z: &Zustand, s: &Zb, typ: i32) -> bool {
    welt.PlayerBuildingEnum(z.spieler, typ).into_iter().any(|u| { let lo = welt.Location(u); (edX(welt, z, lo) - s.x).pow(2) + (edY(welt, z, lo) - s.y).pow(2) <= BASIS_UMKREIS * BASIS_UMKREIS })
}
/// Fertiges Gebaeude der Sorte im Umkreis (nicht im Bau): seine Nummer
fn nahFertig(welt: &mut dyn Welt, z: &Zustand, s: &Zb, typ: i32) -> Option<Id> {
    welt.PlayerBuildingEnum(z.spieler, typ).into_iter().find(|&u| { let lo = welt.Location(u);
        (edX(welt, z, lo) - s.x).pow(2) + (edY(welt, z, lo) - s.y).pow(2) <= BASIS_UMKREIS * BASIS_UMKREIS && welt.GetCurAction(u) != moObjWait })
}
/// Aufgezeichneter, noch nicht stehender Platz der Sorte im Umkreis (typ 0: irgendeiner)
fn nahOffen(z: &Zustand, s: &Zb, typ: i32) -> bool {
    (0..z.anzahlPlaetze.max(0) as usize).any(|i| { let q = z.plaetze[i];
        (typ == 0 || q.typ as i32 == typ) && (q.zustand == P_FAHRT || q.zustand == P_BAUT) && (q.x0 as i32 - s.x).pow(2) + (q.y0 as i32 - s.y).pow(2) <= BASIS_UMKREIS * BASIS_UMKREIS })
}

/// Einheit aus der Gruppe der Hauptbasis in die der zweiten Basis holen
fn holen(welt: &mut dyn Welt, z: &Zustand, s: &Zb, u: Id) {
    if z.bauAn != 0 && inGruppe(welt, z.bau, u) { welt.RemoveUnit(z.bau, u); }
    if s.gruppe != 0 { welt.TakeUnit(s.gruppe, u); }
}

/// Earthworker der zweiten Basis: einen freien holen (die Hauptbasis behaelt einen), sonst daheim einen bauen lassen
fn earthworkerPflegen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, s: &mut Zb) {
    let t = welt.Tick();
    if s.ew != 0 && welt.IsLive(s.ew) && welt.GetType(s.ew) == mapEarthworker { return; }
    s.ew = 0;
    // der eigens gebaute (Nummer hoeher als beim Auftrag, in keiner Gruppe der Hauptbasis)
    if s.ewWarte > 0 {
        let neu = welt.PlayerVehicleEnum(z.spieler).into_iter().find(|&u| welt.GetType(u) == mapEarthworker && u > s.ewWarte && !gehoertBasis(z, u)
            && !(z.bauAn != 0 && inGruppe(welt, z.bau, u)));
        if let Some(u) = neu {
            s.ew = u; s.ewWarte = 0;
            if s.phase >= 5 { holen(welt, z, s, u); }
            melde(welt, z, "zweitbasis", mapEarthworker, s.x, s.y, &format!("Earthworker {} gehoert der zweiten Basis", u));
            return;
        }
    }
    if (s.ewWarte == 0 || t - s.ewSeit > 8000) && l.erz >= welt.GetOreCost(mapEarthworker, z.spieler) {
        let vf = l.vfs.iter().copied().take(l.vfAnzahl.max(0) as usize).find(|&v| v != 0 && !welt.IsBusy(v) && welt.HasPower(v) && welt.HasWorkers(v));
        if let Some(v) = vf {
            s.ewWarte = hoechsteEinheit(welt);
            welt.DoProduce(v, mapEarthworker, mapNone); s.ewSeit = t;
            melde(welt, z, "zweitbasis", mapEarthworker, s.x, s.y, "Earthworker fuer die zweite Basis in Auftrag");
        }
    }
}

/// Missionsmodus, unabhaengige zweite Basis (Groesse 4, Nutzer 2026-11-22): Kommandozentrale und Strukturfabrik kommen als
/// Bausaetze von daheim, dazu ein Earthworker. Danach baut die Basis mit eigener BuildingGroup (ihre Strukturfabrik, zwei
/// ConVecs, der Earthworker) selbst: Kraftwerk, Schmelze, Fahrzeugfabrik, Mine mit Lastwagen, Tuerme; Roehren und
/// Zugangsmauern in ihrem Umkreis gehen ueber gruppeAm an sie. Phasen: 0 warten, 1 Kommandozentrale unterwegs,
/// 2 Strukturfabrik unterwegs, 3 selbststaendig.
fn basisSchritt(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, mut s: Zb) -> Zb {
    let t = welt.Tick();
    let p = z.spieler;
    let warte = |welt: &mut dyn Welt, z: &mut Zustand, s: &mut Zb, txt: String| {
        if t - s.gemeldet >= 6000 { s.gemeldet = t; melde(welt, z, "zweitbasis", mapCommandCenter, s.x, s.y, &txt); }
    };
    if s.phase >= 1 { earthworkerPflegen(welt, z, l, &mut s); }
    match s.phase {
        0 => {
            if angriffLaeuft(welt, z) { warte(welt, z, &mut s, format!("Zweite Basis wartet: Angriff auf die Basis (Bedrohung {})", z.bedrohung)); return s; }
            let kosten = welt.GetOreCost(mapCommandCenter, p) + welt.GetOreCost(mapStructureFactory, p);
            if l.erz < kosten { warte(welt, z, &mut s, format!("Zweite Basis wartet auf Erz: {} von {}", l.erz, kosten)); return s; }
            if s.gruppe == 0 {
                // die eigene Gruppe gleich anlegen (Auftraege im Umkreis gehen ab jetzt an sie); ihre Einheiten kommen erst am
                // Ort hinein -- die Wegsuche des Spiels findet ueber so weite Strecken keinen Weg, der Konvoi faehrt in Etappen
                let g = welt.CreateBuildingGroup(p);
                let (mut rx, mut ry) = (s.x, s.y + 6);
                freierWarteplatz(welt, z, &mut rx, &mut ry, 2, false);
                let (a, b) = (engine(welt, z, rx - 3, ry - 1), engine(welt, z, rx + 3, ry + 2));
                welt.SetRect(g, Rect::new(a, b));
                welt.SetTargCount(g, mapEarthworker, mapNone, 1);
                s.gruppe = g;
            }
            s.phase = 1; s.seit = t;
            melde(welt, z, "zweitbasis", mapCommandCenter, s.x, s.y, "Zweite Basis: Bausaetze fuer Kommandozentrale und Strukturfabrik in Auftrag (Erz vorgemerkt)");
        }
        1 => {
            // beide Bausaetze laden: Kommandozentrale in ConVec 1, Strukturfabrik in ConVec 2
            // nacheinander (eine Strukturfabrik, ein Dock): erst die Kommandozentrale, dann die Strukturfabrik; beim Laden kurz
            // takten, damit der fertige Bausatz gleich umgeladen wird
            s.naechste = t + 40;
            let a = kitLaden(welt, z, l, &mut s, 0, mapCommandCenter);
            let b = a && kitLaden(welt, z, l, &mut s, 1, mapStructureFactory);
            if a && b {
                s.phase = 2; s.seit = t;
                melde(welt, z, "zweitbasis", mapConVec, s.x, s.y, &format!("Zweite Basis: Konvoi faehrt los (ConVecs {} und {}, Earthworker {})", s.cv[0], s.cv[1], s.ew));
            }
        }
        2 => {
            let mut fz: Vec<Id> = vec![s.cv[0], s.cv[1]];
            if s.ew != 0 { fz.push(s.ew); }
            if konvoi(welt, z, s.k, 0, &fz, (s.x, s.y + 3)) {
                melde(welt, z, "zweitbasis", mapConVec, s.x, s.y, &format!("Zweite Basis: Konvoi angekommen nach {} Ticks", t - s.seit));
                s.phase = 3; s.seit = t;
            }
        }
        3 | 4 => {
            // am Ort bauen: erst die Kommandozentrale, dann die Strukturfabrik (sie braucht den Anschluss)
            let (typ, c) = if s.phase == 3 { (mapCommandCenter, s.cv[0]) } else { (mapStructureFactory, s.cv[1]) };
            if let Some(u) = nahFertig(welt, z, &s, typ) {
                melde(welt, z, "zweitbasis", typ, s.x, s.y, &format!("Zweite Basis: {} steht", bauName(typ)));
                if typ == mapStructureFactory { s.sf = u; }
                s.phase += 1; s.seit = t; s.px = -1;
                if s.phase == 5 {
                    // selbststaendig: Strukturfabrik, ConVecs und Earthworker in die eigene Gruppe
                    let units = [s.sf, s.cv[0], s.cv[1], s.ew];
                    for &u in units.iter() { if u != 0 && welt.IsLive(u) { holen(welt, z, &s, u); } }
                    melde(welt, z, "zweitbasis", mapStructureFactory, s.x, s.y, &format!("Zweite Basis selbststaendig: eigene BuildingGroup mit Strukturfabrik {}, ConVecs {} und {}, Earthworker {}", s.sf, s.cv[0], s.cv[1], s.ew));
                }
                return s;
            }
            if nahDa(welt, z, &s, typ) { return s; }   // im Bau
            if c == 0 || !welt.IsLive(c) || welt.GetCargo(c) != typ {
                // Bausatz verloren (ConVec zerstoert): neu
                s.phase = 1; s.seit = t;
                melde(welt, z, "zweitbasis", typ, s.x, s.y, &format!("Zweite Basis: Bausatz {} verloren -- neu", bauName(typ)));
                return s;
            }
            if s.px < 0 || t - s.seit > 3000 {
                // Platz am Ort (ohne den Bebauungsplan der Hauptbasis), dann der Bau an der Ecke rechts unten
                if s.px >= 0 { // erster Versuch ohne Erfolg: Platz freigeben, anderer Platz
                    if let Some(q) = (0..z.anzahlPlaetze.max(0) as usize).find(|&i| z.plaetze[i].x0 as i32 == s.px && z.plaetze[i].y0 as i32 == s.py && z.plaetze[i].typ as i32 == typ) { z.plaetze[q].zustand = P_GESPERRT; }
                }
                let (bx, by) = (z.bx, z.by);
                super::hilfen::ohnePlan(true);
                z.bx = s.x; z.by = s.y;
                let i = platzWaehlen(welt, z, typ);
                z.bx = bx; z.by = by;
                super::hilfen::ohnePlan(false);
                if i < 0 { warte(welt, z, &mut s, format!("Zweite Basis: kein Platz fuer {}", bauName(typ))); return s; }
                let iu = i as usize;
                z.plaetze[iu].zustand = P_FAHRT; z.plaetze[iu].seit = t;
                s.px = z.plaetze[iu].x0 as i32; s.py = z.plaetze[iu].y0 as i32; s.seit = t;
                let g = grundflaeche(welt, typ);
                let ecke = engine(welt, z, s.px + g.w, s.py + g.h);
                welt.DoBuild(c, typ, ecke);
                melde(welt, z, "bau", typ, s.px, s.py, &format!("Zweite Basis: ConVec {} baut {} (Platz {}x{} bei {},{})", c, bauName(typ), g.w, g.h, s.px, s.py));
            }
        }
        _ => {
            // Einheiten nachfuehren
            if s.sf != 0 && !welt.IsLive(s.sf) { s.sf = 0; }
            if s.sf == 0 { if let Some(sf) = nahFertig(welt, z, &s, mapStructureFactory) { s.sf = sf; holen(welt, z, &s, sf); } }
            if s.vf != 0 && !welt.IsLive(s.vf) { s.vf = 0; }
            if s.vf == 0 { if let Some(vf) = nahFertig(welt, z, &s, mapVehicleFactory) { s.vf = vf; } }
            // die Hauptbasis hatte sie stillgelegt, bevor sie hierher gehoerte (Mission cm03: kein Robo-Miner, keine Mine)
            if s.vf != 0 && t % 2000 < 400 && welt.GetCurAction(s.vf) == moDone && !welt.IsBusy(s.vf) && !welt.HasWorkers(s.vf) { welt.DoUnIdle(s.vf); }
            for i in 0..2 { if s.cv[i] != 0 && !welt.IsLive(s.cv[i]) { s.cv[i] = 0; } }
            // Zustand alle 6000 Ticks in die Spur
            if t % 6000 < 400 {
                let b = s.baken[..s.nb].first().copied().unwrap_or(0);
                let bl = welt.Location(b); let (bx, by) = (edX(welt, z, bl), edY(welt, z, bl));
                let (m, tk) = (mineAuf(welt, bl), nahDa(welt, z, &s, mapTokamak));
                let ot = welt.GetOreType(b);
                melde(welt, z, "zweitbasis", mapNone, s.x, s.y, &format!("Zweite Basis Stand: Bake {} bei {},{} Sorte {} Mine {} | Tokamak {} | Fabrik {} Miner {} | offen {}", b, bx, by, ot, m, tk, s.vf, s.rm, nahOffen(z, &s, 0)));
            }
            // je Schritt hoechstens ein Auftrag, und erst wenn keiner offen ist (die Gruppe baut nacheinander)
            if nahOffen(z, &s, 0) { return s; }
            let bauen = |welt: &mut dyn Welt, z: &mut Zustand, s: &mut Zb, typ: i32, txt: &str| -> bool {
                if !kitMoeglich(welt, z, typ) { return false; }
                let k = welt.GetOreCost(typ, z.spieler);
                if l.erz < k { s.bedarf = k; s.bedarfSeit = welt.Tick(); warte(welt, z, s, format!("Zweite Basis wartet auf Erz fuer {}: {} von {}", txt, l.erz, k)); return true; }
                postenBei(welt, z, typ, s.x, s.y, &format!("{} (zweite Basis der Mission)", txt)); true
            };
            // Kraftwerk
            if !nahDa(welt, z, &s, mapTokamak) && !nahDa(welt, z, &s, mapSolarPowerArray) && !nahDa(welt, z, &s, mapGeothermalPlant) {
                if bauen(welt, z, &mut s, mapTokamak, "Tokamak") { return s; }
            }
            // Schmelze zur Bake (Seltenerz-Schmelze bei Seltenerz, sobald erforscht)
            let bake = s.baken[..s.nb].iter().copied().find(|&b| welt.IsLive(b));
            if let Some(b) = bake {
                let selten = welt.GetOreType(b) == 1;
                let typ = if selten { mapRareOreSmelter } else { mapCommonOreSmelter };
                if !nahDa(welt, z, &s, typ) {
                    if !kitMoeglich(welt, z, typ) { warte(welt, z, &mut s, "Zweite Basis wartet auf die Forschung fuer die Seltenerz-Schmelze".into()); }
                    else if bauen(welt, z, &mut s, typ, if selten { "Seltenerz-Schmelze" } else { "Schmelze" }) { return s; }
                }
            }
            // Fahrzeugfabrik (Robo-Miner und Lastwagen baut die Basis selbst)
            if !nahDa(welt, z, &s, mapVehicleFactory) { if bauen(welt, z, &mut s, mapVehicleFactory, "Fahrzeugfabrik") { return s; } }
            // Mine: Robo-Miner aus der eigenen Fabrik an die Bake
            if let Some(b) = bake {
                let bl = welt.Location(b);
                let st = if welt.GetOreType(b) == 1 { mapRareOreSmelter } else { mapCommonOreSmelter };
                let schmelzeDa = nahFertig(welt, z, &s, st).is_some();
                if schmelzeDa && !mineAuf(welt, bl) {
                    let typ = if welt.GetOreType(b) == 1 { mapRareOreMine } else { mapCommonOreMine };
                    if s.rm != 0 && (!welt.IsLive(s.rm) || welt.GetType(s.rm) != mapRoboMiner) { s.rm = 0; }
                    if s.rm == 0 {
                        // ein neuer Robo-Miner an der eigenen Fabrik
                        s.rm = welt.PlayerVehicleEnum(p).into_iter().find(|&u| welt.GetType(u) == mapRoboMiner && !welt.IsBusy(u) && welt.GetCurAction(u) == moDone && !gehoertBasis(z, u) && {
                            let lo = welt.Location(u); (edX(welt, z, lo) - s.x).pow(2) + (edY(welt, z, lo) - s.y).pow(2) <= BASIS_UMKREIS * BASIS_UMKREIS }).unwrap_or(0);
                    }
                    if s.rm != 0 {
                        if t - s.minerSeit > 3000 && !welt.IsBusy(s.rm) {
                            let ecke = super::erz::minenEcke(welt, z, bl); welt.DoBuild(s.rm, typ, ecke); s.minerSeit = t;
                            let (mx, my) = (edX(welt, z, bl), edY(welt, z, bl));
                            melde(welt, z, "zweitbasis", typ, mx, my, &format!("Robo-Miner {} baut die Mine der zweiten Basis", s.rm));
                        }
                    } else if s.vf != 0 && !welt.IsBusy(s.vf) && t - s.minerSeit > 3000 && l.erz >= welt.GetOreCost(mapRoboMiner, p) {
                        welt.DoProduce(s.vf, mapRoboMiner, mapNone); s.minerSeit = t;
                        melde(welt, z, "zweitbasis", mapRoboMiner, s.x, s.y, "Robo-Miner in der Fabrik der zweiten Basis");
                    }
                    return s;
                }
                // Lastwagen: zwei aus der eigenen Fabrik, sobald die Mine steht
                let eigene = [mapCommonOreMine, mapRareOreMine].iter().any(|&mt| welt.PlayerBuildingEnum(p, mt).into_iter().any(|m| { let ml = welt.Location(m); ml.x == bl.x && ml.y == bl.y }));
                if mineAuf(welt, bl) && !eigene { warte(welt, z, &mut s, "Zweite Basis: auf ihrer Bake steht eine fremde Mine".into()); }
                if eigene && s.vf != 0 && !welt.IsBusy(s.vf) && t - s.lkwSeit > 2000 {
                    let n = welt.PlayerVehicleEnum(p).into_iter().filter(|&u| welt.GetType(u) == mapCargoTruck && {
                        let lo = welt.Location(u); (edX(welt, z, lo) - s.x).pow(2) + (edY(welt, z, lo) - s.y).pow(2) <= BASIS_UMKREIS * BASIS_UMKREIS }).count();
                    if n < 2 && l.erz >= welt.GetOreCost(mapCargoTruck, p) {
                        welt.DoProduce(s.vf, mapCargoTruck, mapNone); s.lkwSeit = t;
                        melde(welt, z, "zweitbasis", mapCargoTruck, s.x, s.y, &format!("Lastwagen {} in der Fabrik der zweiten Basis", n + 1));
                        return s;
                    }
                }
            }
            // Wache (Nutzer 2026-11-22: "die 2. Basis hatte keinerlei Fahrzeuge zur Verteidigung"): eigene Kampffahrzeuge aus der
            // eigenen Fabrik, bestes Fahrgestell und Waffe wie die Hauptbasis (aufstieg), vier -- sechs unter Bedrohung
            if PAR(p).zbWache > 0.0 && s.vf != 0 && welt.IsLive(s.vf) && nahFertig(welt, z, &s, mapVehicleFactory).is_some() {
                if s.wache == 0 {
                    s.wache = welt.CreateFightGroup(p);
                    let r = 6;
                    let (a, b) = (engine(welt, z, s.x - r, s.y - r), engine(welt, z, s.x + r, s.y + r));
                    welt.SetRect(s.wache, Rect::new(a, b));
                    welt.DoGuardRect(s.wache);
                }
                let mut drin: Vec<Id> = welt.GroupEnumerator(s.wache).into_iter().filter(|&u| welt.IsLive(u)).collect();
                drin.dedup();
                // neue Kampffahrzeuge an der Basis ohne Gruppe aufnehmen
                for u in welt.PlayerVehicleEnum(p) {
                    if drin.contains(&u) || !welt.IsLive(u) || !kaempfer(welt, u) || welt.GetCurAction(u) != moDone { continue; }
                    let lo = welt.Location(u);
                    if (edX(welt, z, lo) - s.x).pow(2) + (edY(welt, z, lo) - s.y).pow(2) > 144 { continue; }
                    if [z.wache, z.sammel, z.angriff, z.angriff2, z.eingreif].iter().any(|&g| g != 0 && inGruppeG(welt, g, u)) { continue; }
                    welt.TakeUnit(s.wache, u); drin.push(u);
                    let ut = welt.GetType(u);
                    melde(welt, z, "zweitbasis", ut, s.x, s.y, &format!("Wache der zweiten Basis: Fahrzeug {} ({} von ihr)", u, drin.len()));
                }
                let soll = (if z.bedrohung >= 50 { PAR(p).zbWacheDruck } else { PAR(p).zbWache }).max(0.0) as usize;
                if drin.len() < soll && !welt.IsBusy(s.vf) && welt.GetCurAction(s.vf) == moDone && t - s.wacheSeit > 1500 {
                    let fz = { let f = fahrgestellWahl(welt, z, l.selten); if f == 0 { mapLynx } else { f } };
                    let fs = welt.GetRareOreCost(fz, p);
                    let w = waffeWahl(welt, z, fz, l.selten - fs);
                    let k = welt.GetOreCost(fz, p) + welt.GetOreCost(w, p);
                    if einheitErforscht(welt, z, fz) && l.erz >= k {
                        welt.DoProduce(s.vf, fz, w); s.wacheSeit = t;
                        let (nf, nw) = (name(welt, fz), name(welt, w));
                        melde(welt, z, "zweitbasis", fz, s.x, s.y, &format!("Wache der zweiten Basis: {} mit {} in Auftrag ({} von {})", nf, nw, drin.len() + 1, soll));
                        return s;
                    } else if l.erz < k { s.bedarf = k; s.bedarfSeit = t; }
                }
            }
            // Tuerme: an den Zugaengen ihres Gebiets (gebietAuto, mauer.rs; die Mauern folgen, sobald ein Turm steht), sonst
            // drei zum Feind hin
            if l.erz >= 700 && nahFertig(welt, z, &s, mapVehicleFactory).is_some() {
                if PAR(z.spieler).gebietAuto >= 0.5 {
                    if let Some((x, y)) = super::mauer::zugangTurmBei(welt, z, s.x, s.y, BASIS_UMKREIS + 16) {
                        let i = platzEintrag(welt, z, mapGuardPost, x, y);
                        if i >= 0 {
                            let ort = engine(welt, z, x, y);
                            let waffe = eigeneWaffe(welt, z);
                            welt.RecordBuilding(s.gruppe, ort, mapGuardPost, waffe);
                            { let q = &mut z.plaetze[i as usize]; q.zustand = P_FAHRT; q.seit = t; q.convec = 0; }
                            s.tuerme += 1;
                            melde(welt, z, "bau", mapGuardPost, x, y, "Zweite Basis: Wachturm am Zugang ihres Gebiets");
                        }
                    }
                } else if s.tuerme < 3 && turmBei(welt, z, s.x, s.y) { s.tuerme += 1; }
            }
            if nahOffen(z, &s, 0) { return s; }
            // eigenes Leben (Nutzer 2026-11-22: "Die Zweitbasis darf mehr Leben haben -- Wohnungen, bessere Tuerme, Nahrung.
            // Eine eigenstaendige Basis."): Nahrung, Wohnen, Medizin, dann Kindergarten, Freizeit, DIRT, sobald erforscht; je
            // ein Auftrag, der Reihe nach. Dazu zwei Tuerme in der Basis zum Feind hin (die beste Waffe, eigeneWaffe).
            // Spinnenfabrik (Nutzer 2026-11-22: "Bau vorher die Fabrik in der zweiten Basis ein"): die Basis baut ihre Spinne selbst
            let leben: [(i32, usize, &str); 10] = [(mapAgridome, 1, "Agrarkuppel"), (mapResidence, 1, "Wohnhaus"), (mapMedicalCenter, 1, "Medizinzentrum"),
                (mapArachnidFactory, 1, "Spinnenfabrik"),
                (mapResidence, 2, "Wohnhaus"), (mapNursery, 1, "Kindergarten"), (mapRecreationFacility, 1, "Freizeitzentrum"), (mapDIRT, 1, "DIRT"), (mapAgridome, 2, "Agrarkuppel"), (mapForum, 1, "Forum")];
            for (typ0, soll, txt) in leben {
                if typ0 == mapArachnidFactory && (PAR(p).spinnen < 0.5 || welt.IsEden(p)) { continue; }
                // Wohnen: das beste erforschte Wohnmodul; gezaehlt werden alle Wohnarten
                let typ = if typ0 == mapResidence { wohnTyp(welt, z, l) } else { typ0 };
                let arten: Vec<i32> = if typ0 == mapResidence { vec![mapResidence, mapReinforcedResidence, mapAdvancedResidence] } else { vec![typ0] };
                let n: usize = arten.iter().map(|&a| welt.PlayerBuildingEnum(p, a).into_iter().filter(|&u| { let lo = welt.Location(u); (edX(welt, z, lo) - s.x).pow(2) + (edY(welt, z, lo) - s.y).pow(2) <= BASIS_UMKREIS * BASIS_UMKREIS }).count()).sum();
                if n >= soll || !kitMoeglich(welt, z, typ) { continue; }
                if l.erz < welt.GetOreCost(typ, p) + 500 { s.bedarf = welt.GetOreCost(typ, p) + 500; s.bedarfSeit = t; return s; }
                postenBei(welt, z, typ, s.x, s.y, &format!("{} (zweite Basis der Mission)", txt));
                return s;
            }
            if s.innen < 2 && l.erz >= 1000 && turmBei(welt, z, s.x, s.y) { s.innen += 1; }
        }
    }
    s
}

/// Laeuft gerade ein Angriff auf die Basis (Feind in den letzten 6000 Ticks im Umkreis)? Die Bedrohung allein reicht nicht:
/// eine groessere Feindarmee irgendwo hebt sie dauerhaft auf 65 (cm03: der Posten wartete 100 000 Ticks).
fn angriffLaeuft(welt: &mut dyn Welt, z: &Zustand) -> bool { z.letzterAngriff != 0 && welt.Tick() - z.letzterAngriff < 6000 }

/// Konvoi der zweiten Basis: Etappen (Kacheln) je Weg, naechster Punkt, seit wann
#[derive(Clone, Default)]
struct Konvoi { punkte: Vec<(i32, i32)>, i: usize, seit: i32, ziel: (i32, i32), zuletzt: Vec<(Id, i32, i32)> }
thread_local! { static KONVOI: RefCell<[[[Option<Konvoi>; 2]; MAX_ZB]; 8]> = RefCell::new(Default::default()); }

/// Fahrzeuge in Etappen von 10 Kacheln zum Ziel (die Wegsuche des Spiels scheitert auf weite Strecken mit Felsen dazwischen;
/// cm03: der ConVec mit dem Bausatz stand 35 000 Ticks still). true = das erste ist angekommen.
fn konvoi(welt: &mut dyn Welt, z: &mut Zustand, k: usize, w: usize, fz: &[Id], ziel: (i32, i32)) -> bool {
    let t = welt.Tick();
    let sp = (z.spieler & 7) as usize;
    let lebend: Vec<Id> = fz.iter().copied().filter(|&u| u != 0 && welt.IsLive(u)).collect();
    let Some(&erster) = lebend.first() else { return false; };
    let lo = welt.Location(erster);
    let pos = (edX(welt, z, lo), edY(welt, z, lo));
    if (pos.0 - ziel.0).pow(2) + (pos.1 - ziel.1).pow(2) <= 9 { KONVOI.with(|m| m.borrow_mut()[sp][k][w] = None); return true; }
    let alt = KONVOI.with(|m| m.borrow()[sp][k][w].clone());
    let mut m = match alt {
        Some(m) if m.ziel == ziel => m,
        _ => {
            let rs = super::routen::routen(welt, z, pos, ziel);
            let mut punkte: Vec<(i32, i32)> = Vec::new();
            if let Some(r) = rs.first() { let mut j = 10; while j + 5 < r.weg.len() { punkte.push(r.weg[j]); j += 10; } }
            punkte.push(ziel);
            melde(welt, z, "zweitbasis", mapConVec, ziel.0, ziel.1, &format!("Konvoi zur zweiten Basis: {} Etappen von {},{}", punkte.len(), pos.0, pos.1));
            let m = Konvoi { punkte, i: 0, seit: t - 10000, ziel, zuletzt: Vec::new() };
            m
        }
    };
    let p = m.punkte[m.i.min(m.punkte.len() - 1)];
    let nah = |welt: &mut dyn Welt, z: &Zustand, u: Id, q: (i32, i32), r: i32| { let lo = welt.Location(u); (edX(welt, z, lo) - q.0).pow(2) + (edY(welt, z, lo) - q.1).pow(2) <= r * r };
    // weiter, wenn alle am Etappenpunkt sind (oder die Frist um ist); sonst stehengebliebene neu schicken
    let alle = lebend.iter().all(|&u| nah(welt, z, u, p, 3));
    if (alle || t - m.seit > 1500) && m.i + 1 < m.punkte.len() && t - m.seit > 200 {
        m.i += 1; m.seit = t;
        let q = m.punkte[m.i];
        for &u in &lebend { let l = engine(welt, z, q.0, q.1); welt.DoMove(u, l); }
    } else {
        let q = m.punkte[m.i];
        for &u in &lebend {
            // ruhig oder seit dem letzten Schritt nicht bewegt (ein Andock- oder Warteauftrag haengt): neu schicken
            let lo = welt.Location(u); let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
            let steht = m.zuletzt.iter().any(|e| e.0 == u && e.1 == x && e.2 == y);
            if (welt.GetCurAction(u) == moDone || steht) && !nah(welt, z, u, q, 3) { let l = engine(welt, z, q.0, q.1); welt.DoMove(u, l); }
        }
        if t - m.seit > 10000 { m.seit = t; }
    }
    m.zuletzt = lebend.iter().map(|&u| { let lo = welt.Location(u); (u, edX(welt, z, lo), edY(welt, z, lo)) }).collect();
    KONVOI.with(|x| x.borrow_mut()[sp][k][w] = Some(m));
    false
}

/// Bausatz fuer die zweite Basis in ConVec j laden: daheim einen ConVec holen, eine Strukturfabrik den Bausatz bauen lassen,
/// andocken und umladen. true = ConVec j traegt ihn.
fn kitLaden(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, s: &mut Zb, j: usize, typ: i32) -> bool {
    let t = welt.Tick();
    let p = z.spieler;
    let mut c = s.cv[j];
    if c != 0 && (!welt.IsLive(c) || welt.GetType(c) != mapConVec) { c = 0; s.cv[j] = 0; }
    if c != 0 && welt.GetCargo(c) == typ { return true; }
    if c == 0 {
        // ein eigens gebauter ConVec (Nummer hoeher als beim Auftrag, nie in der Gruppe der Hauptbasis): ConVecs aus der
        // Gruppe zu holen liess das Original abstuerzen (0x42A13F)
        let andere = s.cv;
        let frei = if s.cvWarte[j] > 0 { welt.PlayerVehicleEnum(p).into_iter().find(|&u| welt.GetType(u) == mapConVec && u > s.cvWarte[j] && !gehoertBasis(z, u)
            && !andere.contains(&u) && !(z.bauAn != 0 && inGruppe(welt, z.bau, u))) } else { None };
        let Some(u) = frei else {
            // bestellen (neu, wenn der Auftrag nach 8000 Ticks nichts gebracht hat)
            if (s.cvWarte[j] == 0 || t - s.kitSeit[j] > 8000) && l.erz >= welt.GetOreCost(mapConVec, p) {
                let vf = l.vfs.iter().copied().take(l.vfAnzahl.max(0) as usize).find(|&v| v != 0 && !welt.IsBusy(v) && welt.HasPower(v) && welt.HasWorkers(v));
                if let Some(v) = vf { s.cvWarte[j] = hoechsteEinheit(welt); welt.DoProduce(v, mapConVec, mapNone); s.kitSeit[j] = t;
                    melde(welt, z, "zweitbasis", mapConVec, s.x, s.y, "ConVec fuer die zweite Basis in Auftrag"); }
            }
            return false;
        };
        s.cvWarte[j] = 0;
        s.cv[j] = u; c = u;
        melde(welt, z, "zweitbasis", mapConVec, s.x, s.y, &format!("ConVec {} holt den Bausatz {} fuer die zweite Basis", u, bauName(typ)));
    }
    // Bausatz in einer Bucht daheim?
    let sfs: Vec<Id> = welt.PlayerBuildingEnum(p, mapStructureFactory).into_iter().filter(|&u| !gehoertBasis(z, u) && welt.GetCurAction(u) != moObjWait).collect();
    let mut bucht: Option<(Id, i32)> = None;
    for &f in &sfs { for b in 0..6 { if welt.GetFactoryCargo(f, b) == typ { bucht = Some((f, b)); } } }
    if std::env::var("OP2KI_ZB_DIAG").is_ok() && t % 2000 < 40 {
        let cl = welt.Location(c);
        let d: Vec<String> = sfs.iter().map(|&f| { let dk = welt.GetDockLocation(f); let b: Vec<i32> = (0..6).map(|b| welt.GetFactoryCargo(f, b)).collect(); format!("SF {} dock {},{} busy {} bays {:?}", f, dk.x, dk.y, welt.IsBusy(f), b) }).collect();
        let (ak, la) = (welt.GetCurAction(c), welt.GetCargo(c));
        melde(welt, z, "diagnose", typ, s.x, s.y, &format!("kitLaden {}: ConVec {} bei {},{} Aktion {} Ladung {} | {}", bauName(typ), c, cl.x, cl.y, ak, la, d.join(" | ")));
    }
    if let Some((f, b)) = bucht {
        let dock = welt.GetDockLocation(f);
        let cl = welt.Location(c);
        // einmal umladen und die Andock-Animation abwarten (jeder weitere Befehl startet sie neu -- im 40-Tick-Takt kam
        // der Bausatz nie an)
        if cl.x == dock.x && cl.y == dock.y { if t - s.kitSeit[j] > 500 { welt.DoTransferCargo(f, b); s.kitSeit[j] = t; } }
        else if welt.GetCurAction(c) == moDone || t - s.kitSeit[j] > 3000 { welt.DoDock(c, dock); s.kitSeit[j] = t; }
        return false;
    }
    // sonst bauen lassen -- erst, wenn der eigene ConVec am Dock wartet: sonst nahm ein ConVec der Hauptbasis den fertigen
    // Bausatz aus der Bucht, und daheim entstand eine weitere Strukturfabrik (cm03: der Bausatz wurde viermal gebaut)
    let Some(&f) = sfs.iter().find(|&&f| welt.HasPower(f) && welt.HasWorkers(f)) else { return false; };
    let dock = welt.GetDockLocation(f);
    let cl = welt.Location(c);
    let amDock = (cl.x - dock.x).abs() <= 1 && (cl.y - dock.y).abs() <= 1;
    if !amDock { if welt.GetCurAction(c) == moDone || t - s.kitSeit[j] > 3000 { welt.DoDock(c, dock); s.kitSeit[j] = t; } return false; }
    if !welt.IsBusy(f) && t - s.kitSeit[j] > 1000 && l.erz >= welt.GetOreCost(typ, p) {
        welt.DoProduce(f, typ, mapNone); s.kitSeit[j] = t;
        melde(welt, z, "zweitbasis", typ, s.x, s.y, &format!("Strukturfabrik {} baut den Bausatz {} fuer die zweite Basis (ConVec {} wartet am Dock)", f, bauName(typ), c));
    }
    false
}

/// Name der Bausaetze der zweiten Basis (fuer Meldungen)
fn bauName(typ: i32) -> &'static str {
    if typ == mapCommandCenter { "Kommandozentrale" } else if typ == mapStructureFactory { "Strukturfabrik" } else { "Gebaeude" }
}

/// Missionsmodus, Plymouth (PAR spinnen): Spinnen aus der Spinnenfabrik der Hauptbasis -- zwei fuer die Hauptbasis, je eine fuer
/// jede selbststaendige zweite Basis; ihre Spinne faehrt in Etappen hinueber (Konvoi-Weg 1) und gehoert dann ihr.
/// Wartet eine zweite Basis auf ConVecs aus der Fahrzeugfabrik (Phase 1)? Dann darf die Fabrik nicht stillgelegt werden
/// (Mission cm03 mit Rolle defensiv: die Fabrik lag still, der Bausatz-Auftrag wartete 300 000 Ticks).
pub fn wartetAufFahrzeug(z: &Zustand) -> bool {
    (0..MAX_ZB).any(|k| { let s = zb(z.spieler, k); s.groesse == 4 && s.phase == 1 && (s.cv[0] == 0 || s.cv[1] == 0) })
}

pub fn spinnenSchritt(welt: &mut dyn Welt, z: &mut Zustand) {
    if PAR(z.spieler).spinnen < 0.5 || !super::modus::mission(z) || welt.IsEden(z.spieler) { return; }
    let t = welt.Tick();
    if t % 400 >= TAKT { return; }
    let p = z.spieler;
    let spinnen: Vec<Id> = welt.PlayerVehicleEnum(p).into_iter().filter(|&u| welt.GetType(u) == mapSpider && welt.IsLive(u)).collect();
    let fabriken: Vec<Id> = welt.PlayerBuildingEnum(p, mapArachnidFactory).into_iter()
        .filter(|&f| welt.GetCurAction(f) == moDone && !welt.IsBusy(f) && welt.HasPower(f) && welt.HasWorkers(f)).collect();
    let mut eigeneFabrik: Vec<Id> = Vec::new();   // Spinnenfabriken der zweiten Basen
    let mut ohneFabrik = 0;
    for k in 0..MAX_ZB {
        let mut s = zb(p, k);
        if s.groesse != 4 || s.phase < 5 { continue; }
        if s.spinne != 0 && !spinnen.contains(&s.spinne) { s.spinne = 0; }
        let nahPosten = |welt: &mut dyn Welt, u: Id| { let lo = welt.Location(u); (edX(welt, z, lo) - s.x).pow(2) + (edY(welt, z, lo) - s.y).pow(2) <= BASIS_UMKREIS * BASIS_UMKREIS };
        let alleAF = welt.PlayerBuildingEnum(p, mapArachnidFactory);
        let fab: Vec<Id> = alleAF.into_iter().filter(|&f| nahPosten(welt, f)).collect();
        eigeneFabrik.extend(fab.iter().copied());
        if s.spinne == 0 && !fab.is_empty() {
            // eigene Fabrik: eine neue Spinne an der Basis nehmen, sonst dort bauen
            let neu = spinnen.iter().copied().find(|&u| !gehoertBasis(z, u) && nahPosten(welt, u));
            if let Some(u) = neu { s.spinne = u; melde(welt, z, "reparatur", mapSpider, s.x, s.y, &format!("Spinne {} aus der eigenen Fabrik gehoert der zweiten Basis", u)); }
            else if let Some(&f) = fab.iter().find(|f| fabriken.contains(f)) {
                if welt.Ore(p) >= welt.GetOreCost(mapSpider, p) + 300 {
                    welt.DoProduce(f, mapSpider, mapNone);
                    melde(welt, z, "reparatur", mapSpider, s.x, s.y, "Spinne in Auftrag in der Spinnenfabrik der zweiten Basis");
                }
            }
        } else if s.spinne == 0 {
            ohneFabrik += 1;
            // eine freie Spinne der Hauptbasis, wenn dort danach noch zwei bleiben
            let frei: Vec<Id> = spinnen.iter().copied().filter(|&u| !gehoertBasis(z, u) && !welt.IsBusy(u) && welt.GetCurAction(u) == moDone).collect();
            let daheim = spinnen.iter().filter(|&&u| !gehoertBasis(z, u)).count();
            if daheim >= 3 { if let Some(&u) = frei.first() { s.spinne = u;
                melde(welt, z, "reparatur", mapSpider, s.x, s.y, &format!("Spinne {} faehrt zur zweiten Basis", u)); } }
        }
        if s.spinne != 0 {
            let lo = welt.Location(s.spinne);
            let weit = (edX(welt, z, lo) - s.x).pow(2) + (edY(welt, z, lo) - s.y).pow(2) > 144;
            if weit { konvoi(welt, z, k, 1, &[s.spinne], (s.x, s.y + 2)); }
        }
        setze(p, k, s);
    }
    // nachbauen in der Hauptbasis: zwei daheim, dazu je zweite Basis ohne eigene Fabrik eine
    let daheim = spinnen.iter().filter(|&&u| !gehoertBasis(z, u)).count();
    let soll = 2 + ohneFabrik;
    if daheim >= soll || welt.Ore(p) < welt.GetOreCost(mapSpider, p) + 300 { return; }
    if let Some(&af) = fabriken.iter().find(|f| !eigeneFabrik.contains(f)) {
        welt.DoProduce(af, mapSpider, mapNone);
        let lo = welt.Location(af); let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
        melde(welt, z, "reparatur", mapSpider, x, y, &format!("Spinne in Auftrag ({} von {}: zwei fuer die Hauptbasis, eine je zweite Basis ohne Fabrik)", daheim + 1, soll));
    }
}
