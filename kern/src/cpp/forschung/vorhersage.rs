//! Techbaum-Vorhersage: Nutzen je Technik (Kampf, Wirtschaft, Kolonie, Raum), Einheitswerte, Ziele je Bereich, Laborbedarf.
use super::*;
use super::{WERT_WT, WERT_LYNX};

/// C++: bool istWaffe(int typ)  (op2_ki.hpp:5350)
pub fn istWaffe(_welt: &mut dyn Welt, typ: i32) -> bool {
    matches!(typ, mapLaser | mapMicrowave | mapStickyfoam | mapRailGun | mapEMP | mapRPG | mapStarflare
        | mapStarflare2 | mapESG | mapSupernova | mapSupernova2 | mapAcidCloud | mapThorsHammer | mapEnergyCannon)
}

/// C++: bool istFahrgestell(int typ)  (op2_ki.hpp:5358)
pub fn istFahrgestell(_welt: &mut dyn Welt, typ: i32) -> bool {
    typ == mapLynx || typ == mapPanther || typ == mapTiger || typ == mapScorpion
}

/// C++: int sdkWert(int traeger, int waffe)  (op2_ki.hpp:5361)
// SDK-Wert fuer Fahrgestell/Turm + Waffe (wie kampfWert, aber ohne Einheit)
pub fn sdkWert(_welt: &mut dyn Welt, traeger: i32, waffe: i32) -> i32 {
    let stufe = if waffe == mapThorsHammer { 2 } else if waffe == mapLaser || waffe == mapMicrowave || waffe == mapNone { 0 } else { 1 };
    match traeger {
        mapSpider | mapScorpion => 4,
        mapLynx => 5 + stufe,
        mapPanther => 7 + stufe,
        mapTiger => 8 + stufe,
        mapGuardPost => if stufe == 2 { 7 } else { 4 + stufe },
        _ => 0,
    }
}

/// Wert einer Einheit fuer die Forschungsbewertung aus dem Arena-8-Modell (gegenwert mit den SDK-Werten des Spielers,
/// also mit Erforschtem); ohne Traeger die Punkttabelle.
pub fn einheitWertF(welt: &mut dyn Welt, z: &Zustand, traeger: i32, waffe: i32) -> f64 {
    if traeger == 0 { return sdkWert(welt, traeger, waffe) as f64; }
    if traeger == mapSpider { return 1.0; }
    let w = if waffe == mapNone || waffe <= 0 { if traeger == mapScorpion { mapEnergyCannon } else { mapLaser } } else { waffe };
    gegenwert(welt, z.spieler, traeger, w, 1.0) * super::paarung::paarFaktor(welt, z, traeger, w)   // Rust (PAR paarung)
}

/// Staerke je Erz der besten EMP-Beimischung relativ zur reinen Truppe (>= 1), wie in kampfFuehrung.
pub fn empMischFaktor(welt: &mut dyn Welt, z: &Zustand, traeger: i32, waffe: i32) -> f64 {
    let s = z.spieler;
    let kosten = |welt: &mut dyn Welt, f: i32, w: i32| (welt.GetOreCost(f, s) + welt.GetOreCost(w, s) + 2 * (welt.GetRareOreCost(f, s) + welt.GetRareOreCost(w, s))).max(1) as f64;
    let gm = einheitWertF(welt, z, traeger, waffe); let km = kosten(welt, traeger, waffe);
    if gm <= 0.0 { return 1.0; }
    let a = empVerstaerkung(waffe);
    let mut best = 1.0;
    for ec in [mapLynx, traeger] {
        let ge = gegenwert(welt, s, ec, mapEMP, 1.0); let ke = kosten(welt, ec, mapEMP);
        for i in 1..=5 {
            let q = i as f64 / 10.0;
            let v = ((1.0 - q) * gm * (1.0 + a * q) + q * ge) / ((1.0 - q) * km + q * ke) / (gm / km);
            if v > best { best = v; }
        }
    }
    best
}

/// C++: int truppen(const Zustand& z, Trupp* tr, int max)  (op2_ki.hpp:5377)
// Eigene Kampfeinheiten nach Traeger+Waffe zusammengefasst (Bestand + was das Soll noch verlangt; mit
// bekanntem Feind mindestens 4 Tuerme und 4 Lynx).
pub fn truppen(welt: &mut dyn Welt, z: &Zustand, tr: &mut [Trupp], max: i32) -> i32 {
    let mut n: i32 = 0;
    fn dazu(tr: &mut [Trupp], n: &mut i32, max: i32, t: i32, w: i32, k: i32) {
        for i in 0..*n as usize { if tr[i].traeger == t && tr[i].waffe == w { tr[i].anzahl += k; return; } }
        if *n < max { tr[*n as usize] = Trupp { traeger: t, waffe: w, anzahl: k }; *n += 1; }
    }
    for u in welt.PlayerVehicleEnum(z.spieler) {
        if kaempfer(welt, u) { let t = welt.GetType(u); let w = welt.GetWeapon(u); dazu(tr, &mut n, max, t, w, 1); }
    }
    for b in welt.PlayerBuildingEnum(z.spieler, mapGuardPost) {
        let w = if welt.GetWeapon(b) == mapNone { eigeneWaffe(welt, z) } else { welt.GetWeapon(b) };
        dazu(tr, &mut n, max, mapGuardPost, w, 1);
    }
    if z.bedrohung > 0 {
        let mut wt = 0;
        let mut ly = 0;
        for i in 0..n as usize { if tr[i].traeger == mapGuardPost { wt += tr[i].anzahl; } else { ly += tr[i].anzahl; } }
        if wt < 4 { let w = eigeneWaffe(welt, z); dazu(tr, &mut n, max, mapGuardPost, w, 4 - wt); }
        let mut mehr = (z.sollArmee + mobilSoll(welt, z, true) - z.eigeneArmee) / WERT_LYNX;
        if ly + mehr < 4 { mehr = 4 - ly; }
        if mehr > 0 { let w = eigeneWaffe(welt, z); dazu(tr, &mut n, max, mapLynx, w, mehr); }
    }
    n
}

/// Grundtempo der Kampf-Fahrgestelle (Einheitentabelle; kleiner = schneller)
fn grundTempo(typ: i32) -> i32 {
    if typ == mapLynx { 3 } else if typ == mapPanther { 4 } else if typ == mapTiger { 5 } else { 0 }
}

/// Panzerung zaehlt fuer Eden frueh mehr (gegen die Microwave der Plymouth-Lynx): Zuschlag 3, faellt bis Tick 80000 auf 0
fn panzerFruehFaktor(welt: &mut dyn Welt, z: &Zustand) -> f64 {
    if !welt.IsEden(z.spieler) { return 1.0; }
    1.0 + 3.0 * (1.0 - welt.Tick() as f64 / 80000.0).max(0.0)
}

/// C++: Kraftwerte werteJetzt(const Zustand& z, int traeger, int waffe)  (op2_ki.hpp:5396)
pub fn werteJetzt(welt: &mut dyn Welt, z: &Zustand, traeger: i32, waffe: i32) -> Kraftwerte {
    Kraftwerte {
        hp: welt.GetHitPoints(traeger, z.spieler),
        panzer: welt.GetArmor(traeger, z.spieler),
        pen: welt.GetPenetrationDamage(waffe, z.spieler),
        kon: welt.GetConcussionDamage(waffe, z.spieler),
        reload: welt.GetReloadTime(waffe, z.spieler),
    }
}

/// C++: double kampfNutzen(const Zustand& z, const tb::Tech& t, const Trupp* tr, int nt)  (op2_ki.hpp:5402)
// Kampf-Nutzen einer Technik (ohne Voraussetzungen), in SDK-Kampfwert
pub fn kampfNutzen(welt: &mut dyn Welt, z: &Zustand, t: &tb::Tech, tr: &[Trupp], nt: i32) -> f64 {
    let mut g = 0.0f64;
    let nt_u = nt as usize;
    for i in 0..t.nw {
        let w = &tb::WIRKUNG[(t.w0 + i) as usize];
        for k in 0..nt_u {
            if w.typ != tr[k].traeger && w.typ != tr[k].waffe { continue; }
            if w.eig == tb::E_MOVE_SPEED {
                // schnellere Kampffahrzeuge (kleinerer Wert = schneller, z.B. Lynx 3 -> 2 = +50 %): +100 % Tempo = ein Einheitwert
                let alt = grundTempo(tr[k].traeger);
                if w.typ == tr[k].traeger && w.wert > 0 && alt > w.wert {
                    g += tr[k].anzahl as f64 * einheitWertF(welt, z, tr[k].traeger, tr[k].waffe) * (alt as f64 / w.wert as f64 - 1.0);
                }
                continue;
            }
            let vor = werteJetzt(welt, z, tr[k].traeger, tr[k].waffe);
            if w.typ == tr[k].traeger && w.eig == tb::E_ARMOR {
                // Die Forschung setzt die Panzerklasse (0..5), die Welt meldet den Panzerwert (256..40). Nutzen = laengeres
                // Ueberleben gegen die Waffen der Feinde (Schaden je Treffer = Panzerwert * Erschuetterung / 256 + Durchschlag),
                // Kraft ~ Wurzel.
                let (neu, alt) = (panzerWert(w.wert), panzerJetzt(welt, tr[k].traeger, z.spieler));
                if neu < alt {
                    let (a, b) = (feindTreffer(welt, z.spieler, alt), feindTreffer(welt, z.spieler, neu));
                    if b > 0.0 && a > b {
                        let s = einheitWertF(welt, z, tr[k].traeger, tr[k].waffe);
                        g += tr[k].anzahl as f64 * s * ((a / b).sqrt() - 1.0) * panzerFruehFaktor(welt, z);
                    }
                }
                continue;
            }
            let mut nach = vor;
            if w.typ == tr[k].traeger && w.eig == tb::E_HIT_POINTS { nach.hp = w.wert; }
            else if w.typ == tr[k].waffe && w.eig == tb::E_RATE_OF_FIRE { nach.reload = w.wert; }
            else if w.typ == tr[k].waffe && w.eig == tb::E_PENETRATION_DAMAGE { nach.pen = w.wert; }
            else if w.typ == tr[k].waffe && w.eig == tb::E_CONCUSSION_DAMAGE { nach.kon = w.wert; }
            else { continue; }
            let a = kraft(welt, &vor, tr[k].waffe);
            let b = kraft(welt, &nach, tr[k].waffe);
            if a > 0.0 && b > a {
                let s = einheitWertF(welt, z, tr[k].traeger, tr[k].waffe);
                g += tr[k].anzahl as f64 * s * (b / a - 1.0);
            }
        }
    }
    let mut besterTraeger = 0;
    let mut besteWaffe = eigeneWaffe(welt, z);
    for k in 0..nt_u {
        if tr[k].traeger != mapGuardPost && einheitWertF(welt, z, tr[k].traeger, tr[k].waffe) > einheitWertF(welt, z, besterTraeger, besteWaffe) {
            besterTraeger = tr[k].traeger; besteWaffe = tr[k].waffe;
        }
    }
    let mut fz = 0;
    let mut wt = 0;
    for k in 0..nt_u { if tr[k].traeger == mapGuardPost { wt += tr[k].anzahl; } else { fz += tr[k].anzahl; } }
    for i in 0..t.nf {
        let u = tb::FREI[(t.f0 + i) as usize];
        if u == mapEMP {
            // EMP toetet nicht, es verstaerkt die Truppe -- Nutzen = Gewinn der besten Beimischung.
            for k in 0..nt_u {
                if tr[k].traeger == mapGuardPost { continue; }
                let gw = einheitWertF(welt, z, tr[k].traeger, tr[k].waffe);
                let f = empMischFaktor(welt, z, tr[k].traeger, tr[k].waffe);
                g += tr[k].anzahl as f64 * gw * (f - 1.0);
            }
        } else if istWaffe(welt, u) {
            for k in 0..nt_u {
                let d = einheitWertF(welt, z, tr[k].traeger, u) - einheitWertF(welt, z, tr[k].traeger, tr[k].waffe);
                if d > 0.0 { g += d * tr[k].anzahl as f64; }
            }
        } else if istFahrgestell(welt, u) {
            // Ein besseres Fahrgestell ist nichts wert, solange es das erste (Lynx) noch nicht gibt.
            let d = einheitWertF(welt, z, u, besteWaffe) - if besterTraeger != 0 { einheitWertF(welt, z, besterTraeger, besteWaffe) } else { 0.0 };
            if d > 0.0 { g += d * if fz > 4 { fz } else { 4 } as f64; }
        } else if u == mapGuardPost && wt == 0 {
            g += (4 * WERT_WT) as f64;
        } else if u == mapRepairVehicle && z.bedrohung > 0 {
            g += 6.0; // haelt Tuerme und Fahrzeuge im Kampf
        } else if u == mapWall && z.bedrohung > 0 {
            g += 2.0;
        }
    }
    g
}

/// C++: double wirtschaftNutzen(const Zustand& z, const Lage& l, const tb::Tech& t)  (op2_ki.hpp:5444)
// Wirtschafts-Nutzen in Punkten: Kapazitaet/Ertrag/Strombedarf/Tempo eigener Einheiten, nach Bedarf gewichtet
pub fn wirtschaftNutzen(welt: &mut dyn Welt, z: &Zustand, l: &Lage, t: &tb::Tech) -> f64 {
    let mut g = 0.0f64;
    let stromKnapp = l.stromFrei < 150;
    let erzKnapp = l.erz < 3000;
    for i in 0..t.nw {
        let w = &tb::WIRKUNG[(t.w0 + i) as usize];
        if w.typ < 0 || w.typ >= 128 { continue; }
        let n = l.geb[w.typ as usize] + l.fz[w.typ as usize];
        if n == 0 { continue; }
        match w.eig {
            tb::E_PRODUCTION_CAPACITY => {
                let alt = welt.GetProductionCapacity(w.typ, z.spieler);
                if alt > 0 && w.wert > alt {
                    let strom = w.typ == mapTokamak || w.typ == mapMHDGenerator || w.typ == mapSolarPowerArray || w.typ == mapGeothermalPlant;
                    g += n as f64 * 100.0 * (w.wert - alt) as f64 / alt as f64 * if strom { if stromKnapp { 1.0 } else { 0.3 } } else { 0.5 };
                }
            }
            tb::E_IMPROVED => {
                if w.typ == mapCommonOreMine || w.typ == mapCommonOreSmelter { g += (n * if erzKnapp { 40 } else { 20 }) as f64; }
                else { g += (n * 10) as f64; }
            }
            tb::E_POWER_REQUIRED => {
                let alt = welt.GetPowerRequired(w.typ, z.spieler);
                if w.wert < alt { g += (n * (alt - w.wert)) as f64 * if stromKnapp { 0.5 } else { 0.2 }; }
            }
            tb::E_PRODUCTION_RATE => { g += (n * 5) as f64; }
            tb::E_MOVE_SPEED => { if w.typ == mapCargoTruck || w.typ == mapConVec { g += (n * 4) as f64; } }
            tb::E_HIT_POINTS => { if w.typ == mapConVec || w.typ == mapCargoTruck { g += (n * 2) as f64; } }
            _ => {}
        }
    }
    g
}

/// C++: double kolonieNutzen(const Zustand& z, const Lage& l, const tb::Tech& t)  (op2_ki.hpp:5483)
// Kolonie-Nutzen in "Bewohnern, denen etwas fehlt"; Katastrophenforschung ohne messbare Wirkung: 1.
pub fn kolonieNutzen(welt: &mut dyn Welt, z: &Zustand, l: &Lage, t: &tb::Tech) -> f64 {
    let bew = if l.menschen > 0 { l.menschen } else { 1 };
    let mut g = 0.0f64;
    for i in 0..t.nf {
        let u = tb::FREI[(t.f0 + i) as usize];
        if u == mapNursery && l.geb[mapNursery as usize] == 0 { g += bew as f64; }
        else if u == mapUniversity && l.geb[mapUniversity as usize] == 0 { g += (welt.Kids(z.spieler) + 10) as f64; }
        else if u == mapMedicalCenter && l.geb[mapMedicalCenter as usize] == 0 { g += bew as f64; }
        else if u == mapRecreationFacility && l.geb[mapRecreationFacility as usize] == 0 { g += bew as f64; }
        else if (u == mapAdvancedResidence || u == mapReinforcedResidence) && l.wohnraum > 0 && l.menschen * 100 / l.wohnraum > 85 { g += 10.0; }
        else if u == mapDIRT || u == mapGORF || u == mapMeteorDefense { g += 2.0; }
    }
    for i in 0..t.nw {
        let w = &tb::WIRKUNG[(t.w0 + i) as usize];
        if w.typ < 0 || w.typ >= 128 || w.eig != tb::E_PRODUCTION_CAPACITY { continue; }
        let n = l.geb[w.typ as usize];
        if n == 0 { continue; }
        let plus = (w.wert - welt.GetProductionCapacity(w.typ, z.spieler)) * n;
        if plus <= 0 { continue; }
        if w.typ == mapResidence {
            let luecke = l.menschen + 10 - l.wohnraum;
            g += if luecke > 0 { (if plus < luecke { plus } else { luecke }) as f64 } else { plus as f64 * 0.2 };
        } else if w.typ == mapMedicalCenter {
            let luecke = bew - l.medKap;
            g += if luecke > 0 { (if plus < luecke { plus } else { luecke }) as f64 } else { plus as f64 * 0.1 };
        } else if w.typ == mapRecreationFacility {
            let luecke = bew - l.freizeitKap;
            g += if luecke > 0 { (if plus < luecke { plus } else { luecke }) as f64 } else { plus as f64 * 0.1 };
        } else if w.typ == mapAgridome {
            g += if l.nahrungNetto < 20 { plus as f64 } else { plus as f64 * 0.1 };
        } else {
            g += plus as f64 * 0.1;
        }
    }
    // Warntechs heben Summe und Teiler der Engine-Moral um je 2 -- Gewinn aus dem Moralmodell
    if WARNTECHS.contains(&t.id) {
        let (ms, mt) = moralSummeTeiler(welt, z, l);
        let fak = if welt.IsEden(z.spieler) { 0.92 } else { 1.0 };   // morale.txt DIFF_MED / DIFF_MED_PLY
        let vorher = (100.0 * ms as f64 / mt as f64 * fak).clamp(0.0, 99.0);   // die Engine haelt die Moral in 0..99
        let nachher = (100.0 * (ms + 2) as f64 / (mt + 2) as f64 * fak).clamp(0.0, 99.0);
        g += moralNutzen(z, l, vorher, nachher).max(0.0);
    }
    if g == 0.0 && t.kat == 10 { g = 1.0; } // Katastrophen: Wirkung nicht messbar
    g
}

/// C++: double spaceNutzen(const Zustand& z, const Lage& l, const tb::Tech& t)  (op2_ki.hpp:5513)
// Space-Nutzen: was das Ziel noch freischalten muss (Spaceport, SULV, EDWARD)
pub fn spaceNutzen(_welt: &mut dyn Welt, z: &Zustand, l: &Lage, t: &tb::Tech) -> f64 {
    if z.ziel != 1 || z.zielErreicht != 0 { return 0.0; }
    let mut g = 0.0f64;
    for i in 0..t.nf {
        let u = tb::FREI[(t.f0 + i) as usize];
        if (u == mapSpaceport && l.geb[mapSpaceport as usize] == 0) || u == mapSULV || u == mapEDWARDSatellite { g += 100.0; }
    }
    g
}

/// Lambda `nutzen` aus bereichZiele.
pub(super) fn bereichNutzen(welt: &mut dyn Welt, z: &Zustand, l: &Lage, bereich: i32, tt: &tb::Tech, tr: &[Trupp], nt: i32) -> f64 {
    if bereich == 1 { kampfNutzen(welt, z, tt, tr, nt) }
    else if bereich == 2 { wirtschaftNutzen(welt, z, l, tt) }
    else if bereich == 0 { kolonieNutzen(welt, z, l, tt) }
    else { spaceNutzen(welt, z, l, tt) }
}

/// C++: int bereichZiele(const Zustand& z, const Lage& l, int bereich, Vorhersage* out, int max)  (op2_ki.hpp:5526)
// Beste Ziele eines Bereichs nach Nutzen pro Wegkosten, absteigend. Liefert Anzahl.
/// Kann alles, was diese Technik freischaltet, nur mit Seltenerz gebaut werden? (Einheitentabelle, mindestens eine Einheit)
pub fn braeuchteSelten(id: i32) -> bool {
    let mut n = 0;
    for e in tb::EINHEIT.iter() { if e.tech == id { if e.selten <= 0 { return false; } n += 1; } }
    n > 0
}

pub fn bereichZiele(welt: &mut dyn Welt, z: &Zustand, l: &Lage, bereich: i32, out: &mut [Vorhersage], max: i32) -> i32 {
    let mut tr = [Trupp::default(); 16];
    let nt = if bereich == 1 { truppen(welt, z, &mut tr, 16) } else { 0 };
    let mut n: i32 = 0;
    for i in 0..tb::ANZ_TECH as usize {
        let t = &tb::TECH[i];
        if t.bereich != bereich || hatTechId(welt, z, t.id) { continue; }
        let eigen = eigeneKosten(welt, z, t);
        if eigen < 0 { continue; }
        if wegVerteuert(welt, z, t.id, 0) { continue; } // wartet auf die Seltenerz-Mine
        // Was nur mit Seltenerz gebaut werden kann (Acid Weaponry, Panther, Thor's Hammer), zaehlt als eigenes Ziel nur zu 30 %,
        // solange kein Seltenerz da ist (keine Schmelze, Lager leer) -- als Voraussetzung (Panther fuer die Railgun) voll.
        let seltenFehlt = braeuchteSelten(t.id) && l.selten <= 0 && l.geb[mapRareOreSmelter as usize] == 0;
        let mut v = bereichNutzen(welt, z, l, bereich, t, &tr, nt);
        // Freie Folgetechniken (Kosten 0), die mit dieser fertig werden, zaehlen mit (z.B. 3407 -> Laser, Wachturm)
        for j in 0..tb::ANZ_TECH as usize {
            let f = &tb::TECH[j];
            if eigeneKosten(welt, z, f) != 0 || hatTechId(welt, z, f.id) { continue; }
            let fi = techIndex(welt, f.id);
            if fi < 0 { continue; }
            let ti = techInfo(welt, fi);
            let mut braucht = false;
            let mut rest = true;
            for r in 0..ti.required.len() {
                let rq = techInfo(welt, ti.required[r]);
                let rid = rq.id;
                if rid == t.id { braucht = true; }
                else if !hatTechId(welt, z, rid) {
                    match tb::tech(rid) {
                        None => rest = false,
                        Some(rt) => { if eigeneKosten(welt, z, rt) != 0 { rest = false; } } // andere Kolonie-Variante (Kosten 0) zaehlt als da
                    }
                }
            }
            if braucht && rest { v += bereichNutzen(welt, z, l, bereich, f, &tr, nt); }
        }
        // Forschung fuer Forschung (Spieler): ein Teil (freischaltWert) des Nutzens der Techniken, die diese direkt
        // voraussetzen -- schaltet die Folgetechnik eine Einheit frei (Railgun), zaehlt ihr Nutzen voll (Panther).
        let mut folge = 0.0f64;
        for j in 0..tb::ANZ_TECH as usize {
            let f = &tb::TECH[j];
            if f.id == t.id || hatTechId(welt, z, f.id) || eigeneKosten(welt, z, f) <= 0 || f.bereich != bereich { continue; }
            let fi = techIndex(welt, f.id);
            if fi < 0 { continue; }
            let ti = techInfo(welt, fi);
            for r in 0..ti.required.len() {
                if techInfo(welt, ti.required[r]).id == t.id {
                    let gw = if f.nf > 0 { 1.0 } else { PAR(z.spieler).freischaltWert };
                    folge += gw * bereichNutzen(welt, z, l, bereich, f, &tr, nt);
                    break;
                }
            }
        }
        // Eden: das Reparaturfahrzeug (5401) ist kein eigenes Ziel (Spiegelpartien: es kostete Eden Siege)
        let tf = if t.id == 5401 && welt.IsEden(z.spieler) { 0.3 } else { 1.0 };
        v *= tf;
        if seltenFehlt { v *= 0.3; }
        v += folge * tf;   // die Rolle als Voraussetzung ohne Seltenerz-Abschlag
        // eine Linie halten: das zuletzt beste Ziel des Bereichs bleibt vorn, bis ein anderes klar besser ist
        if t.id == linieZiel(z.spieler, bereich) { v *= 1.5; }
        if v <= 0.5 { continue; }
        let k = wegKosten(welt, z, t.id);
        if k <= 0 { continue; }
        let neu = Vorhersage { id: t.id, nutzen: v, kosten: k };
        // Volle Liste: nur besser als der letzte Eintrag kommt hinein (wie jetzt in C++; frueher las C++ beim Fuellen des
        // letzten Platzes einen nie geschriebenen Eintrag -- buildabhaengig).
        let voll = n >= max;
        if voll && out[(max - 1) as usize].nutzen / out[(max - 1) as usize].kosten as f64 >= v / k as f64 { continue; }
        let mut pos = (if voll { max - 1 } else { n += 1; n - 1 }) as usize;
        out[pos] = neu;
        while pos > 0 && out[pos].nutzen / out[pos].kosten as f64 > out[pos - 1].nutzen / out[pos - 1].kosten as f64 {
            out.swap(pos, pos - 1);
            pos -= 1;
        }
    }
    n
}

/// C++: int laborImWeg(const Zustand& z, const Lage& l, int id)  (op2_ki.hpp:5585)
// Welcher Labortyp fehlt auf dem Weg zu `id`? 0 = keiner.
pub fn laborImWeg(welt: &mut dyn Welt, z: &Zustand, l: &Lage, id: i32) -> i32 {
    let mut stapel = [0i32; 64];
    let mut gesehen = [0i32; 96];
    let mut ns = 0usize;
    let mut ng = 0usize;
    let mut fehlt = 0;
    stapel[ns] = id; ns += 1;
    while ns != 0 {
        ns -= 1;
        let t = stapel[ns];
        let mut schon = false;
        for k in 0..ng { if gesehen[k] == t { schon = true; break; } }
        if schon || hatTechId(welt, z, t) { continue; }
        if ng < 96 { gesehen[ng] = t; ng += 1; }
        if let Some(tt) = tb::tech(t) {
            if eigeneKosten(welt, z, tt) > 0 {
                let typ = if tt.lab == 3 { mapAdvancedLab } else if tt.lab == 2 { mapStandardLab } else if tt.lab == 1 { mapBasicLab } else { 0 };
                if typ != 0 && l.geb[typ as usize] == 0 && geplant(welt, z, typ) == 0 && (fehlt == 0 || typ == mapStandardLab) { fehlt = typ; } // Standard zuerst
            }
        }
        let idx = techIndex(welt, t);
        if idx < 0 { continue; }
        let ti = techInfo(welt, idx);
        for r in 0..ti.required.len() {
            if ns < 64 { stapel[ns] = techInfo(welt, ti.required[r]).id; ns += 1; }
        }
    }
    if fehlt == mapBasicLab { fehlt = mapStandardLab; } // Basic Lab gibt es nicht (02106 "NOT AVAILABLE")
    fehlt
}

/// C++: int laborTypenImWeg(const Zustand& z, int id)  (op2_ki.hpp:5609)
// Labortypen (Bitmaske 1<<lab), die auf dem Weg zu `id` noch forschen muessen
pub fn laborTypenImWeg(welt: &mut dyn Welt, z: &Zustand, id: i32) -> i32 {
    let mut stapel = [0i32; 64];
    let mut gesehen = [0i32; 96];
    let mut ns = 0usize;
    let mut ng = 0usize;
    let mut maske = 0;
    stapel[ns] = id; ns += 1;
    while ns != 0 {
        ns -= 1;
        let t = stapel[ns];
        let mut schon = false;
        for k in 0..ng { if gesehen[k] == t { schon = true; break; } }
        if schon || hatTechId(welt, z, t) { continue; }
        if ng < 96 { gesehen[ng] = t; ng += 1; }
        if let Some(tt) = tb::tech(t) {
            if eigeneKosten(welt, z, tt) > 0 && tt.lab >= 1 && tt.lab <= 3 { maske |= 1 << tt.lab; }
        }
        let idx = techIndex(welt, t);
        if idx < 0 { continue; }
        let ti = techInfo(welt, idx);
        for r in 0..ti.required.len() {
            if ns < 64 { stapel[ns] = techInfo(welt, ti.required[r]).id; ns += 1; }
        }
    }
    maske
}

/// C++: void laborBedarf(Zustand& z, const Lage& l)  (op2_ki.hpp:5630)
// Alle 1000 Ticks: fehlt ein Labor auf dem Weg der besten Ziele, bekommt dessen Bau die Dringlichkeit des Bereichs.
pub fn laborBedarf(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    let t = welt.Tick();
    if t < z.naechsterLaborCheck { return; }
    z.naechsterLaborCheck = t + 1000;
    let d = dringlichkeit(welt, z, l);
    let anteil: [i32; 4] = [d.kolonie, d.kampfForschung, d.wirtschaft, d.ziel];
    let mut wunsch = 0;
    let mut wert = 0;
    let mut fuer = 0;
    let mut nutzen: [i32; 4] = [0, 0, 0, 0];
    for b in 0..4usize {
        if anteil[b] <= 0 { continue; }
        let mut v = [Vorhersage::default(); 3];
        let n = bereichZiele(welt, z, l, b as i32, &mut v, 3);
        for i in 0..n as usize {
            let maske = laborTypenImWeg(welt, z, v[i].id);
            for lt in 1..=3usize { if (maske & (1 << lt)) != 0 && anteil[b] > nutzen[lt] { nutzen[lt] = anteil[b]; } }
            if anteil[b] > wert {
                let lab = laborImWeg(welt, z, l, v[i].id);
                if lab != 0 { wunsch = lab; wert = anteil[b]; fuer = v[i].id; }
            }
        }
    }
    // Eroeffnung: ein Labor, das eines ihrer offenen Themen braucht, geht vor -- vorausschauend, damit das Advanced Lab
    // fuer den Lynx steht, wenn seine Vorstufen fertig sind (Dringlichkeit 80)
    for id in super::labore::eroeffnungOffen(welt, z) {
        let lab = laborImWeg(welt, z, l, id);
        if lab != 0 {
            if 80 > wert { wunsch = lab; wert = 80; fuer = id; }
            break;
        }
    }
    for lt in 0..4 { z.laborNutzen[lt] = nutzen[lt]; }
    if wunsch != z.laborWunsch && wunsch != 0 {
        let nm = name(welt, wunsch);
        melde(welt, z, "info", wunsch, z.bx, z.by, &format!("Forschung braucht ein {} (Ziel {}, Dringlichkeit {})", nm, fuer, wert));
    }
    z.laborWunsch = wunsch; z.laborWert = wert; z.laborFuer = fuer;
}
