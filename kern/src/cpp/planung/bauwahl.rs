//! Bauwahl: was als Naechstes gebaut wird (Kuchen der Bereiche Kolonie/Ziel/Wirtschaft/Verteidigung, bauWahl, weitereFabrikLohnt).
use super::*;
use super::super::*;
use super::{SCAN_TAKT, WERTUNG_TICK, WERT_WT, WERT_LYNX, MAX_WT, AUSSEN_R, B_KOLONIE, B_ZIEL, B_WIRTSCHAFT, B_VERTEIDIGUNG, B_ANZAHL, BEREICH, TECH_SELTENERZ, GEBAEUDE, snp, ix};

/// C++: bool weitereFabrikLohnt(const Zustand& z, const Lage& l, char* text, int n)  (op2_ki.hpp:1749)
pub fn weitereFabrikLohnt(welt: &mut dyn Welt, z: &Zustand, l: &Lage, text: &mut String, n: i32) -> bool {
    let sp = z.spieler;
    let par_ = PAR(sp);
    let f = l.geb[ix(mapVehicleFactory)];
    if f <= 0 || f >= MAX_VF as i32 || standardVolk(welt, l) { return false; }
    if hatGeb(welt, z, l, mapVehicleFactory) > f || l.imBau[ix(mapVehicleFactory)] != 0 { return false; } // eine ist schon unterwegs
    let fz = fahrgestellWahl(welt, z, l.selten);
    if fz == 0 { return false; }
    let rs = l.selten - welt.GetRareOreCost(fz, sp);
    let waffe = waffeWahl(welt, z, fz, rs);
    let sw = sdkWertT(welt, fz, waffe);
    let wert = if sw > 0 { sdkWertT(welt, fz, waffe) } else { WERT_LYNX };
    let mut fehlt = 0;
    let k = kampfFehlt(welt, z, l);
    let m = mobilSoll(welt, z, true) - z.eigeneMobil;
    if k > 0 { fehlt += k; }
    if m > 0 { fehlt += m; }
    let stueck = (fehlt + wert - 1) / wert;
    // Rust (PAR fabrikErz, Nutzer 2026-11-17): Fabriken nach der Wirtschaft -- bringen die Erzrouten mehr Erz, als die
    // vorhandenen Fabriken verbauen koennen, lohnt eine weitere, auch wenn das Armee-Soll schon erreichbar ist.
    if kampfModellNeu() && par_.fabrikErz >= 0.5 {
        let kostenE = (welt.GetOreCost(fz, sp) + welt.GetOreCost(waffe, sp)).max(1) as f64;
        let blE = bilanz(welt, z, l);
        let mut zuflussE = erzRouten(welt, z) as f64 * blE.routeErz as f64 * 1000.0 / par_.horizont;
        if z.zuflussErz > zuflussE { zuflussE = z.zuflussErz; }
        let jeFzE = if welt.GetBuildTime(fz, sp) > 0 && welt.GetProductionCapacity(mapVehicleFactory, sp) > 0 {
            welt.GetBuildTime(fz, sp) as f64 * welt.GetProductionCapacity(mapVehicleFactory, sp) as f64 / 128.0 } else { BAU_FZ as f64 };
        let verbauen = f as f64 * kostenE * 1000.0 / jeFzE;          // Erz je 1000 Ticks, das f Fabriken in Fahrzeuge stecken
        let rnE = rangNeu(welt, z, l, mapVehicleFactory, f);
        let beaE = beansprucht(welt, l, M_ARB as i32, rnE, mapVehicleFactory);
        let reserveE = if par_.wissArbeit >= 0.5 { forschungsReserve(welt, z) } else { 0 };
        let arbOk = welt.GetWorkersRequired(mapVehicleFactory, sp) <= l.arbFrei + l.arbStilllegbar[5] - beaE + reserveE;
        if zuflussE > verbauen * par_.fabrikAuslastung && arbOk {
            *text = snp(format!("Fahrzeugfabrik {}: Erzzufluss {:.0} je 1000 Ticks, {} Fabrik(en) verbauen hoechstens {:.0}", f + 1, zuflussE, f, verbauen), n as usize);
            return true;
        }
    }
    if stueck <= 1 { if welt.Tick() % 5000 < TAKT { melde(welt, z, "fabrik", mapVehicleFactory, z.bx, z.by, &format!("zweite Fabrik nein: Armee fehlt nur {} Fahrzeug(e)", stueck.max(0))); } return false; }
    let kosten = welt.GetOreCost(fz, sp) + welt.GetOreCost(waffe, sp);
    // Erz-Zeit: was ueber den Vorrat hinaus fehlt, kommt mit dem Zufluss (je 1000 Ticks)
    let bl = bilanz(welt, z, l);
    let mut zufluss = erzRouten(welt, z) as f64 * bl.routeErz as f64 * 1000.0 / par_.horizont;
    if z.zuflussErz > zufluss { zufluss = z.zuflussErz; }
    if zufluss < 1.0 { zufluss = 1.0; }
    let fehltErz = stueck * kosten - l.erz;
    let tErz = if fehltErz > 0 { fehltErz as f64 * 1000.0 / zufluss } else { 0.0 };
    // Rust (kampfModell): Bauzeit je Fahrzeug aus dem SDK -- Baupunkte des Fahrgestells x Fabrikleistung / 128 (Produce
    // 0x40E913 / Werks-Countdown; Lynx 600 Baupunkte, Leistung 128 -> 600 Ticks, wie gemessen), sonst der alte Schaetzwert.
    let jeFz = if kampfModellNeu() && welt.GetBuildTime(fz, sp) > 0 && welt.GetProductionCapacity(mapVehicleFactory, sp) > 0 {
        welt.GetBuildTime(fz, sp) as f64 * welt.GetProductionCapacity(mapVehicleFactory, sp) as f64 / 128.0
    } else { BAU_FZ as f64 };
    let dauer = |fabriken: i32| -> f64 { let tf = stueck as f64 * jeFz / fabriken as f64; if tf > tErz { tf } else { tErz } };
    let jetzt = dauer(f);
    let dann = dauer(f + 1);
    if jetzt <= par_.reaktion || dann > jetzt * par_.fabrikGewinn { if welt.Tick() % 5000 < TAKT { melde(welt, z, "fabrik", mapVehicleFactory, z.bx, z.by, &format!("zweite Fabrik nein: {} Fahrzeuge, mit {} Fabrik(en) {} Ticks, mit einer mehr {} (Erz-Grenze {}, Reaktion {})", stueck, f, jetzt as i32, dann as i32, tErz as i32, par_.reaktion as i32)); } return false; } // schnell genug, oder das Erz begrenzt
    // Arbeiter nur aus freien oder ueberzaehligen Stellen (Rang < 50)
    let rn = rangNeu(welt, z, l, mapVehicleFactory, f);
    let bea = beansprucht(welt, l, M_ARB as i32, rn, mapVehicleFactory);
    // Rust (PAR wissArbeit): Wissenschaftler, die die Forschung abgeben kann (Teams ueber 1), springen als Arbeiter ein
    let wissReserve = if kampfModellNeu() && par_.wissArbeit >= 0.5 { forschungsReserve(welt, z) } else { 0 };
    let arbVF = welt.GetWorkersRequired(mapVehicleFactory, sp);
    if arbVF > l.arbFrei + l.arbStilllegbar[5] - bea + wissReserve { if welt.Tick() % 5000 < TAKT { melde(welt, z, "fabrik", mapVehicleFactory, z.bx, z.by, &format!("zweite Fabrik nein: Arbeiter fehlen (braucht {}, frei {}, stilllegbar {}, beansprucht {}, aus Forschung {})", arbVF, l.arbFrei, l.arbStilllegbar[5], bea, wissReserve)); } return false; }
    let nm = name(welt, fz);
    *text = snp(format!("Fahrzeugfabrik {}: {} {} fehlen, mit {} Fabrik(en) {} Ticks, mit einer mehr {} (Erz-Grenze {})",
                        f + 1, stueck, nm, f, jetzt as i32, dann as i32, tErz as i32), n as usize);
    true
}

/// C++: int baubar(const Zustand& z, const Lage& l, int typ, bool darfVorgemerktes, char* grund, int n)  (op2_ki.hpp:1787)
pub fn baubar(welt: &mut dyn Welt, z: &Zustand, l: &Lage, typ: i32, darfVorgemerktes: bool, grund: &mut String, n: i32) -> i32 {
    let n = n as usize;
    if !kitMoeglich(welt, z, typ) { let tf = techFuer(welt, typ); *grund = snp(format!("Technik {} fehlt", tf), n); return 1; }
    let sp = z.spieler;
    let erz = welt.GetOreCost(typ, sp);
    let sel = welt.GetRareOreCost(typ, sp);
    // Wichtigere Vorhaben halten ihr Erz und ihre Arbeiter fest (Ansprueche); das Gebaeude bekommt nur den Rest.
    let hg = hatGeb(welt, z, l, typ);
    let mut r = rangNeu(welt, z, l, typ, hg);
    if r < 0 { r = 0; }
    let frei = (if darfVorgemerktes { l.erz } else { l.erz - l.vorErz }) - beansprucht(welt, l, M_ERZ as i32, r, typ);
    let freiSel = (if darfVorgemerktes { l.selten } else { l.selten - l.vorSelten }) - beansprucht(welt, l, M_SELTEN as i32, r, typ);
    // Seltenerz nur pruefen, wenn das Gebaeude welches kostet
    if erz > frei || (sel > 0 && sel > freiSel) {
        *grund = snp(format!("Erz {}/{}, Seltenerz {}/{} (Rest nach {}Anspruechen ab Rang {})", frei, erz, freiSel, sel,
                             if darfVorgemerktes { "" } else { "Vormerkung und " }, r), n);
        return 2;
    }
    if standardVolk(welt, l) { return 0; }
    // Steht schon eins dieses Typs ohne Personal da, bringt ein weiteres nichts
    if typ < 128 && l.unbesetzt[ix(typ)] > 0 {
        let nm = name(welt, typ);
        *grund = snp(format!("schon {} {} ohne Personal", l.unbesetzt[ix(typ)], nm), n);
        return 4;
    }
    // Arbeiter: frei + stilllegbar (unwichtigere Gebaeude) - was gleich wichtige oder wichtigere Ansprueche brauchen
    let arb = welt.GetWorkersRequired(typ, sp);
    let still = l.arbStilllegbar[if r / 10 > 10 { 10 } else { (r / 10) as usize }];
    let belegt = beansprucht(welt, l, M_ARB as i32, r, typ);
    let verfuegbar = l.arbFrei + still - belegt;
    if arb > verfuegbar {
        *grund = snp(format!("fehlen {} Arbeiter (frei {}, stilllegbar {}, beansprucht ab Rang {}: {})", arb - verfuegbar,
                             l.arbFrei, still, r, belegt), n);
        return 3;
    }
    0
}

/// C++: void vorhabenMerken(const Zustand& z, const Lage& l, int typ)  (op2_ki.hpp:1820)
pub fn vorhabenMerken(welt: &mut dyn Welt, z: &Zustand, l: &Lage, typ: i32) {
    let sp = (z.spieler & 7) as usize;
    let mut a = Anspruch::default();
    a.typ = typ; a.vorhaben = true;
    let hg = hatGeb(welt, z, l, typ);
    a.rang = rangNeu(welt, z, l, typ, hg);
    a.menge[M_ARB] = welt.GetWorkersRequired(typ, z.spieler); a.menge[M_WISS] = welt.GetScientistsRequired(typ, z.spieler);
    a.menge[M_ERZ] = welt.GetOreCost(typ, z.spieler); a.menge[M_SELTEN] = welt.GetRareOreCost(typ, z.spieler);
    g_nVorhaben.with(|nv| g_vorhaben.with(|gv| {
        let mut nv = nv.borrow_mut();
        let mut gv = gv.borrow_mut();
        let mut k: i32 = -1;
        for i in 0..nv[sp] as usize { if gv[sp][i].typ == typ { return; } } // schon gemerkt
        if (nv[sp] as usize) < MAX_VORHABEN { k = nv[sp]; nv[sp] += 1; }
        else {
            for i in 0..MAX_VORHABEN {
                if gv[sp][i].rang < a.rang && (k < 0 || gv[sp][i].rang < gv[sp][k as usize].rang) { k = i as i32; }
            }
        }
        if k >= 0 { gv[sp][k as usize] = a; }
    }));
}

/// C++: bool waehle(const Zustand& z, const Lage& l, int typ, int bereich, bool vorgemerkt, Wahl& w, const char* fmt, ...)  (op2_ki.hpp:1833)
pub fn waehle(welt: &mut dyn Welt, z: &Zustand, l: &Lage, typ: i32, bereich: i32, vorgemerkt: bool, w: &mut Wahl, text: &str) -> bool {
    let mut grund = String::new();
    let nicht = baubar(welt, z, l, typ, vorgemerkt, &mut grund, 160);
    if nicht == 2 || nicht == 3 { vorhabenMerken(welt, z, l, typ); }
    if nicht != 0 {
        // Den Grund des wichtigsten Kandidaten merken, auch wenn er nicht baubar ist (fuer die Spur).
        if w.warum.is_empty() {
            let nm = name(welt, typ);
            w.warum = snp(format!("{} {} wartet: {}", BEREICH[bereich as usize], nm, grund), 200);
        }
        return false;
    }
    let text = snp(text.to_string(), 160);
    w.typ = typ; w.bereich = bereich;
    w.warum = snp(format!("{}: {}", BEREICH[bereich as usize], text), 200);
    true
}

/// C++: int wohnTyp(const Zustand& z, const Lage& l)  (op2_ki.hpp:1855)
pub fn wohnTyp(welt: &mut dyn Welt, z: &Zustand, l: &Lage) -> i32 {
    let sp = z.spieler;
    let mut beste = mapResidence;
    let mut besteKap = welt.GetProductionCapacity(mapResidence, sp);
    let mut besteJePlatz = if besteKap > 0 { welt.GetOreCost(mapResidence, sp) as f64 / besteKap as f64 } else { 1e9 };
    const GROESSER: [i32; 2] = [mapReinforcedResidence, mapAdvancedResidence];
    for &t in GROESSER.iter() {
        let tf = techFuer(welt, t);
        if !hatTech(welt, z, tf) { continue; }
        let kap = welt.GetProductionCapacity(t, sp);
        let erz = welt.GetOreCost(t, sp);
        if kap <= 0 { continue; }
        let jePlatz = erz as f64 / kap as f64;
        if if l.erz - l.vorErz >= 2 * erz { kap > besteKap } else { jePlatz < besteJePlatz } { beste = t; besteKap = kap; besteJePlatz = jePlatz; }
    }
    beste
}

/// C++: bool bereichWahl(const Zustand& z, const Lage& l, int b, Wahl& w)  (op2_ki.hpp:1873)
pub fn bereichWahl(welt: &mut dyn Welt, z: &Zustand, l: &Lage, b: i32, w: &mut Wahl) -> bool {
    if b == B_KOLONIE { return wahlKolonie(welt, z, l, b, w); }
    if b == B_ZIEL { return wahlZiel(welt, z, l, b, w); }
    if b == B_VERTEIDIGUNG { return wahlVerteidigung(welt, z, l, b, w); }
    wahlWirtschaft(welt, z, l, b, w)
}

/// Kolonie: Labor/Kindergarten/Uni zuerst, dann die groesste Luecke (Wohnraum, Medizin, Freizeit, Forum, GORF, DIRT)
pub(super) fn wahlKolonie(welt: &mut dyn Welt, z: &Zustand, l: &Lage, b: i32, w: &mut Wahl) -> bool {
    let bew = l.menschen;
    let volk = standardVolk(welt, l);
    // Fehlt Kindergarten oder Uni die Technik und gibt es noch kein Labor, ist das Labor der erste Kolonie-Schritt
    let wachstumBrauchtLabor = (hatGeb(welt, z, l, mapNursery) == 0 && !kitMoeglich(welt, z, mapNursery))
        || (hatGeb(welt, z, l, mapUniversity) == 0 && !kitMoeglich(welt, z, mapUniversity));
    if wachstumBrauchtLabor && hatGeb(welt, z, l, mapStandardLab) == 0 && hatGeb(welt, z, l, mapBasicLab) == 0
        && waehle(welt, z, l, mapStandardLab, b, false, w, "Labor zuerst -- Kindergarten/Uni brauchen erst Forschung") { return true; }
    // Kindergarten vor Uni, dann nach groesster Luecke: Wohnraum, Medizin, Freizeit, Forum.
    if hatGeb(welt, z, l, mapNursery) == 0 && waehle(welt, z, l, mapNursery, b, false, w, "Kindergarten -- ohne ihn keine neuen Kinder") { return true; }
    if hatGeb(welt, z, l, mapUniversity) == 0 && waehle(welt, z, l, mapUniversity, b, false, w, "Universitaet -- bildet Arbeiter und Wissenschaftler aus") { return true; }
    if volk { return false; }
    // PAR moralFest: stabile Moral -- Wohnraum, Medizin, Freizeit, Forum, GORF, DIRT bringen nichts
    if moralFestAktiv(welt, z) { return false; }
    #[derive(Clone, Copy)]
    struct K { typ: i32, wert: i32, text: &'static str }
    let mut k = [K { typ: 0, wert: 0, text: "" }; 12];
    let mut nk = 0usize;
    let med = if bew > 0 { l.medKap * 100 / bew } else { 100 };
    let frz = if bew > 0 { l.freizeitKap * 100 / bew } else { 100 };
    let frm = if bew > 0 { l.forumKap * 100 / bew } else { 100 };
    let mut wohnUnterwegs = false;
    const HAEUSER: [i32; 3] = [mapResidence, mapReinforcedResidence, mapAdvancedResidence];
    for &t in HAEUSER.iter() {
        if l.imBau[ix(t)] + geplant(welt, z, t) + l.kitsImSpiel[ix(t)] > 0 { wohnUnterwegs = true; }
    }
    let wohnArt = wohnTyp(welt, z, l);
    // Wohnraum-Puffer: 10 Plaetze, beim Bevoelkerungsziel so viel, wie die Kolonie gerade waechst (ein Haus mehr)
    let puffer = if z.ziel == 2 && z.zielErreicht == 0 { 25 } else { 10 };
    if bew + puffer > l.wohnraum && !wohnUnterwegs { k[nk] = K { typ: wohnArt, wert: 60 + bew - l.wohnraum, text: "Wohnraum" }; nk += 1; }
    if med < 100 && l.imBau[ix(mapMedicalCenter)] == 0 { k[nk] = K { typ: mapMedicalCenter, wert: 100 - med, text: "Medizin" }; nk += 1; }
    // Freizeitzentrum nach Luecke, wie Medizin
    if frz < 100 && l.imBau[ix(mapRecreationFacility)] == 0 { k[nk] = K { typ: mapRecreationFacility, wert: 100 - frz, text: "Freizeit" }; nk += 1; }
    // Forum nur, wenn seine Technik fuer diese Kolonie erreichbar ist
    let tfF = techFuer(welt, mapForum);
    let forumTech = tb::tech(tfF);
    let forumGeht = match forumTech { None => true, Some(t) => eigeneKostenT(welt, z, t) >= 0 };
    if forumGeht && frm < 100 && bew > 60 && l.imBau[ix(mapForum)] == 0 { k[nk] = K { typ: mapForum, wert: (100 - frm) / 2, text: "Forum" }; nk += 1; }
    // Missionsmodus (PAR raumhafen, Nutzer 2026-11-22): ein Raumhafen in der Mitte der Basis, ohne Satelliten-Ziel
    if PAR(z.spieler).raumhafen >= 0.5 && super::super::modus::mission(z) && !super::super::befehl::drehbuch(z) && hatGeb(welt, z, l, mapSpaceport) == 0 && l.imBau[ix(mapSpaceport)] == 0
        && geplant(welt, z, mapSpaceport) == 0 && kitMoeglich(welt, z, mapSpaceport) && bew > 60 {
        k[nk] = K { typ: mapSpaceport, wert: 80, text: "Raumhafen der Mission" }; nk += 1;
    }
    // Missionsmodus, Plymouth (PAR spinnen, Nutzer 2026-11-22: "fuege jetzt noch die Spinnen hinzu"): Spinnenfabrik -- nur sie
    // baut Spinnen (Spielregel); die Spinnen reparieren je Basis
    // (die Spinnenfabrik einer zweiten Basis zaehlt nicht -- sonst bekam die Hauptbasis nie Spinnen)
    let afHaupt = welt.PlayerBuildingEnum(z.spieler, mapArachnidFactory).into_iter().filter(|&u| !super::super::zweitbasis::amPosten(welt, z, u)).count();
    if PAR(z.spieler).spinnen >= 0.5 && super::super::modus::mission(z) && !super::super::befehl::drehbuch(z) && !welt.IsEden(z.spieler) && afHaupt == 0
        && l.imBau[ix(mapArachnidFactory)] == 0 && geplant(welt, z, mapArachnidFactory) == 0 && kitMoeglich(welt, z, mapArachnidFactory) && bew > 60 {
        k[nk] = K { typ: mapArachnidFactory, wert: 55, text: "Spinnenfabrik der Mission" }; nk += 1;
    }
    // Missionsmodus (PAR mhd, Nutzer 2026-11-22: "wenn Seltenerz verfuegbar ist, soll ein MHD-Generator gebaut werden")
    if PAR(z.spieler).mhd >= 0.5 && super::super::modus::mission(z) && !super::super::befehl::drehbuch(z) && hatGeb(welt, z, l, mapMHDGenerator) == 0 && l.imBau[ix(mapMHDGenerator)] == 0
        && geplant(welt, z, mapMHDGenerator) == 0 && kitMoeglich(welt, z, mapMHDGenerator)
        && (hatGeb(welt, z, l, mapRareOreMine) > 0 || l.selten >= welt.GetRareOreCost(mapMHDGenerator, z.spieler)) {
        k[nk] = K { typ: mapMHDGenerator, wert: 60, text: "MHD-Generator der Mission (Seltenerz da)" }; nk += 1;
    }
    // Missionsmodus (PAR solar): Solarkraftwerk, sobald ein Solarsatellit im All ist (forschung/raumfahrt.rs startet ihn)
    if PAR(z.spieler).solar >= 0.5 && super::super::modus::mission(z) && !super::super::befehl::drehbuch(z) && welt.GetSatelliteCount(z.spieler, mapSolarSatellite) > 0
        && hatGeb(welt, z, l, mapSolarPowerArray) == 0 && l.imBau[ix(mapSolarPowerArray)] == 0 && geplant(welt, z, mapSolarPowerArray) == 0
        && kitMoeglich(welt, z, mapSolarPowerArray) {
        k[nk] = K { typ: mapSolarPowerArray, wert: 60, text: "Solarkraftwerk der Mission (Satellit im All)" }; nk += 1;
    }
    // GORF und DIRT: nach Auslastung -- GORF genau eins in Betrieb, DIRT bis die angeschlossenen Gebaeude abgedeckt sind
    {
        let tg = techFuer(welt, mapGORF);
        if hatTech(welt, z, tg) && l.gorfLaeuft == 0 && hatGeb(welt, z, l, mapGORF) == 0 && l.imBau[ix(mapGORF)] == 0 {
            k[nk] = K { typ: mapGORF, wert: 60, text: "GORF (keins in Betrieb)" }; nk += 1;
        }
        let td = techFuer(welt, mapDIRT);
        if hatTech(welt, z, td) && l.dirtAbdeckung < 100 && l.imBau[ix(mapDIRT)] == 0 && geplant(welt, z, mapDIRT) + l.kitsImSpiel[ix(mapDIRT)] == 0 {
            k[nk] = K { typ: mapDIRT, wert: 100 - l.dirtAbdeckung, text: "DIRT-Abdeckung" }; nk += 1;
        }
    }
    for i in 0..nk { for j in i + 1..nk { if k[j].wert > k[i].wert { let t = k[i]; k[i] = k[j]; k[j] = t; } } }
    for i in 0..nk {
        // Kleine Luecken (unter 20 %) schliesst die Upgrade-Forschung
        if k[i].typ != wohnArt && (k[i].wert < 20 || (k[i].wert < 30 && l.arbFrei < 2)) { continue; }
        if waehle(welt, z, l, k[i].typ, b, false, w, &format!("{} (Wohnraum {}, Medizin {} %, Freizeit {} %, Forum {} % bei {} Bewohnern)",
                  k[i].text, l.wohnraum, med, frz, frm, bew)) { return true; }
    }
    return false;
}

/// Ziel Satellit: Labor, Advanced Lab, Spaceport der Reihe nach
pub(super) fn wahlZiel(welt: &mut dyn Welt, z: &Zustand, l: &Lage, b: i32, w: &mut Wahl) -> bool {
    if z.ziel != 1 || z.zielErreicht != 0 { return false; }
    // Die Seltenerz-Schmelze ist kein fester Schritt: die Rohstoff-Bilanz entscheidet.
    const REIHE: [i32; 3] = [mapStandardLab, mapAdvancedLab, mapSpaceport];
    const WARUM: [&str; 3] = ["Labor fuer die Forschungskette", "Space Program braucht ein Advanced Lab", "Spaceport"];
    for i in 0..3 {
        if hatGeb(welt, z, l, REIHE[i]) != 0 { continue; }
        if waehle(welt, z, l, REIHE[i], b, true, w, &format!("{}", WARUM[i])) { return true; }
        if kitMoeglich(welt, z, REIHE[i]) { return false; } // naechster Schritt baubar, aber noch nicht leistbar: warten
    }
    return false;
}

/// Verteidigung: Aussenpostenturm, Fahrzeugfabrik, Raumhafen fuer EMP, weitere Fabrik, sonst Tuerme bis zum Turmanteil
pub(super) fn wahlVerteidigung(welt: &mut dyn Welt, z: &Zustand, l: &Lage, b: i32, w: &mut Wahl) -> bool {
    let fehlt = kampfFehlt(welt, z, l);
    let aussen = aussenTurmFehlt(welt, z);
    if aussen >= 0 && hatGeb(welt, z, l, mapGuardPost) < MAX_WT + 2
        && waehle(welt, z, l, mapGuardPost, b, z.bedrohung >= 50, w, &format!("Wachturm fuer den Aussenposten bei {},{} (Bedrohung {})",
                  z.aussenX[aussen as usize], z.aussenY[aussen as usize], z.bedrohung)) { return true; }
    if fehlt <= 0 { return false; }
    // Lynx erforscht, aber keine Fahrzeugfabrik: die zuerst
    if hatTech(welt, z, 3851) && hatGeb(welt, z, l, mapVehicleFactory) == 0
        && waehle(welt, z, l, mapVehicleFactory, b, z.bedrohung >= 50, w, &format!("Fahrzeugfabrik fuer Lynx (fehlt {} Kampfwert)", fehlt)) { return true; }
    // Weitere Fahrzeugfabrik, wenn die vorhandenen zu langsam nachruesten und Erz und Arbeiter sie tragen
    {
        let mut text = String::new();
        // Raumhafen fuer die EMP-Rakete
        let mut empWert = 0;
        if empMoeglich(welt, z) && hatGeb(welt, z, l, mapSpaceport) == 0 && l.geb[ix(mapRareOreMine)] > 0 && empLohnt(welt, z, Some(&mut empWert)) {
            let kw = empKostenWert(welt, z);
            if waehle(welt, z, l, mapSpaceport, b, false, w, &format!("Raumhafen fuer EMP-Raketen (dichteste Feindabwehr Wert {}, Rakete kostet ~{})",
                      empWert, kw)) { return true; }
        }
        if weitereFabrikLohnt(welt, z, l, &mut text, 160) {
            let t2 = text.clone();
            if waehle(welt, z, l, mapVehicleFactory, b, false, w, &format!("{}", t2)) { return true; }
        }
    }
    let mut extraWT = if PAR(z.spieler).gelaende > 0.0 { super::erweiterung::passTuermeSoll(welt, z) } else { 0 };
    // Eden ohne Railgun (Nutzer 2026-11-20): Laser verliert gegen Mikrowelle (Arena: ein Drittel bis doppelt so viel Erz
    // noetig), eine Turmstellung haelt dagegen -- mehr Tuerme (PAR laserTuerme) und ein eigener Turmanteil (laserTurmAnteil)
    let laserZeit = welt.IsEden(z.spieler) && !hatTechId(welt, z, 5506);
    if laserZeit { extraWT += PAR(z.spieler).laserTuerme as i32; }
    if hatGeb(welt, z, l, mapGuardPost) >= MAX_WT + extraWT { return false; }
    // Weniger Tuerme: Tuerme nur, solange noch kein Kampffahrzeug gebaut werden kann, oder bis zum Turm-Anteil am Soll
    {
        let fahrzeugGeht = fahrgestellWahl(welt, z, 1 << 20) != 0 && hatGeb(welt, z, l, mapVehicleFactory) != 0;
        let turmWert = z.eigeneWTWert + (geplant(welt, z, mapGuardPost) + l.kitsImSpiel[ix(mapGuardPost)] + l.imBau[ix(mapGuardPost)]) * WERT_WT;
        let turmWert = if PAR(z.spieler).gelaende > 0.0 && super::erweiterung::passOffen(welt, z) { (turmWert as f64 / PAR(z.spieler).engTurmFaktor) as i32 } else { turmWert };
        let so = PAR(z.spieler).sonde;
        if so >= 2.0 && (welt.Tick() as f64) >= so && (welt.Tick() as f64) < so + 600.0 {
            let (g, k1, k2, hg) = (geplant(welt, z, mapGuardPost), l.kitsImSpiel[ix(mapGuardPost)], l.imBau[ix(mapGuardPost)], hatGeb(welt, z, l, mapGuardPost));
            melde(welt, z, "sonde", mapGuardPost, 0, 0, &format!("Sonde Turm fahrzeugGeht {} turmWert {} (wt {} geplant {} kits {} imBau {}) soll {} anteil {:.6} wt {}",
                fahrzeugGeht as i32, turmWert, z.eigeneWTWert, g, k1, k2, z.sollArmee, z.turmAnteil, hg));
        }
        let anteil = if laserZeit && PAR(z.spieler).laserTurmAnteil > 0.0 { PAR(z.spieler).laserTurmAnteil } else { z.turmAnteil };
        if fahrzeugGeht && turmWert as f64 >= z.sollArmee as f64 * anteil { return false; }
    }
    // Unter Druck darf die Verteidigung auch vorgemerktes Erz nehmen
    return waehle(welt, z, l, mapGuardPost, b, z.bedrohung >= 50, w, &format!("Wachturm (Feind Wert {}, eigener {}, Soll {}, fehlt {}, Bedrohung {})",
                  z.feindArmee, z.eigeneArmee, z.sollArmee, fehlt, z.bedrohung));
}

/// Wirtschaft: Labor fuer die Forschung, Fahrzeugfabrik, Schmelze, Garage, Robot Command Center, Erz/Seltenerz, Lager
pub(super) fn wahlWirtschaft(welt: &mut dyn Welt, z: &Zustand, l: &Lage, b: i32, w: &mut Wahl) -> bool {
    let sp = z.spieler;
    // Labor, das die Forschung braucht
    if z.laborWunsch != 0 && hatGeb(welt, z, l, z.laborWunsch) == 0 {
        let nm = name(welt, z.laborWunsch);
        if waehle(welt, z, l, z.laborWunsch, b, false, w, &format!("{} -- die Forschung Richtung {} braucht es (Dringlichkeit {})",
                  nm, z.laborFuer, z.laborWert)) { return true; }
    }
    if hatGeb(welt, z, l, mapVehicleFactory) == 0 && waehle(welt, z, l, mapVehicleFactory, b, false, w, "Fahrzeuge nachbauen") { return true; }
    // Neues Modell: die Schmelze vor Garage und Robot Command Center -- sie bezahlt sich in wenigen tausend Ticks, das RCC in
    // zehntausenden; wartet sie hinter einem RCC, das auf Erz wartet, kommt sie nie (gemessen: eine Schmelze die ganze Partie).
    if kampfModellNeu() && PAR(sp).erzAusbau >= 0.5 {
        let bl = bilanz(welt, z, l);
        let ew = erzWahl(welt, z, l, &bl);
        if ew.typ == mapCommonOreSmelter && ew.nutzen >= PAR(sp).schmelzeNutzen
            && waehle(welt, z, l, ew.typ, b, false, w, &format!("CommonOreSmelter -- Bilanz: Erz fehlt {} im Horizont, Nutzen {:.1} (Route bringt {})",
                      bl.engpassErz, ew.nutzen, bl.routeErz)) { return true; }
    }
    // Garage: wenn die Ersparnis an den beschaedigten Fahrzeugen in Basisnaehe die Baukosten erreicht
    if hatGeb(welt, z, l, mapGarage) == 0 && kitMoeglich(welt, z, mapGarage) {
        let mut ersparnis = 0.0f64;
        for v in welt.PlayerVehicleEnum(sp) {
            if !welt.IsLive(v) || !kaempfer(welt, v) || welt.GetType(v) == mapScorpion { continue; }
            let lo = welt.Location(v);
            let ex = edX(welt, z, lo);
            let ey = edY(welt, z, lo);
            if abstand2(welt, ex, ey, z.bx, z.by) > 35 * 35 { continue; }
            let ct = welt.GetType(v);
            let wt = welt.GetWeapon(v);
            let hp = welt.GetHitPoints(ct, sp);
            if hp > 0 { ersparnis += 0.5 * (welt.GetOreCost(ct, sp) + welt.GetOreCost(wt, sp)) as f64 * welt.GetDamage(v) as f64 / hp as f64; }
        }
        if ersparnis >= welt.GetOreCost(mapGarage, sp) as f64
            && waehle(welt, z, l, mapGarage, b, false, w, &format!("Garage -- beschaedigte Fahrzeuge, Ersparnis {} Erz", ersparnis as i32)) { return true; }
    }
    // Robot Command Center: schaltet die bessere Wegfindung frei. Lohnt, wenn der Zeitgewinn der Lastwagen die Baukosten einbringt.
    if hatGeb(welt, z, l, mapRobotCommand) == 0 && erzRouten(welt, z) > 0 && hatGeb(welt, z, l, mapVehicleFactory) != 0 && kitMoeglich(welt, z, mapRobotCommand) {
        let blr = bilanz(welt, z, l);
        let mut fluss = erzRouten(welt, z) as f64 * blr.routeErz as f64 * 1000.0 / PAR(sp).horizont; // Erz je 1000 Ticks
        if z.zuflussErz > fluss { fluss = z.zuflussErz; }
        let rk = welt.GetOreCost(mapRobotCommand, sp);
        let gewinnJeTick = 0.03 * fluss / 1000.0;
        let tr = welt.Tick();
        let wt = wertungTick(z.spieler); let rest = if wt - tr > 40000 { wt - tr } else { 40000 };
        if gewinnJeTick > 0.0 && rk as f64 / gewinnJeTick < rest as f64
            && waehle(welt, z, l, mapRobotCommand, b, false, w, &format!("Robot Command Center -- bessere Wegfindung, bringt die Kosten in {} Ticks ein", (rk as f64 / gewinnJeTick) as i32)) { return true; }
    }
    // Erz oder Seltenerz: aus der Rohstoff-Bilanz, ohne feste Reihenfolge. Nur, was sich im Horizont bezahlt.
    {
        let bl = bilanz(welt, z, l);
        let ew = erzWahl(welt, z, l, &bl);
        if ew.typ != mapNone && ew.nutzen >= PAR(sp).schmelzeNutzen {
            let nm = name(welt, ew.typ);
            if waehle(welt, z, l, ew.typ, b, ew.typ == mapRareOreSmelter && z.ziel == 1 && z.zielErreicht == 0, w,
                      &format!("{} -- Bilanz: Erz fehlt {}, Seltenerz fehlt {} (Treiber {}), Nutzen {:.1} (Route bringt {} bzw. {} im Horizont)",
                               nm, bl.engpassErz, bl.engpassSelten, bl.treiber, ew.nutzen, bl.routeErz, bl.routeSelten)) { return true; }
        }
    }
    // Lager nur, wenn ein vorgemerktes Vorhaben mehr Erz braucht, als ins Lager passt -- ein volles Lager ist kein Grund.
    let lager = hatGeb(welt, z, l, mapCommonStorage);
    if lager < 2 && l.erzMax > 0 && l.vorErz > l.erzMax && l.imBau[ix(mapCommonStorage)] == 0
        && waehle(welt, z, l, mapCommonStorage, b, false, w, &format!("{} braucht {} Erz, Lager fasst {} (Lager {} von max. 2)",
                  l.vorFuer, l.vorErz, l.erzMax, lager)) { return true; }
    false
}


/// C++: Wahl bauWahl(const Zustand& z, const Lage& l)  (op2_ki.hpp:2202)
pub fn bauWahl(welt: &mut dyn Welt, z: &Zustand, l: &Lage) -> Wahl {
    let mut w = Wahl { typ: mapNone, bereich: -1, warum: String::new() };
    let sp7 = (z.spieler & 7) as usize;
    let tick = welt.Tick();
    g_nVorhaben.with(|v| v.borrow_mut()[sp7] = 0);
    g_vorhabenTick.with(|v| v.borrow_mut()[sp7] = tick); // die Vorhaben dieser Wahl ersetzen die alten
    // Drehbuch der Mission (befehl.rs): ohne Grundversorgung nur die Liste der Phase
    if super::super::befehl::drehbuch(z) && !super::super::befehl::grundversorgung(z) {
        if let Some(w) = super::super::befehl::bauWahl(welt, z, l) { return w; }
    }
    // 1. Notfaelle
    if hatGeb(welt, z, l, mapCommandCenter) == 0 { notfall(&mut w, mapCommandCenter, "keine Kommandozentrale".to_string()); return w; }
    if hatGeb(welt, z, l, mapStructureFactory) == 0 { notfall(&mut w, mapStructureFactory, "keine Strukturfabrik".to_string()); return w; }
    if hatGeb(welt, z, l, mapTokamak) == 0 { notfall(&mut w, mapTokamak, "kein Kraftwerk".to_string()); return w; }
    if hatGeb(welt, z, l, mapCommonOreSmelter) == 0 { notfall(&mut w, mapCommonOreSmelter, "keine Erzschmelze".to_string()); return w; }
    if hatGeb(welt, z, l, mapAgridome) == 0 { notfall(&mut w, mapAgridome, "keine Nahrung".to_string()); return w; }
    let tokamakUnterwegs = geplant(welt, z, mapTokamak) + l.kitsImSpiel[ix(mapTokamak)] + l.imBau[ix(mapTokamak)];
    let agriUnterwegs = geplant(welt, z, mapAgridome) + l.kitsImSpiel[ix(mapAgridome)] + l.imBau[ix(mapAgridome)];
    // Strompuffer: Reserve ein Fuenftel der Erzeugung, damit ein verlorenes Kraftwerk nicht sofort die Kolonie abschaltet.
    let stromPuffer = 25 + welt.GetAmountPowerGenerated(z.spieler) / 5;
    if l.stromFrei < stromPuffer && tokamakUnterwegs == 0 { notfall(&mut w, mapTokamak, format!("nur {} Strom frei (Puffer {})", l.stromFrei, stromPuffer)); return w; }
    let reichweite = nahrungsReichweite(welt, l);
    if (reichweite < 3000 || l.nahrung < 300) && l.nahrungNetto <= 0 && agriUnterwegs == 0 {
        notfall(&mut w, mapAgridome, format!("Nahrung {}, netto {:+}", l.nahrung, l.nahrungNetto)); return w;
    }
    // Drehbuch mit Grundversorgung: nach den Notfaellen Wohnraum, wenn ueberbelegt, sonst die Liste der Phase
    if super::super::befehl::drehbuch(z) {
        if !standardVolk(welt, l) && l.menschen > l.wohnraum && geplant(welt, z, mapResidence) + l.kitsImSpiel[ix(mapResidence)] + l.imBau[ix(mapResidence)] == 0 {
            let wt = wohnTyp(welt, z, l);
            if waehle(welt, z, l, wt, B_KOLONIE, false, &mut w, &format!("Drehbuch, Grundversorgung: Wohnraum ({} Bewohner auf {} Plaetzen)", l.menschen, l.wohnraum)) { return w; }
        }
        // Bevoelkerung: die Kolonie-Bauten (Wohnen, Kindergarten, Universitaet, Medizin, Freizeit, DIRT, Nahrung) entscheidet
        // die KI weiter selbst -- nur mit der Bauliste starb die Kolonie aus (Sim cm03: 5 Arbeiter, 0 Kinder bei 120 000)
        if !standardVolk(welt, l) {
            // Kindergarten, Universitaet und Medizinzentrum baut sonst die Eroeffnung -- im Drehbuch hier
            for (typ, txt) in [(mapNursery, "Kindergarten (sonst keine Geburten)"), (mapUniversity, "Universitaet (Ausbildung)"), (mapMedicalCenter, "Medizinzentrum")] {
                if hatGeb(welt, z, l, typ) == 0 && kitMoeglich(welt, z, typ)
                    && waehle(welt, z, l, typ, B_KOLONIE, false, &mut w, &format!("Drehbuch, Grundversorgung: {}", txt)) { return w; }
            }
            if wahlKolonie(welt, z, l, B_KOLONIE, &mut w) { return w; }
        }
        if let Some(w2) = super::super::befehl::bauWahl(welt, z, l) { return w2; }
    }
    // PAR wohnVorrang (2026-11-20, Personal-Engpass frueh): ueberbelegt (mehr Bewohner als Wohnraum) und kein Wohnhaus
    // unterwegs -> Wohnhaus vor der Eroeffnung. Gemessen: 54 Bewohner auf 50 Plaetzen bis Tick ~12700, die Moralstufe
    // senkt Geburten und hebt das Sterben (morale.txt CROWDED).
    // wohnVorrang >= 1.5 (2026-11-21): erst nach den Gebaeuden der Eroeffnung -- vorher schob das Wohnhaus den Kindergarten
    let wvStufe = PAR(z.spieler).wohnVorrang;
    // wohnVorrang >= 2.5 (Nutzer 2026-11-21): nur als Teil der Eroeffnung -- Wohnhaus, Kindergarten, Universitaet
    let wvSperre = if wvStufe >= 2.5 { !super::forschung::eroeffnungBautNoch(welt, z, l) } else { wvStufe >= 1.5 && super::forschung::eroeffnungBautNoch(welt, z, l) };
    if wvStufe >= 0.5 && !wvSperre && !moralFestAktiv(welt, z) && !standardVolk(welt, l) && l.menschen > l.wohnraum
        && geplant(welt, z, mapResidence) + l.kitsImSpiel[ix(mapResidence)] + l.imBau[ix(mapResidence)] == 0 && kitMoeglich(welt, z, mapResidence) {
        if waehle(welt, z, l, mapResidence, B_KOLONIE, false, &mut w, &format!("Wohnraum vor allem anderen: {} Bewohner auf {} Plaetzen", l.menschen, l.wohnraum)) { return w; }
    }
    // Eroeffnung der Forschung: das Labor fuer eines ihrer offenen Themen zuerst (der Lynx braucht das Advanced Lab;
    // es baut sich ~3300 Ticks, bestellt erst nach Universitaet und Wohnraum stand es zu spaet)
    // ... aber in der Reihenfolge Kindergarten, Universitaet, Labor (Nutzer 2026-11-19): sie sind der Zweck der ersten
    // Eroeffnungsthemen (3304, 3305). Drueckte das Labor den Kindergarten nach hinten, fehlten spaeter die Arbeiter, ihn zu
    // besetzen -- die Kolonie starb aus (mp6_31 Seed 1)
    // PAR eroeffFolge (Nutzer 2026-11-21): Universitaet, AdvancedLab, dann Kindergarten
    let folge: &[(i32, &str)] = if PAR(z.spieler).eroeffFolge >= 0.5 {
        &[(mapUniversity, "Eroeffnung (Folge): Universitaet zuerst"), (mapAdvancedLab, "Eroeffnung (Folge): AdvancedLab nach der Universitaet"),
          (mapNursery, "Eroeffnung (Folge): Kindergarten nach Universitaet und Labor")]
    } else {
        &[(mapNursery, "Eroeffnung: Kindergarten vor Universitaet und Labor -- ohne ihn keine Geburten"),
          (mapUniversity, "Eroeffnung: Universitaet vor dem Labor -- bildet Arbeiter und Wissenschaftler aus")]
    };
    if !eroeffnungOffen(welt, z).is_empty() {
        for &(typ, text) in folge.iter() {
            if hatGeb(welt, z, l, typ) == 0 && kitMoeglich(welt, z, typ) {
                if waehle(welt, z, l, typ, B_KOLONIE, false, &mut w, text) { return w; }
            }
        }
    }
    for id in eroeffnungOffen(welt, z) {
        let lab = laborImWeg(welt, z, l, id);
        if lab == 0 { continue; }
        if hatGeb(welt, z, l, lab) == 0 && kitMoeglich(welt, z, lab) {
            let nm = name(welt, lab);
            if waehle(welt, z, l, lab, B_WIRTSCHAFT, false, &mut w, &format!("Eroeffnung: {} fuer Tech {}", nm, id)) { return w; }
        }
        break;
    }
    // Fabrik frueh: sobald ihre Technik da ist und die Waffenforschung begonnen hat, die erste Fahrzeugfabrik bauen
    if fabrikErwartet(welt, z, l) {
        if waehle(welt, z, l, mapVehicleFactory, B_VERTEIDIGUNG, z.bedrohung >= 50, &mut w, "Fabrik frueh: Waffenforschung begonnen, Fahrzeugfabrik soll bereitstehen") { return w; }
    }
    // PAR moralBau (2026-11-21): eine erforschte Moral-Technik ohne ihr Gebaeude senkt die Moral (sie vergroessert den Teiler
    // der Moralformel, Punkte gibt erst das Gebaeude) -- ml2_05 Seed 1: Medizin 3303 bei 13 203, Medical Center erst bei
    // 21 803, Moral 85 -> 68 %, die Kolonie wuchs nicht mehr. Darum das Gebaeude sofort, vor Tuermen und Ausbau.
    if PAR(z.spieler).moralBau >= 0.5 && !standardVolk(welt, l) {
        for (tech, typ) in [(3303, mapMedicalCenter), (3306, mapRecreationFacility), (3301, mapDIRT), (3302, mapGORF)] {
            if hatTechId(welt, z, tech) && hatGeb(welt, z, l, typ) == 0 && l.imBau[ix(typ)] == 0 && l.kitsImSpiel[ix(typ)] == 0 && kitMoeglich(welt, z, typ) {
                let nm = name(welt, typ);
                if waehle(welt, z, l, typ, B_KOLONIE, false, &mut w, &format!("Moral: Tech {} erforscht, ohne {} senkt sie die Moral", tech, nm)) { return w; }
            }
        }
    }
    // Seltenerz-Schmelze, sobald Seltenerz-Fahrzeuge sich lohnen (Parameter seltenGewinn)
    if seltenErwartet(welt, z, l) {
        let sv = (100.0 * seltenVorteil(welt, z)) as i32;
        let sg = (100.0 * PAR(z.spieler).seltenGewinn) as i32;
        if waehle(welt, z, l, mapRareOreSmelter, B_WIRTSCHAFT, false, &mut w, &format!("Seltenerz-Schmelze: Fahrzeuge mit Seltenerz bringen {} % mehr Kampfkraft je Erz (Schwelle {} %)",
                  sv, sg)) { return w; }
    }
    // Rust (PAR fabrikErz): weitere Fahrzeugfabrik, sobald der Erzzufluss mehr ist, als die Fabriken verbauen -- unabhaengig
    // vom Bau-Kuchen und vom Armee-Soll (Nutzer 2026-11-17: die KI hatte nur eine Fabrik und griff zu selten an)
    if kampfModellNeu() && PAR(z.spieler).fabrikErz >= 0.5 && l.geb[ix(mapVehicleFactory)] > 0 {
        let mut text = String::new();
        if weitereFabrikLohnt(welt, z, l, &mut text, 160) && waehle(welt, z, l, mapVehicleFactory, B_VERTEIDIGUNG, false, &mut w, &text) { return w; }
    }
    // 2. Bau-Kuchen: Bereiche nach Anteil absteigend probieren
    let mut anteil = [0i32; B_ANZAHL as usize];
    bauKuchen(welt, z, l, &mut anteil);
    let mut reihe = [0usize, 1, 2, 3];
    for i in 0..B_ANZAHL as usize {
        for j in i + 1..B_ANZAHL as usize { if anteil[reihe[j]] > anteil[reihe[i]] { let t = reihe[i]; reihe[i] = reihe[j]; reihe[j] = t; } }
    }
    for k in 0..B_ANZAHL as usize {
        if anteil[reihe[k]] == 0 { break; }
        if bereichWahl(welt, z, l, reihe[k] as i32, &mut w) {
            let text = snp(format!("{} [Kuchen K{} Z{} W{} V{}]", w.warum, anteil[0], anteil[1], anteil[2], anteil[3]), 200);
            w.warum = snp(text, 200);
            return w;
        }
    }
    // 3. Lueckenfueller: nur mit Erz, das nicht vorgemerkt ist
    let mut grund = String::new();
    if l.stromFrei < 150 && tokamakUnterwegs == 0 && (PAR(z.spieler).lueckeKlug < 0.5 || stromBaldNoetig(welt, z, l) > l.stromFrei)
        && baubar(welt, z, l, mapTokamak, false, &mut grund, 120) == 0 {
        w.typ = mapTokamak; w.bereich = -1;
        w.warum = snp(format!("Lueckenfueller: nur {} Strom frei", l.stromFrei), 200); return w;
    }
    if l.nahrungNetto < 0 && agriUnterwegs == 0 && baubar(welt, z, l, mapAgridome, false, &mut grund, 120) == 0 {
        w.typ = mapAgridome; w.bereich = -1;
        w.warum = snp(format!("Lueckenfueller: Nahrung netto {:+}, reicht {} Ticks", l.nahrungNetto, reichweite), 200); return w;
    }
    w.typ = mapNone; // w.warum enthaelt ggf. den Grund des wichtigsten wartenden Kandidaten
    w
}

/// Rust (PAR wissArbeit): Wissenschaftler in Forschungsteams ueber dem ersten je Labor -- so viele koennen als Arbeiter
/// einspringen, wenn die Forschung sie abgibt.
pub fn forschungsReserve(welt: &mut dyn Welt, z: &Zustand) -> i32 {
    let mut n = 0;
    for &lt in [mapBasicLab, mapStandardLab, mapAdvancedLab].iter() {
        for lab in welt.PlayerBuildingEnum(z.spieler, lt) {
            if !welt.IsLive(lab) || welt.GetLabCurrentTopic(lab) <= 0 { continue; }
            n += (welt.GetLabScientistCount(lab) - 1).max(0);
        }
    }
    n
}

/// lueckeKlug: Strombedarf der Gebaeude, die der Bebauungsplan als naechste vorsieht und die schon baubar sind
pub fn stromBaldNoetig(welt: &mut dyn Welt, z: &Zustand, l: &Lage) -> i32 {
    let mut n = 0;
    let mut grund = String::new();
    for pp in super::super::vorplan::reserviert(z) {
        if pp.typ == mapTokamak || pp.typ == mapMHDGenerator || pp.typ == mapSolarPowerArray || pp.typ == mapGeothermalPlant { continue; }
        if baubar(welt, z, l, pp.typ, false, &mut grund, 0) != 0 { continue; }
        n += welt.GetPowerRequired(pp.typ, z.spieler).max(0);
    }
    n
}
