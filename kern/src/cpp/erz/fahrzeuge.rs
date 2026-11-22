//! Fahrzeugproduktion: was die Fahrzeugfabrik baut (fahrzeugWahl, fahrzeuge), weitere Fabriken, sichere Gruppenwechsel.
use super::*;
use super::super::*;
use super::{TECH_SELTENERZ, WERT_LYNX, GEBAEUDE, ix, exy};

thread_local! { static LKW_LETZT: std::cell::RefCell<[i32; 8]> = const { std::cell::RefCell::new([-100000; 8]) }; }

pub(super) struct K { typ: i32, wert: i32, warum: &'static str }

// Lambda `pruefe` aus fahrzeugWahl: was Seltenerz kostet, das gerade fehlt, kommt nicht in Frage.
pub(super) fn fzPruefe(welt: &mut dyn Welt, z: &Zustand, l: &Lage, beste: &mut K, typ: i32, wert: i32, w: &'static str) {
    if wert <= beste.wert { return; }
    if welt.GetRareOreCost(typ, z.spieler) > l.selten { return; }
    *beste = K { typ, wert, warum: w };
}

/// C++: int fahrzeugWahl(const Zustand& z, const Lage& l, const char** warum)  (op2_ki.hpp:3714)
// Fahrzeuge nach Dringlichkeit: jeder Typ bekommt einen Wert aus offenen Aufgaben, gebaut wird der hoechste.
pub fn fahrzeugWahl(welt: &mut dyn Welt, z: &Zustand, l: &Lage, warum: &mut &'static str) -> i32 {
    let mut beste = K { typ: mapNone, wert: 0, warum: "" };
    let mut offen = 0;
    for i in 0..z.anzahlPlaetze as usize { if z.plaetze[i].zustand == P_FAHRT || z.plaetze[i].zustand == P_BAUT { offen += 1; } }
    // ConVecs
    if l.fz[ix(mapConVec)] == 0 { fzPruefe(welt, z, l, &mut beste, mapConVec, 100, "kein ConVec -- ohne ihn wird nichts gebaut"); }
    else if l.fz[ix(mapConVec)] < 2 { fzPruefe(welt, z, l, &mut beste, mapConVec, 60, "zweiter ConVec: einer holt Bausaetze, einer baut"); }
    else if l.fz[ix(mapConVec)] < 3 && offen > 0 && z.erzUeberschuss != 0 { fzPruefe(welt, z, l, &mut beste, mapConVec, 20, "dritter ConVec: Bauten stehen an, Erz reicht"); }
    // Earthworker: Roehren (BuildingGroup) und Mauern
    if l.fz[ix(mapEarthworker)] == 0 { fzPruefe(welt, z, l, &mut beste, mapEarthworker, 70, "Roehren und Mauern brauchen einen Earthworker"); }
    // Lastwagen: zwei je Erz- und Seltenerz-Route; dringender, je groesser der Engpass
    {
        let bl = bilanz(welt, z, l);
        let gs = l.geb[ix(mapCommonOreSmelter)];
        let gm = l.geb[ix(mapCommonOreMine)];
        let routen = (if gs < 2 * gm { gs } else { 2 * gm })
            + (if l.geb[ix(mapRareOreSmelter)] > 0 { l.geb[ix(mapRareOreMine)] } else { 0 });
        // Soll je angelegter Erzroute nach Entfernung, sonst zwei
        let mut soll = 0;
        let mut angelegt = 0;
        for i in 0..MAX_MINEN { if z.minenId[i] != 0 { soll += lastwagenSoll(welt, z, i as i32); angelegt += 1; } }
        soll += 2 * (if routen > angelegt { routen - angelegt } else { 0 });
        let fehlen = soll - l.fz[ix(mapCargoTruck)];
        if fehlen > 0 {
            let mut eng = 0;
            if bl.bedarfErz > 0 && bl.engpassErz > 0 { eng = bl.engpassErz * 100 / bl.bedarfErz; }
            if bl.bedarfSelten > 0 && bl.engpassSelten > 0 && bl.engpassSelten * 100 / bl.bedarfSelten > eng { eng = bl.engpassSelten * 100 / bl.bedarfSelten; }
            // Ohne Engpass reicht ein Lastwagen je Route -- weitere erst, wenn Erz fehlt
            if eng > 0 || l.fz[ix(mapCargoTruck)] < routen {
                fzPruefe(welt, z, l, &mut beste, mapCargoTruck, 40 + (if eng > 100 { 100 } else { eng }) / 2, "Lastwagen fehlen auf den Erzrouten");
            }
        }
        // Neue gewoehnliche Mine: Schmelzen-Kapazitaet der Minen voll und Erz fehlt
        if l.fz[ix(mapRoboMiner)] == 0 && gm < MAX_MINEN as i32 && (gs as f64) >= PAR(z.spieler).schmelzenJeMine * gm as f64
            && bl.engpassErz > 0 && bl.bedarfErz > 0 {
            let q = bl.engpassErz * 100 / bl.bedarfErz;
            fzPruefe(welt, z, l, &mut beste, mapRoboMiner, 30 + (if q > 100 { 100 } else { q }) / 2,
                     "neue Mine -- Bilanz: Erz fehlt, Minen ausgelastet");
        }
        // Seltenerz-Mine, sobald die Seltenerz-Schmelze steht
        if l.geb[ix(mapRareOreSmelter)] > 0 && l.geb[ix(mapRareOreMine)] == 0 && l.fz[ix(mapRoboMiner)] == 0 {
            let w = if PAR(z.spieler).seltenNutzen >= 0.5 { 50 + ((seltenVorteil(welt, z) * 50.0) as i32).clamp(0, 45) } else { 50 };
            fzPruefe(welt, z, l, &mut beste, mapRoboMiner, w, "Seltenerz-Mine fuer die Seltenerz-Schmelze");
        }
    }
    if l.fz[ix(mapRoboSurveyor)] == 0 && l.geb[ix(mapCommonOreMine)] < MAX_MINEN as i32 { fzPruefe(welt, z, l, &mut beste, mapRoboSurveyor, 15, "Baken vermessen"); }
    // Missionsmodus, Rolle Kolonie: ein zweiter Vermesser erkundet die Baken der Umgebung
    else if l.fz[ix(mapRoboSurveyor)] < 2 && super::super::modus::rolle(z) == super::super::modus::ROLLE_KOLONIE { fzPruefe(welt, z, l, &mut beste, mapRoboSurveyor, 30, "Kolonie: zweiter Vermesser erkundet die Baken"); }
    // Kampffahrzeuge: so dringend wie die Verteidigung (oder die Aussenposten/Angriffstruppe)
    {
        let fz = fahrgestellWahl(welt, z, l.selten);
        let zahl = l.fz[ix(mapLynx)] + l.fz[ix(mapPanther)] + l.fz[ix(mapTiger)];
        let fehlt = kampfFehlt(welt, z, l);
        let kfz = if istKampfFz(welt, z.fzTyp) { WERT_LYNX } else { 0 };
        let mob = mobilSoll(welt, z, true) - z.eigeneMobil - kfz;
        let _ = zahl;
        if fz != 0 && (fehlt > 0 || mob > 0) {
            let mut wert = 0;
            if fehlt > 0 {
                let s = if z.sollArmee > 0 { z.sollArmee } else { fehlt };
                let q = fehlt * 100 / s;
                wert = z.bedrohung * (if q > 100 { 100 } else { q }) / 100;
            }
            if mob > 0 && wert < 25 { wert = 25; }
            let ms = mobilSoll(welt, z, true);
            let t = statischerText(&format!("Kampffahrzeug: Feind Wert {} (+{:.1} je 1000 Ticks), eigener {}, Soll {}, mobil {}/{}",
                                            z.feindArmee, z.feindWachstum, z.eigeneArmee, z.sollArmee, z.eigeneMobil, ms));
            fzPruefe(welt, z, l, &mut beste, fz, wert, t);
        }
    }
    // Reparatur nach Erwartung: gebraucht, sobald ein Feind bekannt ist oder etwas beschaedigt ist;
    // ein weiteres je 15 Kampffahrzeuge.
    if hatTech(welt, z, 5051) {
        let mut schaden = 0;
        let mut kampfFz = 0;
        for &ty in GEBAEUDE {
            for b in welt.PlayerBuildingEnum(z.spieler, ty) {
                let a2 = welt.GetCurAction(b);
                if a2 != moObjWait && a2 != moDevelop && welt.GetDamage(b) > 0 { schaden += 1; }
            }
        }
        {
            for v in welt.PlayerVehicleEnum(z.spieler) {
                if welt.IsLive(v) && kaempfer(welt, v) && welt.GetType(v) != mapScorpion {
                    kampfFz += 1;
                    let hp = welt.GetHitPoints(welt.GetType(v), z.spieler);
                    if hp > 0 && welt.GetDamage(v) * 100 / hp > 50 { schaden += 1; }
                }
            }
        }
        let rep = l.fz[ix(mapRepairVehicle)] + l.fz[ix(mapSpider)];
        // Neues Modell: diese Wahl gilt der Fahrzeugfabrik -- Spinnen baut nur die Arachnid-Fabrik (Spielregel; die Engine
        // prueft es nicht und baut sie sonst in der Fahrzeugfabrik). Plymouth repariert dann ueber ConVec und Garage.
        let spinneHier = !kampfModellNeu();
        let rtOk = reparaturTyp(welt, z) != mapSpider || spinneHier;
        if rtOk && rep == 0 && (schaden > 0 || z.feindGemeldet != 0 || z.bedrohung > 0) {
            let rt = reparaturTyp(welt, z);
            fzPruefe(welt, z, l, &mut beste, rt, 50, "Reparaturfahrzeug: Feind bekannt oder etwas beschaedigt");
        } else if rtOk && rep > 0 && rep < 1 + kampfFz / 15 {
            let rt = reparaturTyp(welt, z);
            fzPruefe(welt, z, l, &mut beste, rt, 30, "weiteres Reparaturfahrzeug: je 15 Kampffahrzeuge eins");
        }
        // Spinnen fuer die Uebernahme: nur Plymouth, nur wenn EMP im Einsatz ist (Truppe oder Rakete)
        if spinneHier && !welt.IsEden(z.spieler) && (z.zielEmp > 0 || z.empRakete != 0)
            && l.fz[ix(mapSpider)] < 1 + (if z.zielEmp > 0 { z.zielEmp } else { 2 }) / 2 + 1 {
            fzPruefe(welt, z, l, &mut beste, mapSpider, 35, "Spinnen: uebernehmen betaeubte Feindfahrzeuge");
        }
    }
    *warum = beste.warum;
    beste.typ
}

/// C++: int fahrzeugBedarf(const Zustand& z, const Lage& l, const char** warum)  (op2_ki.hpp:3807)
pub fn fahrzeugBedarf(welt: &mut dyn Welt, z: &Zustand, l: &Lage, warum: &mut &'static str) -> i32 {
    // Dringlichkeit des wichtigsten Fahrzeugs (fuer die Wirtschafts-Dringlichkeit, wenn die Fabrik fehlt)
    let mut w: &'static str = "";
    let typ = fahrzeugWahl(welt, z, l, &mut w);
    *warum = w;
    if typ == mapNone { return 0; }
    if typ == mapConVec && l.fz[ix(mapConVec)] == 0 { return 100; }
    if typ == mapCargoTruck || typ == mapRoboMiner { 60 } else if istKampfFz(welt, typ) { z.bedrohung } else { 40 }
}

/// C++: void weitereFabriken(Zustand& z, const Lage& l)  (op2_ki.hpp:3820)
// Weitere Fahrzeugfabriken bauen nur Kampffahrzeuge: ohne Kampfbedarf stilllegen, mit Bedarf wieder einschalten.
pub fn weitereFabriken(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    for j in 1..l.vfAnzahl as usize {
        let v: Id = l.vfs[j];
        if !welt.IsLive(v) || welt.GetCurAction(v) != moDone { continue; }
        let mut still: i32 = -1;
        for i in 0..MAX_VF { if z.fabrikStillW[i] == v { still = i as i32; } }
        if still < 0 && z.gruppenBedarf == 0 && !standardVolk(welt, l) && !welt.IsBusy(v) && welt.HasWorkers(v) {
            for i in 0..MAX_VF {
                let alt: Id = z.fabrikStillW[i];
                if z.fabrikStillW[i] != 0 && welt.IsLive(alt) && welt.GetType(alt) == mapVehicleFactory { continue; }
                welt.DoIdle(v);
                z.fabrikStillW[i] = v;
                let vl = welt.Location(v);
                let (vx, vy) = exy(welt, z, vl);
                melde(welt, z, "arbeit", mapVehicleFactory, vx, vy, "weitere Fahrzeugfabrik stillgelegt -- Kampfgruppen voll");
                break;
            }
        } else if still >= 0 && z.gruppenBedarf != 0 {
            if !welt.IsBusy(v) { z.fabrikStillW[still as usize] = 0; continue; }   // laeuft schon wieder
            if l.arbFrei < welt.GetWorkersRequired(mapVehicleFactory, z.spieler) { continue; }   // Arbeiter fehlen
            welt.DoUnIdle(v);
            z.fabrikStillW[still as usize] = 0;
            let vl = welt.Location(v);
            let (vx, vy) = exy(welt, z, vl);
            melde(welt, z, "arbeit", mapVehicleFactory, vx, vy, "weitere Fahrzeugfabrik eingeschaltet -- Kampfgruppen brauchen Fahrzeuge");
        }
    }
}

/// C++: void fahrzeuge(Zustand& z, const Lage& l)  (op2_ki.hpp:3846)
pub fn fahrzeuge(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    if l.vf == 0 { return; }
    // Die erste Fabrik der Liste, als "weitere" stillgelegt (die Reihenfolge der Fabriken hat sich geaendert -- Nutzer-Partie
    // cm03: die Fabrik der zweiten Basis stand kurz vorn, die Hauptfabrik wurde als weitere stillgelegt und nie mehr geweckt,
    // denn weitereFabriken weckt nur ab der zweiten und fahrzeuge nur fabrikStill): an fabrikStill uebergeben
    let eigene = &l.vfs[..l.vfAnzahl.max(0) as usize];
    if z.fabrikStill != 0 && !eigene.contains(&z.fabrikStill) { z.fabrikStill = 0; }   // gehoert nicht mehr dazu (zweite Basis, zerstoert)
    for i in 0..MAX_VF {
        if z.fabrikStillW[i] != 0 && z.fabrikStillW[i] == l.vfs[0] {
            z.fabrikStillW[i] = 0;
            if z.fabrikStill == 0 { z.fabrikStill = l.vfs[0]; }
        }
    }
    weitereFabriken(welt, z, l);
    let vf: Id = if PAR(z.spieler).fabrikLaeuft >= 0.5 { fabrikWahl(welt, z, l) } else { l.vf };
    let t = welt.Tick();
    let vl = welt.Location(vf);
    let (vx, vy) = exy(welt, z, vl);
    if z.fzTyp != 0 {
        if t - z.fzSeit > 100 && t - z.fzSeit < 100 + TAKT + 1 && welt.GetCurAction(vf) == moDone && !welt.IsBusy(vf) {
            let txt = format!("Fahrzeugfabrik nimmt den Auftrag nicht an (Strom {}, Arbeiter {}) -- in 500 Ticks erneut",
                              welt.HasPower(vf) as i32, welt.HasWorkers(vf) as i32);
            melde(welt, z, "warte", z.fzTyp, vx, vy, &txt);
            z.fzTyp = 0; z.fzPauseBis = t + 500;
            return;
        }
        if welt.GetCurAction(vf) == moDevelop || welt.IsBusy(vf) {
            if t - z.fzSeit > GEDULD_KIT { melde(welt, z, "fehler", z.fzTyp, vx, vy, "Fahrzeug dauert zu lange"); z.fzTyp = 0; }
            return;
        }
        z.fzTyp = 0;
    }
    let mut warum: &'static str = "";
    let mut typ = fahrzeugWahl(welt, z, l, &mut warum);
    // Kampffahrzeuge baut die Fabrikgruppe fuer Wache und Sammelgruppe -- hier nur die Wirtschaftsfahrzeuge.
    if istKampfFz(welt, typ) { typ = mapNone; }
    // Missionsmodus: die zweite Basis wartet auf ConVecs (zweitbasis::kitLaden bestellt sie selbst) -- Fabrik nicht stilllegen
    let zbWartet = super::super::modus::mission(z) && super::super::zweitbasis::wartetAufFahrzeug(z);
    // Drehbuch: die Phase verlangt noch Fahrzeuge (befehl::schritt gibt sie in Auftrag) -- Fabrik wach halten
    let befohlen = super::super::befehl::fahrzeugeOffen(welt, z);
    // Keine neuen Fahrzeuge noetig: Fabrik stilllegen (gibt 3 Arbeiter frei), bei Bedarf wieder einschalten.
    if z.fabrikStill == vf {
        if typ == mapNone && z.gruppenBedarf == 0 && !zbWartet && !befohlen { return; }
        if typ == mapNone && zbWartet { typ = mapConVec; warum = "ConVecs fuer die zweite Basis"; }   // nur zum Wecken
        if typ == mapNone && befohlen && z.gruppenBedarf == 0 { typ = mapLynx; warum = "Drehbuch: befohlene Fahrzeuge"; }   // nur zum Wecken
        if typ == mapNone { typ = mapLynx; warum = "Kampfgruppen brauchen Fahrzeuge (Fabrikgruppe)"; }   // nur zum Wecken
        if !welt.IsBusy(vf) { z.fabrikStill = 0; return; }           // laeuft wieder -- naechster Schritt produziert
        let braucht = welt.GetWorkersRequired(mapVehicleFactory, z.spieler);
        if l.arbFrei < braucht {
            // DoUnIdle wirkt nur mit genug freien Arbeitern -> Wunsch an die Rangliste
            z.wunschArbeiter = braucht;
            let r = rang(welt, z, l, mapVehicleFactory) + (if seltenAktiv(welt, z) { 10 } else { 0 });
            z.wunschRang = r;
            z.wunschSeit = welt.Tick();
            if selten(welt, z, 25, 2000) {
                let txt = format!("Fahrzeugfabrik gebraucht ({}), aber nur {} von {} Arbeitern frei", warum, l.arbFrei, braucht);
                melde(welt, z, "warte", mapVehicleFactory, vx, vy, &txt);
            }
            return;
        }
        welt.DoUnIdle(vf);
        z.wunschArbeiter = 0;
        let txt = format!("Fahrzeugfabrik wird eingeschaltet: {}", warum);
        melde(welt, z, "arbeit", mapVehicleFactory, vx, vy, &txt);
        return;
    }
    // PAR lkwWechsel (2026-11-21): unter dem Armee-Soll hoechstens alle lkwWechsel Ticks ein weiterer Lastwagen, sobald jede
    // Route einen hat -- dazwischen baut die Fabrikgruppe Kampffahrzeuge. Gemessen ml2_11: Eden baute nach der zweiten Schmelze
    // 6 Lastwagen am Stueck, die Kampf-Sollzahlen ruhten, beim ersten Plymouth-Angriff hatte Eden 0 Lynx.
    if typ == mapCargoTruck && PAR(z.spieler).lkwWechsel > 0.0 && z.eigeneArmee + z.eigeneWTWert < z.sollArmee
        && l.fz[ix(mapCargoTruck)] >= l.geb[ix(mapCommonOreSmelter)].max(1) {
        let sp = (z.spieler & 7) as usize;
        let letzt = LKW_LETZT.with(|v| v.borrow()[sp]);
        if t - letzt < PAR(z.spieler).lkwWechsel as i32 && t >= letzt {
            typ = mapNone;
            if selten(welt, z, 31, 2000) {
                melde(welt, z, "fahrzeug", mapCargoTruck, vx, vy, &format!("Lastwagen wartet: Armee {} unter Soll {} -- erst Kampffahrzeuge (letzter Lastwagen vor {} Ticks)",
                      z.eigeneArmee + z.eigeneWTWert, z.sollArmee, t - letzt));
            }
        }
    }
    if PAR(z.spieler).fabrikWirtschaft >= 0.5 { wirtschaftVorrang(welt, z, vf, typ, warum, t); }
    if welt.IsBusy(vf) || t < z.fzPauseBis { return; }
    if typ == mapNone {
        if z.gruppenBedarf != 0 || zbWartet || befohlen { return; }              // Fabrikgruppe soll Kampffahrzeuge bauen / zweite Basis
        // Nutzer 2026-11-20: nicht stilllegen, solange die mobile Armee unter ihrem Soll liegt -- auch bevor ein
        // Kampffahrzeug erforscht ist: stillgelegt gab sie ihre Arbeiter ab und bekam sie lange nicht zurueck
        if !standardVolk(welt, l) && welt.GetCurAction(vf) == moDone && welt.HasWorkers(vf) && z.eigeneMobil >= mobilSoll(welt, z, true)
            && !super::super::zweitbasis::gehoertBasis(z, vf) && !(super::super::modus::mission(z) && super::super::zweitbasis::amPosten(welt, z, vf)) {
            welt.DoIdle(vf);
            z.fabrikStill = vf;
            melde(welt, z, "arbeit", mapVehicleFactory, vx, vy, "Fahrzeugfabrik stillgelegt -- keine neuen Fahrzeuge noetig");
        }
        return;
    }
    let waffe = if istKampfFz(welt, typ) {
        let s = l.selten - welt.GetRareOreCost(typ, z.spieler);
        waffeWahl(welt, z, typ, s)
    } else { mapNone };
    let mut kosten = welt.GetOreCost(typ, z.spieler);
    let mut kostenSelten = welt.GetRareOreCost(typ, z.spieler);
    if waffe != mapNone { kosten += welt.GetOreCost(waffe, z.spieler); kostenSelten += welt.GetRareOreCost(waffe, z.spieler); }
    let erzFrei = l.erz;
    if kosten > erzFrei || kostenSelten > l.selten {
        if selten(welt, z, 2, 1500) {
            let n = name(welt, typ);
            let txt = format!("Erz {}/{}, Seltenerz {}/{} fuer {}", erzFrei, kosten, l.selten, kostenSelten, n);
            melde(welt, z, "warte", typ, vx, vy, &txt);
        }
        return;
    }
    welt.DoProduce(vf, typ, waffe);
    if typ == mapCargoTruck { let sp = (z.spieler & 7) as usize; LKW_LETZT.with(|v| v.borrow_mut()[sp] = t); }
    z.fzVf = vf;
    if !istKampfFz(welt, typ) { z.wirtVorrang = 0; }
    z.fzTyp = typ; z.fzSeit = t;
    if istKampfFz(welt, typ) { z.letztesKampfFz = t; }
    if waffe != mapNone {
        let nt = name(welt, typ);
        let nw = name(welt, waffe);
        let sw = sdkWertT(welt, typ, waffe);
        let txt = format!("{} mit {} (SDK-Wert {}): {}", nt, nw, sw, warum);
        melde(welt, z, "fahrzeug", typ, vx, vy, &txt);
    } else {
        let txt = format!("{}", warum);
        melde(welt, z, "fahrzeug", typ, vx, vy, &txt);
    }
}

/// C++: bool sicherNehmen(const Zustand& z, ScGroup& g, Unit u)  (op2_ki.hpp:3935)
// Eine Gruppe fasst hoechstens 32 Einheiten; TakeUnit in eine volle Gruppe loescht den Ueberschuss aus dem Spiel.
pub fn sicherNehmen(welt: &mut dyn Welt, z: &Zustand, g: Gruppe, u: Id) -> bool {
    if welt.TotalUnitCount(g) >= GRUPPE_MAX { return false; }
    welt.TakeUnit(g, u);
    true
}

// FEHLT: ScGroup::TakeAllUnits(quelle) gibt es in welt.rs nicht. Ueberbrueckt: jede Einheit der Quelle aus ihr nehmen
// und ins Ziel geben (Reihenfolge des GroupEnumerator).
pub(super) fn takeAllUnits(welt: &mut dyn Welt, ziel: Gruppe, quelle: Gruppe) {
    for u in welt.GroupEnumerator(quelle) {
        welt.RemoveUnit(quelle, u);
        welt.TakeUnit(ziel, u);
    }
}

/// C++: int sicherAlle(const Zustand& z, ScGroup& ziel, ScGroup& quelle)  (op2_ki.hpp:3941)
// Alles aus quelle, soweit Platz ist; der Rest bleibt in quelle. Liefert, wie viele dort bleiben.
pub fn sicherAlle(welt: &mut dyn Welt, z: &Zustand, ziel: Gruppe, quelle: Gruppe) -> i32 {
    if welt.TotalUnitCount(ziel) + welt.TotalUnitCount(quelle) <= GRUPPE_MAX { takeAllUnits(welt, ziel, quelle); return 0; }
    let mut v: Vec<Id> = Vec::new();
    for u in welt.GroupEnumerator(quelle) {
        if v.len() >= GRUPPE_MAX as usize { break; }
        if welt.IsLive(u) { v.push(u); }
    }
    let n = v.len();
    let mut i = 0;
    while i < n && welt.TotalUnitCount(ziel) < GRUPPE_MAX { welt.RemoveUnit(quelle, v[i]); welt.TakeUnit(ziel, v[i]); i += 1; }
    welt.TotalUnitCount(quelle)
}

/// fabrikWirtschaft: die Fabrikgruppe baut laufend Kampffahrzeuge, die Wahl sagt, ein Wirtschaftsfahrzeug ist dringender
/// -> Kampf-Sollzahlen ruhen lassen (kampfFuehrung), bis die Fabrik frei ist und es baut; nach 4000 Ticks oder ohne
/// Bedarf wieder normal
pub(super) fn wirtschaftVorrang(welt: &mut dyn Welt, z: &mut Zustand, vf: Id, typ: i32, warum: &str, t: i32) {
    if typ == mapNone { z.wirtVorrang = 0; return; }
    if z.wirtVorrang != 0 && t - z.wirtVorrang > 4000 { z.wirtVorrang = 0; z.fzPauseBis = t + 4000; return; }   // nicht dauernd
    if z.wirtVorrang == 0 && welt.IsBusy(vf) && t >= z.fzPauseBis {
        z.wirtVorrang = t;
        let vl = welt.Location(vf); let (vx, vy) = exy(welt, z, vl);
        let nm = name(welt, typ);
        melde(welt, z, "fahrzeug", typ, vx, vy, &format!("Fabrik nach dem laufenden Kampffahrzeug fuer {}: {} (Kampf-Sollzahlen ruhen)", nm, warum));
    }
}

/// fabrikLaeuft: Fahrzeugfabrik fuer Wirtschaftsfahrzeuge -- die des laufenden Auftrags; sonst eine, die laeuft (Strom und
/// Arbeiter), lieber eine freie; sonst die stillgelegte (zum Wiedereinschalten); sonst die erste
pub(super) fn fabrikWahl(welt: &mut dyn Welt, z: &Zustand, l: &Lage) -> Id {
    if z.fzTyp != 0 && z.fzVf != 0 && welt.IsLive(z.fzVf) { return z.fzVf; }
    let alle: Vec<Id> = (0..l.vfAnzahl.max(0) as usize).map(|i| l.vfs[i]).filter(|&v| v != 0 && welt.IsLive(v)).collect();
    let laeuft = |welt: &mut dyn Welt, v: Id| welt.HasPower(v) && welt.HasWorkers(v);
    if let Some(&v) = alle.iter().find(|&&v| laeuft(welt, v) && !welt.IsBusy(v)) { return v; }
    if let Some(&v) = alle.iter().find(|&&v| laeuft(welt, v)) { return v; }
    if z.fabrikStill != 0 && alle.contains(&z.fabrikStill) { return z.fabrikStill; }
    l.vf
}
