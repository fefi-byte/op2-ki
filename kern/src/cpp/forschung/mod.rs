//! Forschung: Labore (Themenwahl), Techbaum-Vorhersage, Ausbildung, Garage und Reparatur, Raumfahrt, Selbstpruefung.
//! C++-KI op2_ki.hpp Zeilen 4990-6329, nach Rust uebertragen.
use super::*;

mod garage;
mod vorhersage;
mod pruefung;
mod labore;
mod ausbildung;
mod raumfahrt;
pub use garage::*;
pub use vorhersage::*;
pub use pruefung::*;
pub use labore::*;
pub use ausbildung::*;
pub use raumfahrt::*;
use std::cell::RefCell;

// ---------------------------------------------------------------- fremde Konstanten (Dubletten beim Zusammenfuegen entfernen)
const TECH_RAUMFAHRT: i32 = 5405; // FREMD aus Zeile 521
const WERT_WT: i32 = 4; // FREMD aus Zeile 981
const WERT_LYNX: i32 = 5; // FREMD aus Zeile 982
const EMP_TECH: i32 = 12201; // FREMD aus Zeile 1037
// FREMD aus Zeile 577
const GEBAEUDE: [i32; 38] = [
    mapCommandCenter, mapStructureFactory, mapVehicleFactory, mapTokamak, mapMHDGenerator, mapSolarPowerArray,
    mapCommonOreSmelter, mapRareOreSmelter, mapCommonOreMine, mapRareOreMine, mapCommonStorage, mapRareStorage,
    mapAgridome, mapResidence, mapReinforcedResidence, mapAdvancedResidence, mapBasicLab, mapStandardLab,
    mapAdvancedLab, mapMedicalCenter, mapNursery, mapUniversity, mapRecreationFacility, mapForum, mapDIRT,
    mapGarage, mapConsumerFactory, mapGuardPost, mapLightTower, mapGORF, mapRobotCommand, mapObservatory,
    mapMeteorDefense, mapTradeCenter, mapArachnidFactory, mapSpaceport, mapGeothermalPlant, mapMagmaWell,
];

// ---------------------------------------------------------------- Hilfen dieses Moduls

/// std::snprintf(buf, n, "%s", s): hoechstens n-1 Bytes (an einer Zeichengrenze).
fn kurz(s: String, n: usize) -> String {
    if n == 0 { return String::new(); }
    if s.len() <= n - 1 { return s; }
    let mut e = n - 1;
    while e > 0 && !s.is_char_boundary(e) { e -= 1; }
    s[..e].to_string()
}

/// FEHLT: TechInfo::GetTechName() -- welt.rs/TechInfo liefern keinen Technamen. Bis dahin: kein Name (C++ setzt dann "?").
fn techName(_welt: &mut dyn Welt, _idx: i32) -> Option<&'static str> { None }

thread_local! {
    /// techIndex: static int ids[512], anzahl = -1 (Cache techID -> Tabellenindex; die Daten aendern sich nicht)
    static TECH_IDS: RefCell<(Vec<i32>, i32)> = RefCell::new((vec![0; 512], -1));
}

/// Zustandsbehaftete statics dieses Moduls zuruecksetzen. Der techIndex-Cache bleibt (Techtabelle ist fest),
/// artText in forschung() ist nur ein Puffer -- es gibt hier nichts zurueckzusetzen.
pub fn zuruecksetzen_forschung() {
    TECH_IDS.with(|c| *c.borrow_mut() = (vec![0; 512], -1));
    TECH_INFO.with(|c| c.borrow_mut().clear());
    TECH_NR.with(|c| c.borrow_mut().clear());
    HAT_TECH.with(|c| *c.borrow_mut() = Default::default());
}

// Beschleunigung (nur Rust, Entscheidungen unveraendert): der Techbaum steht waehrend der Partie fest -- GetTechInfo einmal je
// Index lesen; techID -> Index ueber eine Tabelle; "hat Spieler p Technik i" einmal je Tick und Spieler fragen.
thread_local! {
    static TECH_INFO: RefCell<Vec<TechInfo>> = const { RefCell::new(Vec::new()) };
    /// techID -> erster Tabellenindex (-1 = keiner), direkt ueber die ID
    static TECH_NR: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
    /// je Spieler: (Tick, je ID 0 = noch nicht gefragt, 1 = nein, 2 = ja)
    static HAT_TECH: RefCell<[(i32, Vec<u8>); 8]> = RefCell::new(Default::default());
}
/// GetTechInfo aus dem Speicher.
pub fn techInfo(welt: &mut dyn Welt, i: i32) -> TechInfo {
    if i < 0 { return welt.GetTechInfo(i); }
    TECH_INFO.with(|c| {
        let mut c = c.borrow_mut();
        if c.is_empty() { let n = welt.GetTechCount(); for k in 0..n { c.push(welt.GetTechInfo(k)); } }
        c.get(i as usize).copied().unwrap_or_else(|| welt.GetTechInfo(i))
    })
}
/// HasTechnology, je Tick einmal gefragt (Technik-Stand aendert sich nur im Spieltakt).
pub fn hatTechnologie(welt: &mut dyn Welt, p: i32, id: i32) -> bool {
    if !(0..8).contains(&p) || !(0..65536).contains(&id) { return welt.HasTechnology(p, id); }
    let t = welt.Tick();
    let alt = HAT_TECH.with(|c| {
        let mut c = c.borrow_mut();
        let e = &mut c[p as usize];
        if e.0 != t { e.0 = t; e.1.iter_mut().for_each(|x| *x = 0); }
        e.1.get(id as usize).copied().unwrap_or(0)
    });
    if alt != 0 { return alt == 2; }
    let v = welt.HasTechnology(p, id);
    HAT_TECH.with(|c| { let mut c = c.borrow_mut(); let e = &mut c[p as usize].1; if e.len() <= id as usize { e.resize(id as usize + 1, 0); } e[id as usize] = if v { 2 } else { 1 }; });
    v
}

// ---------------------------------------------------------------------------------------------------
// Forschung / research: freie Labore suchen sich ein Thema

/// C++: int laborTyp(int unitTyp)  (op2_ki.hpp:5161)
pub fn laborTyp(_welt: &mut dyn Welt, unitTyp: i32) -> i32 {
    if unitTyp == mapBasicLab { ltBasic } else if unitTyp == mapStandardLab { ltStandard } else if unitTyp == mapAdvancedLab { ltAdvanced } else { 0 }
}

/// C++: bool techVerteuert(const Zustand& z, int id)  (op2_ki.hpp:5170)
// Technik, die Einheiten ploetzlich Seltenerz kosten laesst (z.B. 5317), erst forschen, wenn eine Seltenerz-Mine steht.
pub fn techVerteuert(welt: &mut dyn Welt, z: &Zustand, id: i32) -> bool {
    // (Rust: "steht eine Seltenerz-Mine" einmal je Tick und Spieler -- die Vorhersage fragt das je Technik und Voraussetzung)
    thread_local! { static MINE: std::cell::Cell<(i32, i32, bool)> = const { std::cell::Cell::new((-1, -1, false)) }; }
    let t = welt.Tick();
    let (mt, ms, mv) = MINE.with(|c| c.get());
    let mine = if mt == t && ms == z.spieler { mv } else {
        let mut da = false;
        for m in welt.PlayerBuildingEnum(z.spieler, mapRareOreMine) { if welt.IsLive(m) { da = true; break; } }
        MINE.with(|c| c.set((t, z.spieler, da)));
        da
    };
    if mine { return false; }
    let t = match tb::tech(id) { Some(t) => t, None => return false };
    for i in 0..t.nw {
        let w = &tb::WIRKUNG[(t.w0 + i) as usize];
        if w.eig != tb::E_RARE_REQUIRED || w.wert <= 0 || w.typ <= 0 || w.typ >= 128 { continue; }
        if welt.GetRareOreCost(w.typ, z.spieler) == 0 { return true; }
    }
    false
}

/// C++: bool wegVerteuert(const Zustand& z, int id, int tiefe = 0)  (op2_ki.hpp:5183)
// ... auch, wenn eine noch fehlende Voraussetzung so eine Technik ist (rekursiv).
pub fn wegVerteuert(welt: &mut dyn Welt, z: &Zustand, id: i32, tiefe: i32) -> bool {
    if tiefe > 12 || hatTechId(welt, z, id) { return false; }
    if techVerteuert(welt, z, id) { return true; }
    let ti = techIndex(welt, id);
    if ti < 0 { return false; }
    let info = techInfo(welt, ti);
    for r in 0..info.required.len() {
        let rid = techInfo(welt, info.required[r]).id;
        if wegVerteuert(welt, z, rid, tiefe + 1) { return true; }
    }
    false
}

/// C++: int themaWaehlen(const Zustand& z, int lab, char* name, int n)  (op2_ki.hpp:5194)
pub fn themaWaehlen(welt: &mut dyn Welt, z: &Zustand, lab: i32, name: &mut String, n: i32) -> i32 {
    const vorrang: [i32; 11] = [tcFood, tcMetals, tcPower, tcPopulation, tcMorale, tcBasic, tcVehicles, tcDefense, tcDisaster, tcWeapons, tcSpace];
    let anzahl = welt.GetTechCount();
    let mut besteIdx = -1;
    let mut besterRang = 99;
    let mut besteKosten = 1 << 30;
    for i in 0..anzahl {
        let ti = techInfo(welt, i);
        if !ti.valid { continue; }
        if welt.GetPlayerHasTech(i, z.spieler) { continue; }
        if ti.lab != lab { continue; }
        if techVerteuert(welt, z, ti.id) { continue; } // wartet auf die Seltenerz-Mine (Lueckenfueller nahm 5317)
        { let id = ti.id; if id == 3301 || id == 3302 || id == 3306 || id == 3406 { continue; } } // Moraltest: kostet Moral
        let kosten = if welt.IsEden(z.spieler) { ti.eden_cost } else { ti.plymouth_cost };
        if kosten <= 0 { continue; } // 0 = frei, -1 = nicht erforschbar
        let mut voraussetzungen = true;
        let mut r = 0;
        while r < ti.required.len() && voraussetzungen {
            let ri = ti.required[r];
            let req = techInfo(welt, ri);
            if !req.valid || !welt.GetPlayerHasTech(ri, z.spieler) { voraussetzungen = false; }
            r += 1;
        }
        if !voraussetzungen { continue; }
        let mut rang = 50;
        for k in 0..vorrang.len() { if vorrang[k] == ti.category { rang = k as i32; break; } }
        if rang < besterRang || (rang == besterRang && kosten < besteKosten) { besteIdx = i; besterRang = rang; besteKosten = kosten; }
    }
    if besteIdx >= 0 {
        let nm = techName(welt, besteIdx);
        *name = kurz(nm.unwrap_or("?").to_string(), n as usize);
    }
    besteIdx
}

/// C++: bool hatIdx(const Zustand& z, int idx)  (op2_ki.hpp:5224)
pub fn hatIdx(welt: &mut dyn Welt, z: &Zustand, idx: i32) -> bool {
    let ti = techInfo(welt, idx);
    ti.valid && welt.GetPlayerHasTech(idx, z.spieler)
}

/// C++: int techIndex(int techID)  (op2_ki.hpp:5229)
pub fn techIndex(welt: &mut dyn Welt, techID: i32) -> i32 {
    // Zuordnung techID -> Tabellenindex einmal aufbauen (die Vorhersage fragt sehr oft)
    TECH_IDS.with(|c| {
        let mut c = c.borrow_mut();
        if c.1 < 0 {
            let mut anzahl = welt.GetTechCount();
            if anzahl > 512 { anzahl = 512; }
            TECH_NR.with(|m| m.borrow_mut().clear());
            for i in 0..anzahl {
                let ti = techInfo(welt, i);
                c.0[i as usize] = if ti.valid { ti.id } else { -1 };
                // erster Index je ID, wie die lineare Suche
                if ti.valid && (0..65536).contains(&ti.id) {
                    TECH_NR.with(|m| { let mut m = m.borrow_mut(); if m.len() <= ti.id as usize { m.resize(ti.id as usize + 1, -1); } if m[ti.id as usize] < 0 { m[ti.id as usize] = i; } });
                }
            }
            c.1 = anzahl;
        }
        if techID < 0 { return -1; }
        TECH_NR.with(|m| m.borrow().get(techID as usize).copied().unwrap_or(-1))
    })
}

/// C++: int zielThema(const Zustand& z, int zielTech, int lab, const int* belegt, int nb, char* name, int n)  (op2_ki.hpp:5244)
// Naechstes erforschbares Thema auf dem Weg zu `zielTech` fuer diesen Labortyp. Billigstes zuerst. -1 = keins.
pub fn zielThema(welt: &mut dyn Welt, z: &Zustand, zielTech: i32, lab: i32, belegt: &[i32], nb: i32, name: &mut String, n: i32) -> i32 {
    let start = techIndex(welt, zielTech);
    if start < 0 || hatIdx(welt, z, start) { return -1; }
    let mut stapel = [0i32; 64];
    let mut gesehen = [0i32; 64];
    let mut ns = 0usize;
    let mut ng = 0usize;
    stapel[ns] = start; ns += 1;
    let mut beste = -1;
    let mut besteKosten = 1 << 30;
    while ns != 0 {
        ns -= 1;
        let i = stapel[ns];
        let mut schon = false;
        for k in 0..ng { if gesehen[k] == i { schon = true; break; } }
        if schon { continue; }
        if ng < 64 { gesehen[ng] = i; ng += 1; }
        if hatIdx(welt, z, i) { continue; }
        let ti = techInfo(welt, i);
        if !ti.valid { continue; }
        let mut alle = true;
        for r in 0..ti.required.len() {
            let req = ti.required[r];
            if !hatIdx(welt, z, req) { alle = false; if ns < 64 { stapel[ns] = req; ns += 1; } }
        }
        if !alle { continue; }
        let kosten = if welt.IsEden(z.spieler) { ti.eden_cost } else { ti.plymouth_cost };
        if kosten <= 0 { continue; } // frei: kommt mit den Voraussetzungen
        if techVerteuert(welt, z, ti.id) { continue; } // wartet auf die Seltenerz-Mine
        // GORF-Forschung raus (Spieler)
        { let id = ti.id; if id == 3301 || id == 3302 || id == 3306 || id == 3406 { continue; } } // Moraltest: kostet Moral
        if ti.lab != lab { continue; }
        let mut laeuft = false;
        for k in 0..nb as usize { if belegt[k] == ti.id || belegt[k] == i { laeuft = true; } }
        if laeuft { continue; }
        if kosten < besteKosten { beste = i; besteKosten = kosten; }
    }
    if beste >= 0 {
        let nm = techName(welt, beste);
        *name = kurz(nm.unwrap_or("?").to_string(), n as usize);
    }
    beste
}

/// C++: int themaGruppe(int techID)  (op2_ki.hpp:5286)
// Vorrang eines Forschungsthemas: 2 Ziel, 1 Kolonie, 0 Lueckenfueller/sonstiges
pub fn themaGruppe(_welt: &mut dyn Welt, techID: i32) -> i32 {
    match techID {
        3305 | 3401 | 3402 | 3405 | 5110 | 5405 | 3502 | 5104 => 2,
        3407 | 3851 | 5051 => 2, // Militaer (Laser/Wachturm, Lynx)
        3304 | 3303 | 3406 | 5307 | 3403 => 1,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------------------------------
// Techbaum-Vorhersage / tech tree prediction (Nutzen pro Forschungskosten einschliesslich fehlender Voraussetzungen)

/// C++: int eigeneKosten(const Zustand& z, const tb::Tech& t)  (op2_ki.hpp:5316)
pub fn eigeneKosten(welt: &mut dyn Welt, z: &Zustand, t: &tb::Tech) -> i32 {
    if welt.IsEden(z.spieler) { t.eden } else { t.ply }
}

/// C++: bool hatTechId(const Zustand& z, int id)  (op2_ki.hpp:5321)
// Nummer 0 ist der Platzhalter "keine Voraussetzung": zaehlt als vorhanden.
pub fn hatTechId(welt: &mut dyn Welt, z: &Zustand, id: i32) -> bool {
    id == 0 || hatTech(welt, z, id)
}

/// C++: int wegKosten(const Zustand& z, int id)  (op2_ki.hpp:5324)
// Kosten aller noch fehlenden Techniken bis einschliesslich `id`; -1 = fuer diese Kolonie nicht erreichbar.
pub fn wegKosten(welt: &mut dyn Welt, z: &Zustand, id: i32) -> i32 {
    let mut stapel = [0i32; 64];
    let mut gesehen = [0i32; 96];
    let mut ns = 0usize;
    let mut ng = 0usize;
    let mut summe = 0;
    stapel[ns] = id; ns += 1;
    while ns != 0 {
        ns -= 1;
        let t = stapel[ns];
        let mut schon = false;
        for k in 0..ng { if gesehen[k] == t { schon = true; break; } }
        if schon || hatTechId(welt, z, t) { continue; }
        if ng < 96 { gesehen[ng] = t; ng += 1; }
        let tt = match tb::tech(t) { Some(tt) => tt, None => return -1 };
        let k = eigeneKosten(welt, z, tt);
        if k < 0 { return -1; }
        summe += k;
        let idx = techIndex(welt, t);
        if idx < 0 { return -1; }
        let ti = techInfo(welt, idx);
        for r in 0..ti.required.len() {
            let rq = techInfo(welt, ti.required[r]);
            if ns < 64 { stapel[ns] = rq.id; ns += 1; }
        }
    }
    summe
}

/// Nimmt das Labor einen Auftrag an? Die Engine verlangt Arbeiter und Schaden unter 3/4 der Trefferpunkte
/// (IsEnabled 0x483710) -- ein Gebaeude im Aufbau oder ohne Roehre steht so lange still.
/// Im Aufbau (Aktion ObjWait/Develop) nimmt es nichts an, und ein DoStop bricht den Aufbau ab: das Gebaeude steht dann mit
/// dem erreichten Rest-HP als "fertig" da, der ConVec haengt im Baubefehl (Sim on6_03 s1: Grosslabor 80 HP, Reparatur danach).
pub fn laborBereit(welt: &mut dyn Welt, z: &Zustand, lab: Id, typ: i32) -> bool {
    let akt = welt.GetCurAction(lab);
    akt != moObjWait && akt != moDevelop && akt != moOperationalWait && !imAufbau(welt, z, lab, typ)
        && welt.HasPower(lab) && welt.HasWorkers(lab) && welt.GetDamage(lab) * 4 < welt.GetHitPoints(typ, z.spieler) * 3
}

/// Moral-Summe und -Teiler nach der Engine (MoraleManager::UpdatePlayerMorale 0x44E660; op2-sim colony.rs morale_target),
/// aus dem Lagebild geschaetzt. Werte aus morale.txt des Spiels (CROWDED_*, FOOD_*, DIS_BLD_*, *_UT_*, NURSERY_ON,
/// UNIV_ON, DIRT_*, UNOC_*, DIS_*_ON je 2). Moral = Ereignis + 100 x Summe / Teiler, mal Schwierigkeit.
pub fn moralSummeTeiler(welt: &mut dyn Welt, z: &Zustand, l: &Lage) -> (i32, i32) {
    let sp = z.spieler;
    let bew = l.menschen.max(1);
    let mut s = 0;
    // Ueberbelegung
    if l.wohnraum > 0 {
        let c = bew * 100 / l.wohnraum;
        s += if c <= 100 { 2 } else { 0 };
        if c > 100 { s -= 1; } if c > 133 { s -= 1; } if c > 166 { s -= 2; } if c > 200 { s -= 2; }
    }
    // Nahrung: Ueberschuss +2, Mangel mit grossem Vorrat +1, Mangel -2
    s += if l.nahrungNetto > 0 { 2 } else if bew * 15 <= l.nahrung { 1 } else { -2 };
    // Abgeschaltete Gebaeude (nicht stillgelegt, ohne Strom/Personal): keines +2, sonst -1
    let mut aus = 0;
    for &typ in GEBAEUDE.iter() {
        if typ == mapCommandCenter { continue; }
        let (a, w, st) = (welt.GetWorkersRequired(typ, sp), welt.GetScientistsRequired(typ, sp), welt.GetPowerRequired(typ, sp));
        for u in welt.PlayerBuildingEnum(sp, typ) {
            if !welt.IsLive(u) || welt.GetCurAction(u) != moDone || welt.IsBusy(u) { continue; }
            if (st > 0 && !welt.HasPower(u)) || (a > 0 && !welt.HasWorkers(u)) || (w > 0 && !welt.HasScientists(u)) { aus += 1; }
        }
    }
    s += if aus == 0 { 2 } else { -1 };
    // Auslastung Freizeit(+Forum) und Medizin: <=100 % drei Stufen, <=150 zwei, sonst eine
    let ut = |kap: i32| -> i32 { if kap <= 0 { 0 } else { let v = bew * 100 / kap; if v <= 100 { 3 } else if v <= 150 { 2 } else { 1 } } };
    s += ut(l.freizeitKap + l.forumKap) + ut(l.medKap);
    if l.geb[mapNursery as usize] > 0 { s += 2; }
    if l.geb[mapUniversity as usize] > 0 { s += 2; }
    if l.gorfLaeuft > 0 { s += 2; }
    // DIRT: Abdeckung als Naeherung des Schutzwerts (Stufen 1, 625, 1250, 1775 von 2500)
    if l.dirtAbdeckung > 0 {
        let schutz = 2500 * l.dirtAbdeckung.min(100) / 100;
        for lim in [1, 625, 1250, 1775] { if schutz >= lim { s += 1; } }
    }
    // Arbeitslose Erwachsene
    let erw = (bew - welt.Kids(sp)).max(1);
    let frei = (l.arbFrei + l.wissFrei).max(0) * 100 / erw;
    if frei >= 10 { s -= 1; } if frei >= 30 { s -= 2; } if frei >= 50 { s -= 3; }
    // Teiler: Grund 6, erforschte Kolonie-Techs, Warntechs (je +2 auf Summe und Teiler)
    let mut t = 6;
    let tech = |welt: &mut dyn Welt, typ: i32| -> bool { let id = techFuer(welt, typ); id > 0 && hatTech(welt, z, id) };
    if tech(welt, mapMedicalCenter) { t += 4; }
    if tech(welt, mapNursery) { t += 2; }
    if tech(welt, mapUniversity) { t += 2; }
    if tech(welt, mapGORF) { t += 2; }
    if tech(welt, mapRecreationFacility) { t += 4; }
    if tech(welt, mapDIRT) { t += 4; }
    for id in WARNTECHS { if hatTechId(welt, z, id) { s += 2; t += 2; } }
    (s, t.max(1))
}

/// Katastrophen-Warntechs (weapons.txt, Research Topic der Katastrophen-Zeilen): Erdbeben, Ausbruch, Blitz, Wirbelsturm, Meteor.
pub const WARNTECHS: [i32; 5] = [3201, 3202, 5302, 5303, 8317];

/// Nutzen einer Moralaenderung in "Bewohnern" (Waehrung von kolonieNutzen): Bewohner x PAR moralWert x Stufen-Gewinn.
/// Stufen nach morale.txt (M_PTS_TOT: 25, 45, 70, 90), dazwischen linear -- jede Stufe aendert Fabrik-Bauzeit (10 %),
/// Forschung (8) und Geburtenrate.
pub fn moralNutzen(z: &Zustand, l: &Lage, vorher: f64, nachher: f64) -> f64 {
    fn stufe(m: f64) -> f64 {
        const P: [(f64, f64); 5] = [(0.0, 0.0), (25.0, 1.0), (45.0, 2.0), (70.0, 3.0), (90.0, 4.0)];
        if m >= 90.0 { return 4.0; }
        for k in 1..5 { if m < P[k].0 { let (a, b) = (P[k - 1], P[k]); return a.1 + (m - a.0) / (b.0 - a.0); } }
        4.0
    }
    l.menschen.max(1) as f64 * PAR(z.spieler).moralWert * (stufe(nachher) - stufe(vorher))
}
