//! Hilfen fuer alle Bereiche: Grundflaechen, Namen, Meldungen (melde/selten), Lage erheben, Karte, Plaetze, Kampfwerte, Turmnetz.
//! C++-KI op2_ki.hpp Zeilen 375-1205, nach Rust uebertragen.
use super::*;

mod karte;
mod platz;
mod kraft;
pub use karte::*;
pub use platz::*;
pub use kraft::*;

/// C++: Groesse grundflaeche(int typ)  (op2_ki.hpp:377)
pub fn grundflaeche(_welt: &mut dyn Welt, typ: i32) -> Groesse {
    let (w, h) = match typ {
        mapCommandCenter => (3, 2),
        mapStructureFactory => (4, 3),
        mapVehicleFactory => (4, 3),
        mapTokamak => (2, 2),
        mapMHDGenerator => (2, 2),
        mapSolarPowerArray => (3, 2),
        mapCommonOreSmelter => (4, 3),
        mapRareOreSmelter => (4, 3),
        mapCommonOreMine => (2, 1),
        mapRareOreMine => (2, 1),
        mapCommonStorage => (1, 2),
        mapRareStorage => (1, 2),
        mapAgridome => (3, 2),
        mapResidence => (2, 2),
        mapReinforcedResidence => (3, 2),
        mapAdvancedResidence => (3, 3),
        mapBasicLab => (2, 2),
        mapStandardLab => (3, 2),
        mapAdvancedLab => (3, 3),
        mapMedicalCenter => (2, 2),
        mapNursery => (2, 2),
        mapUniversity => (2, 2),
        mapRecreationFacility => (2, 2),
        mapForum => (2, 2),
        mapDIRT => (3, 2),
        mapGarage => (3, 2),
        mapConsumerFactory => (3, 3),
        mapGuardPost => (1, 1),
        mapLightTower => (1, 1),
        mapGORF => (3, 2),
        mapRobotCommand => (2, 2),
        mapObservatory => (2, 2),
        mapMeteorDefense => (2, 2),
        mapTradeCenter => (2, 2),
        mapSpaceport => (5, 4),
        _ => (2, 2),
    };
    Groesse { w, h }
}

/// C++: const char* name(int typ)  (op2_ki.hpp:418)
pub fn name(_welt: &mut dyn Welt, typ: i32) -> &'static str {
    match typ {
        mapCommandCenter => "CommandCenter", mapStructureFactory => "StructureFactory",
        mapVehicleFactory => "VehicleFactory", mapTokamak => "Tokamak",
        mapMHDGenerator => "MHDGenerator", mapSolarPowerArray => "SolarPowerArray",
        mapCommonOreSmelter => "CommonOreSmelter", mapRareOreSmelter => "RareOreSmelter",
        mapCommonOreMine => "CommonOreMine", mapRareOreMine => "RareOreMine",
        mapCommonStorage => "CommonStorage", mapRareStorage => "RareStorage",
        mapAgridome => "Agridome", mapResidence => "Residence",
        mapReinforcedResidence => "ReinforcedResidence", mapAdvancedResidence => "AdvancedResidence",
        mapBasicLab => "BasicLab", mapStandardLab => "StandardLab",
        mapAdvancedLab => "AdvancedLab", mapMedicalCenter => "MedicalCenter",
        mapNursery => "Nursery", mapUniversity => "University",
        mapRecreationFacility => "RecreationFacility", mapForum => "Forum",
        mapDIRT => "DIRT", mapGarage => "Garage",
        mapConsumerFactory => "ConsumerFactory", mapGuardPost => "GuardPost",
        mapLightTower => "LightTower", mapGORF => "GORF",
        mapRobotCommand => "RobotCommand", mapObservatory => "Observatory",
        mapMeteorDefense => "MeteorDefense", mapTradeCenter => "TradeCenter",
        mapConVec => "ConVec", mapCargoTruck => "CargoTruck",
        mapRoboMiner => "RoboMiner", mapRoboSurveyor => "RoboSurveyor",
        mapRoboDozer => "RoboDozer", mapEarthworker => "Earthworker",
        mapScout => "Scout", mapRepairVehicle => "RepairVehicle",
        mapEvacuationTransport => "EvacuationTransport",
        mapLynx => "Lynx", mapPanther => "Panther",
        mapTiger => "Tiger", mapMiningBeacon => "MiningBeacon",
        mapSpaceport => "Spaceport", mapSULV => "SULV",
        mapEDWARDSatellite => "EDWARDSatellite", mapArachnidFactory => "ArachnidFactory",
        mapSpider => "Spider", mapScorpion => "Scorpion",
        mapEMPMissile => "EMPMissile",
        mapLaser => "Laser", mapMicrowave => "Microwave", mapRailGun => "RailGun",
        mapEMP => "EMP", mapRPG => "RPG", mapAcidCloud => "AcidCloud",
        mapESG => "ESG", mapThorsHammer => "ThorsHammer", mapStickyfoam => "Stickyfoam",
        mapStarflare => "Starflare", mapSupernova => "Supernova", mapEnergyCannon => "EnergyCannon",
        mapNone => "-",
        _ => "?",
    }
}

// Zelltypen (siehe mapview/celltypes.py)
/// C++: bool zelleBebaubar(int c)  (op2_ki.hpp:458)
pub fn zelleBebaubar(_welt: &mut dyn Welt, c: i32) -> bool { c == 0 || c == 2 || c == 3 || c == 4 || c == 5 || c == 7 || c == 21 }

/// C++: bool zelleRoehre(int c)  (op2_ki.hpp:459)
pub fn zelleRoehre(_welt: &mut dyn Welt, c: i32) -> bool { c >= 26 && c <= 31 }

/// C++: bool zelleBefahrbar(int c)  (op2_ki.hpp:460)
pub fn zelleBefahrbar(welt: &mut dyn Welt, c: i32) -> bool { zelleBebaubar(welt, c) || c == 22 || zelleRoehre(welt, c) }

/// C++: int eigeneKostenT(const Zustand& z, const tb::Tech& t)  (op2_ki.hpp:463)
pub fn eigeneKostenT(welt: &mut dyn Welt, z: &Zustand, t: &tb::Tech) -> i32 { if welt.IsEden(z.spieler) { t.eden } else { t.ply } }

// Techs fuer das Raumfahrtziel / techs for the space goal
pub const TECH_RAUMFAHRT: i32 = 5405;   // Space Program: Spaceport, SULV, EDWARD
pub const TECH_SELTENERZ: i32 = 5110;   // Rare Ore Processing: Seltenerz-Mine, -Schmelze, -Lager

// Eine Entscheidung mit Begruendung. `was`: bau, kit, fahrzeug, forschung, mine, roehre, warte, fehler, info
/// C++: void melde(const Zustand& z, const char* was, int typ, int x, int y, const char* fmt, ...)  (op2_ki.hpp:551)
pub fn melde(welt: &mut dyn Welt, z: &Zustand, was: &str, typ: i32, x: i32, y: i32, text: &str) {
    // char warum[400] + vsnprintf: hoechstens 399 Zeichen
    let mut ende = text.len().min(399);
    while !text.is_char_boundary(ende) { ende -= 1; }
    let warum = &text[..ende];
    let mut p = Puffer::default();
    p.f(&format!("{{\"t\":{},\"art\":\"entscheid\",\"p\":{},\"was\":", welt.Tick(), z.spieler));
    p.str(was);
    p.f(",\"typ\":"); p.str(name(welt, typ));
    p.f(&format!(",\"x\":{},\"y\":{},\"warum\":", x, y));
    p.str(warum);
    p.roh("}");
    p.schreiben(welt);
}

// Wiederkehrende Meldung hoechstens alle `abstand` Ticks (z.B. "warte auf Erz")
/// C++: bool selten(Zustand& z, int kanal, int abstand)  (op2_ki.hpp:567)
pub fn selten(welt: &mut dyn Welt, z: &mut Zustand, kanal: i32, abstand: i32) -> bool {
    let t = welt.Tick();
    if kanal < 0 || kanal >= 32 { return true; }
    let k = kanal as usize;
    if z.meldungenSeit[k] != 0 && t - z.meldungenSeit[k] < abstand { return false; }
    z.meldungenSeit[k] = t;
    true
}

// Alle Gebaeudetypen, die die KI zaehlt (PlayerBuildingEnum braucht einen Typ).
pub const GEBAEUDE: [i32; 38] = [
    mapCommandCenter, mapStructureFactory, mapVehicleFactory, mapTokamak, mapMHDGenerator, mapSolarPowerArray,
    mapCommonOreSmelter, mapRareOreSmelter, mapCommonOreMine, mapRareOreMine, mapCommonStorage, mapRareStorage,
    mapAgridome, mapResidence, mapReinforcedResidence, mapAdvancedResidence, mapBasicLab, mapStandardLab,
    mapAdvancedLab, mapMedicalCenter, mapNursery, mapUniversity, mapRecreationFacility, mapForum, mapDIRT,
    mapGarage, mapConsumerFactory, mapGuardPost, mapLightTower, mapGORF, mapRobotCommand, mapObservatory,
    mapMeteorDefense, mapTradeCenter, mapArachnidFactory, mapSpaceport, mapGeothermalPlant, mapMagmaWell,
];

// enum Mittel (M_ARB..M_SELTEN), struct Anspruch, MAX_ANSPR, MAX_VORHABEN, g_vorhaben: stehen in zustand.rs / globale.rs.

// Summe der Ansprueche auf ein Mittel mit mindestens diesem Rang. Das eigene Vorhaben (gleicher Typ) zaehlt nicht mit.
/// C++: int beansprucht(const Lage& l, int m, int abRang, int ohneVorhaben = -1)  (op2_ki.hpp:627)
pub fn beansprucht(_welt: &mut dyn Welt, l: &Lage, m: i32, abRang: i32, ohneVorhaben: i32) -> i32 {
    let mut s = 0;
    for i in 0..l.nAnspr as usize {
        let a = &l.anspr[i];
        if a.rang < abRang || (a.vorhaben && a.typ == ohneVorhaben) { continue; }
        s += a.menge[m as usize];
    }
    s
}

// Hoechster Rang, der ein Mittel beansprucht (-1 = keiner).
/// C++: int hoechsterAnspruch(const Lage& l, int m)  (op2_ki.hpp:637)
pub fn hoechsterAnspruch(_welt: &mut dyn Welt, l: &Lage, m: i32) -> i32 {
    let mut r = -1;
    for i in 0..l.nAnspr as usize {
        if l.anspr[i].menge[m as usize] > 0 && l.anspr[i].rang > r { r = l.anspr[i].rang; }
    }
    r
}

/// C++: void anspruchDazu(Lage& l, int typ, int rang, int arb, int wiss, int erz, int selten, bool vorhaben = false)  (op2_ki.hpp:642)
pub fn anspruchDazu(_welt: &mut dyn Welt, l: &mut Lage, typ: i32, rang: i32, arb: i32, wiss: i32, erz: i32, selten: i32, vorhaben: bool) {
    if l.nAnspr >= MAX_ANSPR as i32 || (arb <= 0 && wiss <= 0 && erz <= 0 && selten <= 0) { return; }
    let a = &mut l.anspr[l.nAnspr as usize];
    l.nAnspr += 1;
    a.typ = typ; a.rang = rang; a.vorhaben = vorhaben;
    a.menge[M_ARB] = if arb > 0 { arb } else { 0 };
    a.menge[M_WISS] = if wiss > 0 { wiss } else { 0 };
    a.menge[M_ERZ] = if erz > 0 { erz } else { 0 };
    a.menge[M_SELTEN] = if selten > 0 { selten } else { 0 };
}

/// C++: void erhebe(const Zustand& z, Lage& l)  (op2_ki.hpp:649)
pub fn erhebe(welt: &mut dyn Welt, z: &Zustand, l: &mut Lage) {
    l.geb = [0; 128];
    l.fz = [0; 128];
    l.kitsImSpiel = [0; 128];
    l.imBau = [0; 128];
    l.cc = 0; l.sf = 0; l.vf = 0; l.vfAnzahl = 0; l.vfLaeuft = 0; l.kitsFabrik = 0; l.nAnspr = 0; l.seltenSpart = false; l.erzSpart = false;
    let p = z.spieler;
    for &t in GEBAEUDE.iter() {
        for u in welt.PlayerBuildingEnum(p, t) {
            if t >= 0 && t < 128 { l.geb[t as usize] += 1; }
            if t >= 0 && t < 128 && welt.GetCurAction(u) == moObjWait { l.imBau[t as usize] += 1; }   // im Bau (Lauf 13 gemessen: Aktion 6)
            if t == mapCommandCenter && l.cc == 0 { l.cc = u; }
            if t == mapStructureFactory {
                if l.sf == 0 { l.sf = u; }
                for bay in 0..6 {
                    let k = welt.GetFactoryCargo(u, bay);
                    if k > 0 && k < 128 { l.kitsImSpiel[k as usize] += 1; l.kitsFabrik += 1; }
                }
            }
            if t == mapVehicleFactory && l.vf == 0 { l.vf = u; }
            if t == mapVehicleFactory && l.vfAnzahl < MAX_VF as i32 { l.vfs[l.vfAnzahl as usize] = u; l.vfAnzahl += 1; }
            if t == mapVehicleFactory && welt.HasPower(u) && welt.HasWorkers(u) { l.vfLaeuft += 1; }
        }
    }
    {
        for u in welt.PlayerVehicleEnum(p) {
            let t = welt.GetType(u);
            if t >= 0 && t < 128 { l.fz[t as usize] += 1; }
            if t == mapConVec {
                let k = welt.GetCargo(u);
                if k > 0 && k < 128 { l.kitsImSpiel[k as usize] += 1; }
            }
        }
    }
    l.erz = welt.Ore(p);
    l.erzMax = welt.GetMaxOre(p);
    l.nahrung = welt.FoodStored(p);
    l.nahrungNetto = welt.GetNetFoodProduction(p);
    l.stromFrei = welt.GetAmountPowerAvailable(p);
    l.menschen = welt.Kids(p) + welt.Workers(p) + welt.Scientists(p);
    l.wohnraum = welt.GetTotalResidenceCapacity(p);
        l.selten = welt.RareOre(p);
    l.seltenMax = welt.GetMaxRareOre(p);
    l.arbFrei = welt.GetNumAvailableWorkers(p);
    l.wissFrei = welt.GetNumAvailableScientists(p);
    l.arbStilllegbar = [0; 11];
    l.vorErz = 0; l.vorSelten = 0; l.vorFuer = "-";
    l.arbFehlt = 0; l.wissFehlt = 0;
    l.unbesetzt = [0; 128];
    l.medKap = welt.GetTotalMedCenterCapacity(p);
    l.freizeitKap = welt.GetTotalRecreationFacilityCapacity(p);
    l.forumKap = welt.GetTotalForumCapacity(p);
    // DIRT-Abdeckung und GORF selbst erheben (keine HFL-Abfrage): angeschlossen = alle Gebaeude ausser Tuermen, Minen, Kraftwerken
    {
        let mut dirt = 0; let mut angeschlossen = 0; l.gorfLaeuft = 0;
        for &t in GEBAEUDE.iter() {
            for u in welt.PlayerBuildingEnum(p, t) {
                if !welt.IsLive(u) { continue; }
                let laeuft = welt.HasPower(u) && (welt.HasWorkers(u) || welt.GetWorkersRequired(t, p) == 0);
                if t == mapDIRT && laeuft { dirt += 1; }
                if t == mapGORF && laeuft { l.gorfLaeuft = 1; }
                if t != mapGuardPost && t != mapCommonOreMine && t != mapRareOreMine && t != mapTokamak && t != mapLightTower && t != mapMagmaWell { angeschlossen += 1; }
            }
        }
        l.angeschlossen = angeschlossen;
        l.dirtAbdeckung = if angeschlossen > 0 { if dirt * 10 * 100 / angeschlossen > 100 { 100 } else { dirt * 10 * 100 / angeschlossen } } else { 100 };
    }
}

/// Gebaeude im Aufbau? Das Original meldet zur AIProc-Zeit dafuer Aktion 0 (= moDone,
/// gemessen an Mine und Grosslabor), der Rohbau sieht also aus wie ein fertiges, beschaedigtes Gebaeude. Erkennbar am
/// eigenen ConVec mit laufendem Baubefehl an der Bauecke unten rechts der Grundflaeche (Sim: Grosslabor 149..151 x 75..77,
/// ConVec auf 152,78), eine Kachel Spiel -- ein ConVec am Nachbargebaeude steht dort nicht (1 Kachel Abstand).
pub fn imAufbau(welt: &mut dyn Welt, z: &Zustand, b: Id, typ: i32) -> bool {
    let lo = welt.Location(b);
    let (bx, by) = (edX(welt, z, lo), edY(welt, z, lo));
    let r = rechteckUm(welt, typ, bx, by);
    let (ex, ey) = (r.x0 + r.w, r.y0 + r.h);
    for u in welt.PlayerVehicleEnum(z.spieler) {
        if welt.GetType(u) != mapConVec || !welt.IsBusy(u) || welt.GetLastCommand(u) != ctMoBuild { continue; }
        let l = welt.Location(u);
        let (ux, uy) = (edX(welt, z, l), edY(welt, z, l));
        if (ux - ex).abs() <= 1 && (uy - ey).abs() <= 1 { return true; }
    }
    false
}

thread_local! {
    /// angeschlossene Wachtuerme je Spieler: (Tick der Rechnung, Turm-Ids)
    static TURMNETZ: std::cell::RefCell<std::collections::HashMap<i32, (i32, Vec<Id>)>> = std::cell::RefCell::new(std::collections::HashMap::new());
}
pub fn zuruecksetzen_turmnetz() { TURMNETZ.with(|t| t.borrow_mut().clear()); }

/// Rust: haengt der Wachturm `u` im Roehrennetz einer Kommandozentrale seines Besitzers? Flutfuellung in Engine-Kacheln
/// von den Zentralen ueber Roehrenkacheln und Gebaeudeflaechen (4 Nachbarn), je Spieler alle 100 Ticks neu.
pub fn turmAngeschlossen(welt: &mut dyn Welt, u: Id) -> bool {
    let p = welt.OwnerID(u); let t = welt.Tick();
    if let Some(ja) = TURMNETZ.with(|m| m.borrow().get(&p).filter(|e| t - e.0 < 100).map(|e| e.1.contains(&u))) { return ja; }
    use std::collections::{HashMap, HashSet};
    let mut flaeche: HashMap<(i32, i32), bool> = HashMap::new();   // Kachel -> gehoert zu einer Zentrale
    let mut tuerme: Vec<(Id, i32, i32)> = Vec::new();
    for &typ in GEBAEUDE.iter() {
        for b in welt.PlayerBuildingEnum(p, typ) {
            if !welt.IsLive(b) { continue; }
            let l = welt.Location(b);
            if typ == mapGuardPost { tuerme.push((b, l.x, l.y)); }
            let r = rechteckUm(welt, typ, l.x, l.y);
            for y in r.y0..r.y0 + r.h { for x in r.x0..r.x0 + r.w { let e = flaeche.entry((x, y)).or_insert(false); if typ == mapCommandCenter { *e = true; } } }
        }
    }
    let mut netz: HashSet<(i32, i32)> = flaeche.iter().filter(|e| *e.1).map(|e| *e.0).collect();
    let mut q: Vec<(i32, i32)> = netz.iter().cloned().collect();
    let mut k = 0;
    while k < q.len() && netz.len() < 40000 {
        let (x, y) = q[k]; k += 1;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let n = (x + dx, y + dy);
            if netz.contains(&n) { continue; }
            let c = welt.GetCellType(Loc::new(n.0, n.1));
            if flaeche.contains_key(&n) || zelleRoehre(welt, c) { netz.insert(n); q.push(n); }
        }
    }
    let an: Vec<Id> = tuerme.iter().filter(|&&(_, x, y)| netz.contains(&(x, y))).map(|e| e.0).collect();
    let ja = an.contains(&u);
    TURMNETZ.with(|m| m.borrow_mut().insert(p, (t, an)));
    ja
}
