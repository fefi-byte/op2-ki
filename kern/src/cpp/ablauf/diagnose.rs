//! Diagnose-Spur: Lastwagen/Erzrouten, Stau an Schmelzen, Spur-Zeile "stand" je SPUR_TAKT.
use super::*;
use super::super::*;
use super::{WERTUNG_TICK, SCAN_TAKT, B_ANZAHL, GEBAEUDE, OreTypeCommon, Bar3, Bar2, VariantRandom, KI_MESSUNG};

/// C++: void lkwDiagnose(Zustand& z)  (op2_ki.hpp:6453)
// Lastwagen-Diagnose: alle 500 Ticks die Erzrouten (Mine, Schmelze, Lastwagen in der Gruppe) und jeder Lastwagen mit Ort,
// Aktion, Ladung, bisherigen Fahrten und Route als Zeile "art":"lkw". Truck diagnostics.
/// Lastwagen-Diagnose (Spur "lkw" alle 1000 Ticks): Routen und Lastwagen; Stau an einer Schmelze einmal melden.
/// Rust 2026-11-18 aufgeteilt (Ausgabe gleich).
pub fn lkwDiagnose(welt: &mut dyn Welt, z: &mut Zustand) {
    lkwSpur(welt, z);
    if let Some(sm) = stauErkennen(welt, z) { stauMelden(welt, z, sm); }
}

/// Spur "lkw": Erzrouten [Nr, Mine lebt, x, y, Schmelze lebt, x, y, Lastwagen] und alle Lastwagen
pub(super) fn lkwSpur(welt: &mut dyn Welt, z: &mut Zustand) {
    let mut p = Puffer::default();
    p.f(&format!("{{\"t\":{},\"art\":\"lkw\",\"p\":{},\"routen\":[", welt.Tick(), z.spieler));
    let mut erst = true;
    for i in 0..MAX_MINEN {
        if z.minenId[i] == 0 { continue; }
        let m: Id = z.minenId[i];
        let s: Id = z.schmelzeId[i];
        let ml = if welt.IsLive(m) { welt.Location(m) } else { Loc::new(0, 0) };
        let sl = if s != 0 && welt.IsLive(s) { welt.Location(s) } else { Loc::new(0, 0) };
        let mlebt = if welt.IsLive(m) { 1 } else { 0 };
        let slebt = if s != 0 && welt.IsLive(s) { 1 } else { 0 };
        let (mx, my, sx, sy) = (edX(welt, z, ml), edY(welt, z, ml), edX(welt, z, sl), edY(welt, z, sl));
        let tc = welt.TotalUnitCount(z.minenGruppe[i]);
        p.f(&format!("{}[{},{},{},{},{},{},{},{}]", if erst { "" } else { "," }, i, mlebt, mx, my, slebt, sx, sy, tc));
        erst = false;
    }
    p.roh("],\"lkw\":[");
    erst = true;
    for u in welt.PlayerVehicleEnum(z.spieler) {
        if welt.GetType(u) != mapCargoTruck || !welt.IsLive(u) { continue; }
        let mut route = -1;
        let mut i = 0;
        while i < MAX_MINEN && route < 0 {
            if z.minenId[i] != 0 {
                for v in welt.GroupEnumerator(z.minenGruppe[i]) { if v == u { route = i as i32; break; } }
            }
            i += 1;
        }
        let lo = welt.Location(u);
        // [unitID, x, y, Aktion, beschaeftigt, Ladungsart, Menge, Fahrten bisher, Route, Schaden]
        let (lx, ly) = (edX(welt, z, lo), edY(welt, z, lo));
        p.f(&format!("{}[{},{},{},{},{},{},{},{},{},{}]", if erst { "" } else { "," }, u, lx, ly, welt.GetCurAction(u),
            if welt.IsBusy(u) { 1 } else { 0 }, welt.GetCargoType(u), welt.GetCargoAmount(u), welt.GetNumTruckLoadsSoFar(u), route, welt.GetDamage(u)));
        erst = false;
    }
    p.roh("]}");
    p.schreiben(welt);
}

/// Stau erkennen: beladener Lastwagen einer Route steht bei 3 Messungen (3000 Ticks) am selben Fleck nahe seiner Schmelze.
/// Liefert die Schmelze, einmal je Stau.
pub(super) fn stauErkennen(welt: &mut dyn Welt, z: &mut Zustand) -> Option<Id> {
    let sp = (z.spieler & 7) as usize;
    let mut neuId = [0i32; 16];
    let mut neuX = [0i32; 16];
    let mut neuY = [0i32; 16];
    let mut neuZ = [0i32; 16];
    let mut nn = 0usize;
    let mut stauSchmelze = -1;
    let st = LKW_STATIK.with(|s| s.borrow().clone());
    for v in welt.PlayerVehicleEnum(z.spieler) {
        if nn >= 16 { break; }
        if welt.GetType(v) != mapCargoTruck || !welt.IsLive(v) || welt.GetCargoAmount(v) <= 0 { continue; }
        let mut route: i32 = -1;
        let mut i = 0;
        while i < MAX_MINEN && route < 0 {
            if z.minenId[i] != 0 && z.schmelzeId[i] != 0 {
                for w in welt.GroupEnumerator(z.minenGruppe[i]) { if w == v { route = i as i32; break; } }
            }
            i += 1;
        }
        if route < 0 { continue; }
        let sm: Id = z.schmelzeId[route as usize];
        if !welt.IsLive(sm) { continue; }
        let vl = welt.Location(v);
        let sl = welt.Location(sm);
        if abstand2(welt, vl.x, vl.y, sl.x, sl.y) > 6 * 6 { continue; }
        let mut zz = 1;
        for k in 0..16 { if st.altId[sp][k] == v && st.altX[sp][k] == vl.x && st.altY[sp][k] == vl.y { zz = st.zaehler[sp][k] + 1; } }
        neuId[nn] = v; neuX[nn] = vl.x; neuY[nn] = vl.y; neuZ[nn] = zz; nn += 1;
        if zz >= 3 { stauSchmelze = z.schmelzeId[route as usize]; }
    }
    let gemeldet = LKW_STATIK.with(|s| {
        let mut s = s.borrow_mut();
        for k in 0..16 {
            s.altId[sp][k] = if k < nn { neuId[k] } else { 0 };
            s.altX[sp][k] = if k < nn { neuX[k] } else { 0 };
            s.altY[sp][k] = if k < nn { neuY[k] } else { 0 };
            s.zaehler[sp][k] = if k < nn { neuZ[k] } else { 0 };
        }
        if stauSchmelze < 0 { s.gemeldet[sp] = 0; return None; }
        if s.gemeldet[sp] != 0 { return None; }
        s.gemeldet[sp] = 1;
        Some(())
    });
    if gemeldet.is_none() { return None; }
    Some(stauSchmelze)
}

/// Stau melden: was auf der Anlegestelle steht, Zustand der Schmelze; Ansicht dorthin (fuer ein Bildschirmfoto)
pub(super) fn stauMelden(welt: &mut dyn Welt, z: &mut Zustand, sm: Id) {
    let sl = welt.Location(sm);
    let dl = welt.GetDockLocation(sm);
    let mut q = Puffer::default();
    let (sx, sy, dx_, dy_) = (edX(welt, z, sl), edY(welt, z, sl), edX(welt, z, dl), edY(welt, z, dl));
    q.f(&format!("{{\"t\":{},\"art\":\"lkwstau\",\"p\":{},\"schmelze\":[{},{}],\"dock\":[{},{}],\"zustand\":[{},{},{},{},{}],\"aufDock\":[",
        welt.Tick(), z.spieler, sx, sy, dx_, dy_, if welt.IsBusy(sm) { 1 } else { 0 }, welt.GetCurAction(sm),
        if welt.HasPower(sm) { 1 } else { 0 }, if welt.HasWorkers(sm) { 1 } else { 0 }, welt.GetDamage(sm)));
    // alles, was auf der Anlegestelle und den Kacheln daneben steht: [x, y, Typ, Besitzer, Aktion]
    let mut erst2 = true;
    for dy in -1..=1 {
        for dx in -1..=1 {
            for o in welt.LocationEnumerator(Loc::new(dl.x + dx, dl.y + dy)) {
                let ol = welt.Location(o);
                let (ox, oy) = (edX(welt, z, ol), edY(welt, z, ol));
                q.f(&format!("{}[{},{},{},{},{}]", if erst2 { "" } else { "," }, ox, oy, welt.GetType(o), welt.OwnerID(o), welt.GetCurAction(o)));
                erst2 = false;
            }
        }
    }
    q.f(&format!("],\"zelleDock\":{}}}", welt.GetCellType(dl)));
    q.schreiben(welt);
    let lp = welt.LocalPlayer();
    welt.CenterViewOn(lp, dl.x, dl.y);
}

/// C++: void spur(const Zustand& z, const Lage& l)  (op2_ki.hpp:6533)
/// Spur-Zeile "stand" (JSON) je Takt: Vorrat, Bevoelkerung, Kuchen, Kampf, inaktive Gebaeude, alle Gebaeude/Fahrzeuge,
/// Plaetze und Roehren. Rust 2026-11-18 in Abschnitte aufgeteilt (Ausgabe gleich).
pub fn spur(welt: &mut dyn Welt, z: &Zustand, l: &Lage) {
    let mut p = Puffer::default();
    spurStand(welt, z, l, &mut p);
    // Unbekannte Felder der Spielerstruktur (HFL OP2Player) -- Messung, um den genauen Moralwert zu finden.
    hinweis(welt, z, "ki::spur/1");
    // FEHLT: Rohspeicher der Spielerstruktur (isInited && playerArray, ",\"roh\":[...]") -- in Welt nicht verfuegbar, entfaellt
    // Tafel: Bau-Kuchen, Vormerkung und Stand der Teilziele (ok / bau / frei = baubar / Tech = Technik fehlt)
    hinweis(welt, z, "ki::spur/2");
    spurTafel(welt, z, l, &mut p);
    // Kontrolle: fertige Gebaeude, die nicht laufen (ohne Arbeiter, Wissenschaftler oder Strom), ausser den
    // bewusst stillgelegten. Check: finished buildings that are not running, except deliberately idled ones.
    hinweis(welt, z, "ki::spur/3");
    spurInaktiv(welt, z, l, &mut p);
    p.roh(",\"geb\":[");
    hinweis(welt, z, "ki::spur/4");
    spurGebaeude(welt, z, l, &mut p);
    p.roh("],\"fz\":[");
    hinweis(welt, z, "ki::spur/5");
    spurFahrzeuge(welt, z, l, &mut p);
    p.roh("],\"plaetze\":[");
    hinweis(welt, z, "ki::spur/6");
    spurPlaetzeRoehren(welt, z, l, &mut p);
    p.roh("]}");
    p.schreiben(welt);
    if std::env::var("OP2KI_ARMEE_SPUR").is_ok() { spurArmee(welt, z); }
}

/// Vorrat, Strom, Bevoelkerung, Ziel, Arbeit, DIRT
pub(super) fn spurStand(welt: &mut dyn Welt, z: &Zustand, l: &Lage, p: &mut Puffer) {
    let pl = z.spieler;
    p.f(&format!("{{\"t\":{},\"art\":\"stand\",\"p\":{},\"erz\":{},\"erzMax\":{},\"selten\":{},\"nahrung\":{},\"nahrungNetto\":{},\
\"strom\":[{},{}],\"kinder\":{},\"arbeiter\":{},\"wiss\":{},\"wohnraum\":{},\"moral\":{}",
        welt.Tick(), pl, l.erz, l.erzMax, welt.RareOre(pl), l.nahrung, l.nahrungNetto,
        welt.GetAmountPowerGenerated(pl), welt.GetAmountPowerConsumed(pl),
        welt.Kids(pl), welt.Workers(pl), welt.Scientists(pl), l.wohnraum, welt.MoraleLevel(pl)));
    { let nw = netzWert(welt, z, l); if nw > -1e8 { p.f(&format!(",\"netzWert\":{:.0}", nw)); } }   // Wertnetz (falls geladen)
    p.f(&format!(",\"ziel\":{},\"zielErreicht\":{},\"raumSchritt\":{},\"satelliten\":{},\"seltenMax\":{}",
        z.ziel, z.zielErreicht, z.raumSchritt, welt.GetSatelliteCount(pl, mapEDWARDSatellite), l.seltenMax));
    // Bevoelkerung und Arbeit: frei/benoetigt, stillgelegte/aktive Gebaeude, Kapazitaeten (Medizin, Freizeit, Forum)
    p.f(&format!(",\"arbFrei\":{},\"arbBedarf\":{},\"wissFrei\":{},\"wissBedarf\":{},\"wissForschung\":{},\
\"stillgelegt\":{},\"aktiv\":{},\"ohneStrom\":{},\"kap\":[{},{},{}]",
        welt.GetNumAvailableWorkers(pl), welt.GetNumWorkersRequired(pl),
        welt.GetNumAvailableScientists(pl), welt.GetNumScientistsRequired(pl),
        welt.GetNumScientistsAssignedToResearch(pl),
        welt.GetNumIdleBuildings(pl), welt.GetNumActiveBuildings(pl), welt.GetNumUnpoweredStructures(pl),
        welt.GetTotalMedCenterCapacity(pl), welt.GetTotalRecreationFacilityCapacity(pl),
        welt.GetTotalForumCapacity(pl)));
    p.f(&format!(",\"dirt\":[{},{},{}]", l.dirtAbdeckung, l.gorfLaeuft, l.angeschlossen));   // DIRT-Abdeckung %, GORF in Betrieb, angeschlossene Gebaeude
}

/// Bau-Kuchen, Vormerkung, Bilanz, Lernen, Dringlichkeit, Kampf und Tafel der Teilziele
pub(super) fn spurTafel(welt: &mut dyn Welt, z: &Zustand, l: &Lage, p: &mut Puffer) {
    let pl = z.spieler;
    let mut anteil = [0i32; B_ANZAHL as usize];
    bauKuchen(welt, z, l, &mut anteil);
    p.f(&format!(",\"kuchen\":{{\"Kolonie\":{},\"Ziel\":{},\"Wirtschaft\":{},\"Verteidigung\":{}}},\"vorgemerkt\":[{},{},\"{}\"]",
        anteil[0], anteil[1], anteil[2], anteil[3], l.vorErz, l.vorSelten, l.vorFuer));
    // Kampf: [Feind Armee, davon nahe, Feind-Gebaeude, eigener Wert, Soll, Bedrohung, Angriffe, Wache]
    // dazu: groesste Feindarmee, mobil eigen/soll, Modus (0 Wache 1 Abfangen 2 Angriff), Aussenposten mit Gruppe
    let mut aussen = 0;
    for i in 0..6 { if z.aussenAn[i] != 0 { aussen += 1; } }
    {
        let bl = bilanz(welt, z, l);
        // Bilanz: [Bedarf Erz, Bedarf Seltenerz, Engpass Erz, Engpass Seltenerz, Zufluss Erz, Zufluss Seltenerz je 1000 Ticks]
        p.f(&format!(",\"bilanz\":[{},{},{},{},{},{}]", bl.bedarfErz, bl.bedarfSelten, bl.engpassErz, bl.engpassSelten,
            z.zuflussErz as i32, z.zuflussSelten as i32));
        let sv = (100.0 * seltenVorteil(welt, z)) as i32;
        p.f(&format!(",\"seltenVorteil\":{},\"seltenSpart\":{}", sv, if l.seltenSpart { 1 } else { 0 }));   // % mehr Kampfkraft je Erz
    }
    {
        let d = dringlichkeit(welt, z, l);
        p.f(&format!(",\"potenzial\":{}", z.feindPotenzial));
        let lp = laufPunkte(welt, z);
        p.f(&format!(",\"lernen\":{{\"lauf\":{},\"reserve\":{:.2},\"unterwegs\":{},\"punkte\":{},\"zerstoert\":{},\"verloren\":{}}}", z.laufNr, z.reserve,
            z.feindUnterwegs, lp, z.zerstoertWert, z.verlorenWert));
        p.f(&format!(",\"dringlich\":{{\"Kolonie\":{},\"Ziel\":{},\"Wirtschaft\":{},\"Verteidigung\":{},\"Militaerforschung\":{},\"Wellenabstand\":{},\"warumKolonie\":\"{}\",\"warumWirtschaft\":\"{}\"}}",
            d.kolonie, d.ziel, d.wirtschaft, d.verteidigung, d.kampfForschung, z.wellenAbstand, d.kolonieGrund, d.wirtschaftGrund));
    }
    let ms = mobilSoll(welt, z, true);
    p.f(&format!(",\"kampf\":[{},{},{},{},{},{},{},{},{},{},{},{},{}]", z.feindArmee, z.feindNah, z.feindGeb, z.eigeneArmee, z.sollArmee,
        z.bedrohung, z.angriffe, z.wacheAnzahl, z.staerksterAngriff, z.eigeneMobil, ms, z.kampfModus, aussen));
    const TEILE: [i32; 9] = [mapNursery, mapUniversity, mapMedicalCenter, mapStandardLab, mapAdvancedLab,
                             mapVehicleFactory, mapRareOreSmelter, mapRareOreMine, mapSpaceport];
    p.roh(",\"tafel\":{");
    let mut erst = true;
    for t in TEILE {
        let tu = t as usize;
        let st = if l.geb[tu] > l.imBau[tu] { "ok" }
                 else if l.imBau[tu] != 0 || geplant(welt, z, t) != 0 || l.kitsImSpiel[tu] != 0 { "bau" }
                 else if kitMoeglich(welt, z, t) { "frei" } else { "Tech" };
        let nm = name(welt, t);
        p.f(&format!("{}\"{}\":\"{}\"", if erst { "" } else { "," }, nm, st));
        erst = false;
    }
    p.f(&format!(",\"Satellit\":\"{}\"}}", if z.zielErreicht != 0 { "ok" } else if z.ziel == 1 { "offen" } else { "-" }));
}

/// Fertige Gebaeude, die nicht laufen (ohne Arbeiter, Wissenschaftler oder Strom)
pub(super) fn spurInaktiv(welt: &mut dyn Welt, z: &Zustand, l: &Lage, p: &mut Puffer) {
    let pl = z.spieler;
    p.roh(",\"inaktiv\":[");
    let mut erst = true;
    for &t in GEBAEUDE {
        let arbN = welt.GetWorkersRequired(t, pl);
        let wissN = welt.GetScientistsRequired(t, pl);
        for u in welt.PlayerBuildingEnum(pl, t) {
            if !welt.IsLive(u) { continue; }
            if welt.GetCurAction(u) != moDone || welt.IsBusy(u) { continue; }
            let grund = if arbN > 0 && !welt.HasWorkers(u) { Some("Arbeiter") }
                        else if wissN > 0 && !welt.HasScientists(u) { Some("Wissenschaftler") }
                        else if welt.GetPowerRequired(t, pl) > 0 && !welt.HasPower(u) { Some("Strom") } else { None };
            let grund = match grund { Some(g) => g, None => continue };
            let nm = name(welt, t);
            p.f(&format!("{}[\"{}\",\"{}\"]", if erst { "" } else { "," }, nm, grund));
            erst = false;
        }
    }
    p.roh("]");
}

/// Alle Gebaeude (hoechstens 120): Name, Ort, beschaeftigt, Aktion, Strom, Arbeiter, Id
pub(super) fn spurGebaeude(welt: &mut dyn Welt, z: &Zustand, l: &Lage, p: &mut Puffer) {
    let pl = z.spieler;
    let mut erst = true;
    let mut k = 0;
    for &t in GEBAEUDE {
        for u in welt.PlayerBuildingEnum(pl, t) {
            let weiter = k < 120; k += 1;
            if !weiter { break; }
            let lo = welt.Location(u);
            // [Name, x, y, beschaeftigt, Aktion, Strom, Arbeiter, unitID] -- fertig, angeschlossen, besetzt?
            let nm = name(welt, t);
            let (lx, ly) = (edX(welt, z, lo), edY(welt, z, lo));
            p.f(&format!("{}[\"{}\",{},{},{},{},{},{},{}]", if erst { "" } else { "," }, nm, lx, ly, if welt.IsBusy(u) { 1 } else { 0 },
                welt.GetCurAction(u), if welt.HasPower(u) { 1 } else { 0 }, if welt.HasWorkers(u) { 1 } else { 0 }, u));
            erst = false;
        }
    }
}

/// Alle Fahrzeuge (hoechstens 120): Name, Ort, Ladung, Aktion, Id
pub(super) fn spurFahrzeuge(welt: &mut dyn Welt, z: &Zustand, l: &Lage, p: &mut Puffer) {
    let pl = z.spieler;
    let mut erst = true;
    let mut k = 0;
    for u in welt.PlayerVehicleEnum(pl) {
        let weiter = k < 120; k += 1;
        if !weiter { break; }
        // Zerstoerte Fahrzeuge koennen noch in der Liste stehen (Duell 13: 3x Absturz 00475EF5 hier, immer im Gefecht)
        if !welt.IsLive(u) { continue; }
        let lo = welt.Location(u);
        let ty = welt.GetType(u);
        let ca = welt.GetCargo(u);
        let (n1, n2) = (name(welt, ty), name(welt, ca));
        let (lx, ly) = (edX(welt, z, lo), edY(welt, z, lo));
        p.f(&format!("{}[\"{}\",{},{},\"{}\",{},{}]", if erst { "" } else { "," }, n1, lx, ly,
            n2, welt.GetCurAction(u), u));
        erst = false;
    }
}

/// Bauplaetze und Roehrenauftraege
pub(super) fn spurPlaetzeRoehren(welt: &mut dyn Welt, z: &Zustand, l: &Lage, p: &mut Puffer) {
    let pl = z.spieler;
    for i in 0..z.anzahlPlaetze as usize {
        let q = z.plaetze[i];
        let g = grundflaeche(welt, q.typ as i32);
        let nm = name(welt, q.typ as i32);
        p.f(&format!("{}[\"{}\",{},{},{},{},{}]", if i != 0 { "," } else { "" }, nm, q.x0, q.y0, g.w, g.h, q.zustand as i32));
    }
    p.roh("],\"roehren\":[");
    hinweis(welt, z, "ki::spur/7");
    for i in 0..z.anzahlRoehren as usize {
        let r = z.roehren[i];
        p.f(&format!("{}[{},{},{},{},{}]", if i != 0 { "," } else { "" }, r.x1, r.y1, r.x2, r.y2, r.zustand as i32));
    }
}

// ---------------------------------------------------------------------------------------------------
// Oeffentliche Schnittstelle / public interface

/// Nur mit OP2KI_ARMEE_SPUR: Spur-Zeile "armee" -- Kampffahrzeuge je Fahrgestell/Waffe (GetWeapon; GetCargo liefert bei
/// Kampffahrzeugen keine Waffe)
fn spurArmee(welt: &mut dyn Welt, z: &Zustand) {
    let mut n: Vec<(String, i32)> = Vec::new();
    for u in welt.PlayerVehicleEnum(z.spieler) {
        if !welt.IsLive(u) || kampfWert(welt, u) <= 0 { continue; }
        let (ty, wf) = (welt.GetType(u), welt.GetWeapon(u));
        let k = format!("{}/{}", name(welt, ty), name(welt, wf));
        match n.iter_mut().find(|e| e.0 == k) { Some(e) => e.1 += 1, None => n.push((k, 1)) }
    }
    let txt: Vec<String> = n.iter().map(|e| format!("\"{}\":{}", e.0, e.1)).collect();
    // Labore: [Typ, Thema (Tech-ID, 0 = keins), Wissenschaftler]
    let mut lab: Vec<String> = Vec::new();
    for &ty in [mapBasicLab, mapStandardLab, mapAdvancedLab].iter() {
        for b in welt.PlayerBuildingEnum(z.spieler, ty) {
            if !welt.IsLive(b) { continue; }
            let th = welt.GetLabCurrentTopic(b);
            let id = if th > 0 { let i = if th < welt.GetTechCount() { th } else { super::super::forschung::techIndex(welt, th) }; if i >= 0 { super::super::forschung::techInfo(welt, i).id } else { th } } else { 0 };
            lab.push(format!("[\"{}\",{},{}]", name(welt, ty), id, welt.GetLabScientistCount(b)));
        }
    }
    let mut p = Puffer::default();
    p.roh(&format!("{{\"t\":{},\"art\":\"armee\",\"p\":{},\"n\":{{{}}},\"labore\":[{}]}}", welt.Tick(), z.spieler, txt.join(","), lab.join(",")));
    p.schreiben(welt);
}
