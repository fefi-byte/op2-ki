//! Personal: Arbeiter-Rangliste (arbeitErheben), Stilllegen/Einschalten (arbeit), Leerlauf, gesperrte Bausaetze.
use super::*;
use super::{GEBAEUDE, TECH_RAUMFAHRT};

/// Vorhaben der letzten Bauwahl (g_vorhaben[sp][0..g_nVorhaben[sp]]) und g_vorhabenTick[sp].
pub(super) fn vorhabenListe(sp: usize) -> (i32, Vec<Anspruch>) {
    let tick = g_vorhabenTick.with(|v| v.borrow()[sp]);
    let n = g_nVorhaben.with(|v| v.borrow()[sp]).max(0) as usize;
    let liste = g_vorhaben.with(|v| v.borrow()[sp][..n.min(MAX_VORHABEN)].to_vec());
    (tick, liste)
}

/// C++: void arbeitErheben(const Zustand& z, Lage& l)  (op2_ki.hpp:2266)
pub fn arbeitErheben(welt: &mut dyn Welt, z: &Zustand, l: &mut Lage) {
    for &t in GEBAEUDE.iter() {
        let liste = welt.PlayerBuildingEnum(z.spieler, t);
        let r = rang(welt, z, l, t);
        let arb = welt.GetWorkersRequired(t, z.spieler);
        let wiss = welt.GetScientistsRequired(t, z.spieler);
        if arb <= 0 && wiss <= 0 { continue; }
        for u in liste {
            // Im Bau: braucht sein Personal, sobald es steht (Anspruch). Under construction: claims its staff.
            if welt.GetCurAction(u) == moObjWait {
                let rn = rangNeu(welt, z, l, t, l.geb[t as usize] - 1);
                anspruchDazu(welt, l, t, rn, arb, wiss, 0, 0, false);
                continue;
            }
            if welt.GetCurAction(u) != moDone || welt.IsBusy(u) { continue; } // im Bau, stillgelegt oder arbeitet
            if arb > 0 && !welt.HasWorkers(u) {
                l.arbFehlt += arb;
                if t < 128 { l.unbesetzt[t as usize] += 1; }
                anspruchDazu(welt, l, t, r, arb, 0, 0, 0, false);
                continue;
            }
            if wiss > 0 && !welt.HasScientists(u) {
                l.wissFehlt += wiss;
                if t < 128 { l.unbesetzt[t as usize] += 1; }
                anspruchDazu(welt, l, t, r, 0, wiss, 0, 0, false);
                continue;
            }
            if arb > 0 {
                for i in 0..=10 { if r < i * 10 { l.arbStilllegbar[i as usize] += arb; } }
            }
        }
    }
    // Bausaetze unterwegs zum Bauplatz (schon bezahlt): Personal. Kits on the way: their staff.
    for i in 0..z.anzahlPlaetze as usize {
        let q = z.plaetze[i];
        if q.zustand != P_FAHRT && q.zustand != P_BAUT { continue; }
        let typ = q.typ as i32;
        let rn = rangNeu(welt, z, l, typ, l.geb[typ as usize]);
        let a = welt.GetWorkersRequired(typ, z.spieler);
        let w = welt.GetScientistsRequired(typ, z.spieler);
        anspruchDazu(welt, l, typ, rn, a, w, 0, 0, false);
    }
    // Vorhaben der letzten Bauwahl und Weck-Wunsch (Fahrzeugfabrik, Strukturfabrik). Plans and wake wishes.
    let sp = (z.spieler & 7) as usize;
    {
        let (vt, liste) = vorhabenListe(sp);
        if welt.Tick() - vt < 6000 {
            for a in liste.iter() {
                anspruchDazu(welt, l, a.typ, a.rang, a.menge[M_ARB], a.menge[M_WISS], a.menge[M_ERZ], a.menge[M_SELTEN], true);
            }
        }
    }
    if z.wunschArbeiter > 0 && welt.Tick() - z.wunschSeit < 1000 {
        anspruchDazu(welt, l, -2, z.wunschRang, z.wunschArbeiter, 0, 0, 0, false);
    }
    // Erwartete erste Fahrzeugfabrik: Arbeiter und Erz, sobald der Weg zum Lynx begonnen hat (sonst bildete die Uni die Arbeiter
    // aus, bevor die Bauwahl die Fabrik ueberhaupt pruefte -- Eden 2026-11-09: Uni 20 Ticks nach Forschungsbeginn, Fabrik erst 18000).
    // Expected first vehicle factory: its workers and ore are claimed as soon as the way to the Lynx has begun.
    if fabrikErwartet(welt, z, l) {
        let mut schon = false;
        let (vt, liste) = vorhabenListe(sp);
        if welt.Tick() - vt < 6000 { for a in liste.iter() { if a.typ == mapVehicleFactory { schon = true; } } }
        if !schon {
            let rg = rang(welt, z, l, mapVehicleFactory);
            let arb = welt.GetWorkersRequired(mapVehicleFactory, z.spieler);
            let erz = welt.GetOreCost(mapVehicleFactory, z.spieler);
            let sel = welt.GetRareOreCost(mapVehicleFactory, z.spieler);
            anspruchDazu(welt, l, mapVehicleFactory, rg, arb, 0, erz, sel, true);
        }
    }
    // Erwartete Seltenerz-Schmelze: Arbeiter und Erz; solange das Erz fehlt, warten neue Kampffahrzeuge (Fabrikgruppe haelt ihr Soll).
    // Expected rare-ore smelter: its workers and ore; while ore is short, new combat vehicles wait.
    if seltenErwartet(welt, z, l) {
        let mut schon = false;
        let (vt, liste) = vorhabenListe(sp);
        if welt.Tick() - vt < 6000 { for a in liste.iter() { if a.typ == mapRareOreSmelter { schon = true; } } }
        if !schon {
            let rg = rang(welt, z, l, mapRareOreSmelter);
            let arb = welt.GetWorkersRequired(mapRareOreSmelter, z.spieler);
            let erz = welt.GetOreCost(mapRareOreSmelter, z.spieler);
            let sel = welt.GetRareOreCost(mapRareOreSmelter, z.spieler);
            anspruchDazu(welt, l, mapRareOreSmelter, rg, arb, 0, erz, sel, true);
        }
        l.seltenSpart = l.erz < welt.GetOreCost(mapRareOreSmelter, z.spieler);
    }
    // Lohnende Erzschmelze -- Arbeiter und Erz vormerken; solange das Erz fehlt, warten neue Kampffahrzeuge.
    {
        let bl = super::planung::bilanz(welt, z, l);
        let ew = super::planung::erzWahl(welt, z, l, &bl);
        if ew.typ == mapCommonOreSmelter && ew.nutzen >= PAR(z.spieler).schmelzeNutzen {
            let rg = rang(welt, z, l, mapCommonOreSmelter);
            let arb = welt.GetWorkersRequired(mapCommonOreSmelter, z.spieler);
            let erz = welt.GetOreCost(mapCommonOreSmelter, z.spieler);
            anspruchDazu(welt, l, mapCommonOreSmelter, rg, arb, 0, erz, 0, true);
            l.erzSpart = l.erz < erz;
        }
    }
    // vormerkungen liest nur l.geb (nicht die Ausgabefelder): Kopie der Lage als Eingabe
    let kopie = l.clone();
    let (mut ve, mut vs, mut vf) = (l.vorErz, l.vorSelten, l.vorFuer);
    vormerkungen(welt, z, &kopie, &mut ve, &mut vs, &mut vf);
    l.vorErz = ve; l.vorSelten = vs; l.vorFuer = vf;
}

// Rust-Erweiterung: Erz, das beim Stilllegen von u verfaellt. Die Engine zaehlt nur laufende Schmelzen und
// Lager zur Lagergroesse (colony 0x472xxx: max = laufende Schmelzen x Lager + laufende Tanks x Lager) und kappt den Vorrat
// darauf -- eine stillgelegte einzige Schmelze loescht den ganzen Vorrat. Anteil je laufendem Lagergebaeude gleich verteilt.
pub fn erzVerlustBeiStill(welt: &mut dyn Welt, z: &Zustand, l: &Lage, u: Id) -> i32 {
    let typ = welt.GetType(u);
    let (gruppe, vorrat, max): (&[i32], i32, i32) = match typ {
        t if t == mapCommonOreSmelter || t == mapCommonStorage => (&[mapCommonOreSmelter, mapCommonStorage], l.erz, l.erzMax),
        t if t == mapRareOreSmelter || t == mapRareStorage => (&[mapRareOreSmelter, mapRareStorage], l.selten, l.seltenMax),
        _ => return 0,
    };
    let mut n = 0;
    for &gt in gruppe.iter() {
        for v in welt.PlayerBuildingEnum(z.spieler, gt) {
            if welt.IsLive(v) && welt.GetCurAction(v) == moDone && !welt.IsBusy(v) && welt.HasPower(v) && welt.HasWorkers(v) { n += 1; }
        }
    }
    if n == 0 { return 0; }
    let rest = max - max / n;
    if vorrat > rest { vorrat - rest } else { 0 }
}

// Arbeiter-Rangliste / worker ranking (alle 200 Ticks, eine Aenderung je Runde):
// - Ein wichtiges Gebaeude steht fertig, aber ohne Arbeiter -> das unwichtigste laufende stilllegen (DoIdle).
// - Sonst, wenn Arbeiter frei sind -> das wichtigste stillgelegte wieder einschalten (DoUnIdle).
// Idle the least important running building for a more important unstaffed one; unidle when workers free up.
/// C++: void arbeit(Zustand& z, const Lage& l)  (op2_ki.hpp:2328)
/// Wer im Personalvergleich fehlt bzw. Opfer waere: wichtigstes fertiges Gebaeude ohne Arbeiter (fehlt) bzw. ohne
/// Wissenschaftler (fehltWiss), unwichtigstes laufendes mit Arbeitern (opfer), samt Rang
pub(super) struct Personal { fehlt: Id, opfer: Id, fehltWiss: Id, rFehlt: i32, rOpfer: i32 }

/// C++: void arbeit(Zustand& z, const Lage& l). Je 200 Ticks hoechstens eine Massnahme. Rust 2026-11-18 aufgeteilt (Verhalten gleich).
pub fn arbeit(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    let t = welt.Tick();
    if t < z.naechsteArbeit || standardVolk(welt, l) { return; }
    z.naechsteArbeit = t + 200;
    let sp = z.spieler;
    parkListeBereinigen(welt, z);
    if haengendeWecken(welt, z, l, sp) { return; }
    // Strommangel: ein wichtiges Gebaeude steht ohne Strom, ein unwichtigeres laeuft -> das unwichtigere abschalten.
    // Power shortage: idle a lesser consumer.
    // Anschluss aus dem Roehrennetz; ein Gebaeude ausserhalb bekommt nie Personal (Arbeiter nur im Netz einer
    // Kommandozentrale), Stilllegen fuer es waere reiner Verlust. Lager, deren Stilllegen Erz vernichtet, sind keine Opfer.
    let netz = netzKacheln(welt, z);
    if stromUmverteilen(welt, z, l, sp) { return; }
    let Personal { fehlt, opfer, fehltWiss, rFehlt, rOpfer } = personalErheben(welt, z, l, sp, &netz);
    if wissFuerGebaeude(welt, z, sp, fehlt, fehltWiss) { return; }
    // Arbeiter-Wunsch (z.B. Fahrzeugfabrik fuer den Seltenerz-Miner) wie ein fehlendes Gebaeude behandeln
    let wunsch = z.wunschArbeiter > 0 && welt.Tick() - z.wunschSeit < 1000 && l.arbFrei < z.wunschArbeiter;
    if wunschStilllegen(welt, z, fehlt, wunsch, opfer, rOpfer) { return; }
    if wissAlsArbeiter(welt, z, l, sp, fehlt) { return; }
    if fuerFehlendesStilllegen(welt, z, fehlt, opfer, rFehlt, rOpfer) { return; }
    if fehlt != 0 || wunsch || l.arbFrei <= 0 || z.anzahlGeparkt == 0 { return; }
    geparktesWecken(welt, z, l, sp);
}

/// Liste bereinigen: zerstoerte oder schon wieder laufende Gebaeude entfernen
pub(super) fn parkListeBereinigen(welt: &mut dyn Welt, z: &mut Zustand) {
    let mut k = 0usize;
    for i in 0..z.anzahlGeparkt as usize {
        let u: Id = z.geparkt[i];
        if welt.IsLive(u) && welt.IsBusy(u) && welt.GetCurAction(u) == moDone {
            z.geparktGrund[k] = z.geparktGrund[i]; z.geparkt[k] = z.geparkt[i]; k += 1;
        }
    }
    z.anzahlGeparkt = k as i32;
}

/// Haengende Stilllegungen: ein Gebaeude steht stillgelegt da, ohne dass die KI es geparkt hat. Erneut einschalten.
/// Stuck idle: re-issue DoUnIdle for buildings idle without being parked by the AI.
pub(super) fn haengendeWecken(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, sp: i32) -> bool {
    for &typ in GEBAEUDE.iter() {
        // Nur Gebaeude, die Strom UND Personal brauchen. Only buildings needing power and staff.
        if welt.GetPowerRequired(typ, sp) <= 0 || (welt.GetWorkersRequired(typ, sp) <= 0 && welt.GetScientistsRequired(typ, sp) <= 0) { continue; }
        for u in welt.PlayerBuildingEnum(sp, typ) {
            if welt.GetCurAction(u) != moDone || !welt.IsBusy(u) || welt.HasPower(u) || welt.HasWorkers(u) { continue; }
            if u == z.fabrikStill { continue; }
            let mut weitereStill = false;
            for i in 0..MAX_VF { if z.fabrikStillW[i] == u { weitereStill = true; } }
            if weitereStill { continue; }
            let mut geparktHier = false;
            for i in 0..z.anzahlGeparkt as usize { if z.geparkt[i] == u { geparktHier = true; } }
            if geparktHier { continue; }
            let rr = rang(welt, z, l, typ);
            if l.arbFrei - beansprucht(welt, l, M_ARB as i32, rr + 1, -1) < welt.GetWorkersRequired(typ, sp) { continue; } // DoUnIdle wirkt nur mit genug Arbeitern
            welt.DoUnIdle(u);
            let lo = welt.Location(u);
            let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
            melde(welt, z, "arbeit", typ, x, y, "hing stillgelegt (nicht geparkt) -- erneut eingeschaltet");
            return true;
        }
    }
    false
}

/// Strommangel: ein wichtiges Gebaeude steht ohne Strom, ein unwichtigeres laeuft -> das unwichtigere abschalten
pub(super) fn stromUmverteilen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, sp: i32) -> bool {
    if z.anzahlGeparkt < 16 {
        let (mut ohne, mut weg): (Id, Id) = (0, 0);
        let (mut rOhne, mut rWeg) = (-1, 1000);
        for &typ in GEBAEUDE.iter() {
            if welt.GetPowerRequired(typ, sp) <= 0 || typ == mapCommandCenter { continue; }
            let r = rang(welt, z, l, typ);
            for u in welt.PlayerBuildingEnum(sp, typ) {
                if !welt.IsLive(u) || welt.GetCurAction(u) != moDone || welt.IsBusy(u) { continue; }
                if !welt.HasPower(u) { if r > rOhne { rOhne = r; ohne = u; } }
                else if r < rWeg && typ != mapGuardPost && erzVerlustBeiStill(welt, z, l, u) <= 0 { rWeg = r; weg = u; } // Tuerme halten die Kolonie, sie bleiben an
            }
        }
        if ohne != 0 && weg != 0 && rWeg + 5 < rOhne {
            welt.DoIdle(weg);
            let n = z.anzahlGeparkt as usize;
            z.geparktGrund[n] = 0;
            z.geparkt[n] = weg; z.anzahlGeparkt += 1;
            let wt = welt.GetType(weg);
            let lo = welt.Location(weg);
            let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
            let ot = welt.GetType(ohne);
            let on = name(welt, ot);
            melde(welt, z, "arbeit", wt, x, y, &format!("stillgelegt (Rang {}) -- Strom fuer {} (Rang {})", rWeg, on, rOhne));
            return true;
        }
    }
    false
}

/// Wichtigstes fertiges Gebaeude ohne Arbeiter (bzw. ohne Wissenschaftler) und unwichtigstes laufendes mit Arbeitern
pub(super) fn personalErheben(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, sp: i32, netz: &[bool]) -> Personal {
    let (mut fehlt, mut opfer, mut fehltWiss): (Id, Id, Id) = (0, 0, 0);
    let (mut rFehlt, mut rOpfer, mut rFehltWiss) = (-1, 1000, -1);
    for &typ in GEBAEUDE.iter() {
        let arbN = welt.GetWorkersRequired(typ, sp);
        let wissN = welt.GetScientistsRequired(typ, sp);
        if arbN <= 0 && wissN <= 0 { continue; }
        let r = rang(welt, z, l, typ);
        for u in welt.PlayerBuildingEnum(sp, typ) {
            if welt.GetCurAction(u) != moDone || welt.IsBusy(u) { continue; } // im Bau, stillgelegt oder arbeitet
            let ausserhalb = hatStutzen(welt, typ) && typ != mapCommandCenter && {
                let lo = welt.Location(u);
                let (ex, ey) = (edX(welt, z, lo), edY(welt, z, lo));
                let r = rechteckUm(welt, typ, ex, ey);
                !imNetz(z, netz, &r)
            };
            if arbN > 0 && !welt.HasWorkers(u) { if r > rFehlt && !ausserhalb { rFehlt = r; fehlt = u; } }
            else if wissN > 0 && !welt.HasScientists(u) { if r > rFehltWiss && !ausserhalb { rFehltWiss = r; fehltWiss = u; } }
            else if arbN > 0 && r < rOpfer && typ != mapCommandCenter && erzVerlustBeiStill(welt, z, l, u) <= 0 { rOpfer = r; opfer = u; }
        }
    }
    Personal { fehlt, opfer, fehltWiss, rFehlt, rOpfer }
}

/// Gebaeude ohne Wissenschaftler: die unwichtigste laufende Forschung gibt welche ab (Personal geht vor Forschung)
pub(super) fn wissFuerGebaeude(welt: &mut dyn Welt, z: &mut Zustand, sp: i32, fehlt: Id, fehltWiss: Id) -> bool {
    if fehltWiss != 0 && fehlt == 0 {
        let mut quelle: Id = 0;
        let (mut gQ, mut aQ) = (99, 0);
        let labTypen = [mapBasicLab, mapStandardLab, mapAdvancedLab];
        for &lt in labTypen.iter() {
            for lab in welt.PlayerBuildingEnum(sp, lt) {
                if !welt.IsLive(lab) { continue; } // gerade zerstoert
                let th = welt.GetLabCurrentTopic(lab);
                let anz = welt.GetLabScientistCount(lab);
                if th <= 0 || anz <= 1 { continue; }
                let i2 = if th < welt.GetTechCount() { th } else { techIndex(welt, th) };
                let g = if i2 >= 0 { let id = super::forschung::techInfo(welt, i2).id; themaGruppe(welt, id) } else { 0 };
                if g < gQ || (g == gQ && anz > aQ) { gQ = g; aQ = anz; quelle = lab; }
            }
        }
        if quelle != 0 {
            let ft = welt.GetType(fehltWiss);
            let mut ab = welt.GetScientistsRequired(ft, sp);
            if ab > aQ - 1 { ab = aQ - 1; }
            welt.SetLabScientistCount(quelle, aQ - ab);
            welt.RecalculateValues(sp);
            let lo = welt.Location(fehltWiss);
            let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
            melde(welt, z, "arbeit", ft, x, y, &format!("ohne Wissenschaftler -- Forschung im Labor gibt {} ab (Personal vor Forschung)", ab));
            return true;
        }
    }
    false
}

/// Fuer Wuensche (z.B. Fahrzeugfabrik fuer den Seltenerz-Miner) nur Unwichtiges stilllegen (Rang < 50)
pub(super) fn wunschStilllegen(welt: &mut dyn Welt, z: &mut Zustand, fehlt: Id, wunsch: bool, opfer: Id, rOpfer: i32) -> bool {
    if fehlt == 0 && wunsch && opfer != 0 && rOpfer < 50 && rOpfer + 5 < z.wunschRang && z.anzahlGeparkt < 16 {
        welt.DoIdle(opfer);
        let n = z.anzahlGeparkt as usize;
        z.geparktGrund[n] = 0;
        z.geparkt[n] = opfer; z.anzahlGeparkt += 1;
        let ot = welt.GetType(opfer);
        let lo = welt.Location(opfer);
        let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
        melde(welt, z, "arbeit", ot, x, y,
              &format!("stillgelegt (Rang {}), Arbeiter fuer einen Wunsch (Rang {}, {} Arbeiter)", rOpfer, z.wunschRang, z.wunschArbeiter));
        return true;
    }
    false
}

/// Fehlen einem Gebaeude Arbeiter (Nutzer 2026-11-17), gibt die unwichtigste laufende Forschung
/// Wissenschaftler ab -- die Engine setzt freie Wissenschaftler selbst als Arbeiter ein. Erst danach wird stillgelegt.
pub(super) fn wissAlsArbeiter(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, sp: i32, fehlt: Id) -> bool {
    if fehlt != 0 {
        let ft = welt.GetType(fehlt);
        let brauche = welt.GetWorkersRequired(ft, sp) - l.arbFrei.max(0) - l.wissFrei.max(0);
        if brauche > 0 {
            let mut quelle: Id = 0;
            let (mut gQ, mut aQ) = (99, 0);
            for &lt in [mapBasicLab, mapStandardLab, mapAdvancedLab].iter() {
                for lab in welt.PlayerBuildingEnum(sp, lt) {
                    if !welt.IsLive(lab) { continue; }
                    let th = welt.GetLabCurrentTopic(lab);
                    let anz = welt.GetLabScientistCount(lab);
                    if th <= 0 || anz <= 1 { continue; }
                    let i2 = if th < welt.GetTechCount() { th } else { techIndex(welt, th) };
                    let g = if i2 >= 0 { let id = super::forschung::techInfo(welt, i2).id; themaGruppe(welt, id) } else { 0 };
                    if g < gQ || (g == gQ && anz > aQ) { gQ = g; aQ = anz; quelle = lab; }
                }
            }
            if quelle != 0 {
                let ab = brauche.min(aQ - 1);
                welt.SetLabScientistCount(quelle, aQ - ab);
                welt.RecalculateValues(sp);
                let lo = welt.Location(fehlt);
                let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
                melde(welt, z, "arbeit", ft, x, y, &format!("ohne Arbeiter -- Forschung gibt {} Wissenschaftler ab, die als Arbeiter einspringen (statt Stilllegen)", ab));
                return true;
            }
        }
    }
    false
}

/// Fuer ein wichtigeres Gebaeude ohne Arbeiter das unwichtigste laufende stilllegen
pub(super) fn fuerFehlendesStilllegen(welt: &mut dyn Welt, z: &mut Zustand, fehlt: Id, opfer: Id, rFehlt: i32, rOpfer: i32) -> bool {
    if fehlt != 0 && opfer != 0 && rOpfer + 5 < rFehlt && z.anzahlGeparkt < 16 {
        welt.DoIdle(opfer);
        let n = z.anzahlGeparkt as usize;
        z.geparktGrund[n] = 0;
        z.geparkt[n] = opfer; z.anzahlGeparkt += 1;
        let ot = welt.GetType(opfer);
        let lo = welt.Location(opfer);
        let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
        let ft = welt.GetType(fehlt);
        let fname = name(welt, ft);
        melde(welt, z, "arbeit", ot, x, y, &format!("stillgelegt (Rang {}), Arbeiter fuer {} (Rang {})", rOpfer, fname, rFehlt));
        return true;
    }
    false
}

/// Wichtigstes stillgelegtes, das mit den freien Arbeitern laufen kann, wieder einschalten
pub(super) fn geparktesWecken(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, sp: i32) {
    let (mut beste, mut rBeste) = (-1i32, -1);
    for i in 0..z.anzahlGeparkt as usize {
        if z.geparktGrund[i] != 0 { continue; } // Leerlauf: weckt der zustaendige Bereich
        let u: Id = z.geparkt[i];
        let ut = welt.GetType(u);
        let r = rang(welt, z, l, ut);
        if welt.GetPowerRequired(ut, sp) > l.stromFrei { continue; } // erst, wenn Strom da ist
        let frei = l.arbFrei - beansprucht(welt, l, M_ARB as i32, r + 1, -1);
        if welt.GetWorkersRequired(ut, sp) <= frei && r > rBeste { rBeste = r; beste = i as i32; }
    }
    if beste < 0 { return; }
    let b = beste as usize;
    let u: Id = z.geparkt[b];
    welt.DoUnIdle(u);
    let ut = welt.GetType(u);
    let lo = welt.Location(u);
    let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
    melde(welt, z, "arbeit", ut, x, y, &format!("wieder eingeschaltet (Rang {}, {} Arbeiter frei)", rBeste, l.arbFrei));
    z.anzahlGeparkt -= 1;
    let n = z.anzahlGeparkt as usize;
    z.geparkt[b] = z.geparkt[n]; z.geparktGrund[b] = z.geparktGrund[n];
}

/// C++: bool istGeparkt(const Zustand& z, int id)  (op2_ki.hpp:2479)
pub fn istGeparkt(_welt: &mut dyn Welt, z: &Zustand, id: i32) -> bool {
    for i in 0..z.anzahlGeparkt as usize { if z.geparkt[i] == id { return true; } }
    false
}

/// C++: void leerlaufWecken(Zustand& z, int id, const char* warum)  (op2_ki.hpp:2484)
pub fn leerlaufWecken(welt: &mut dyn Welt, z: &mut Zustand, id: i32, warum: &str) {
    for i in 0..z.anzahlGeparkt as usize {
        if z.geparkt[i] != id { continue; }
        let u: Id = id;
        let ut = welt.GetType(u);
        let braucht = welt.GetWorkersRequired(ut, z.spieler);
        if welt.GetNumAvailableWorkers(z.spieler) < braucht {
            // DoUnIdle wirkt nur mit genug freien Arbeitern -> Wunsch an die Rangliste, geparkt lassen
            z.wunschArbeiter = braucht; z.wunschRang = 70; z.wunschSeit = welt.Tick();
            return;
        }
        welt.DoUnIdle(u);
        let ut2 = welt.GetType(u);
        let lo = welt.Location(u);
        let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
        melde(welt, z, "arbeit", ut2, x, y, &format!("wieder eingeschaltet: {}", warum));
        // Leerlauf-Zaehler zuruecksetzen (sonst gilt es sofort wieder als "seit langem untaetig")
        for j in 0..z.anzahlLeer as usize {
            if z.leerId[j] == id {
                z.anzahlLeer -= 1;
                let n = z.anzahlLeer as usize;
                z.leerId[j] = z.leerId[n]; z.leerSeit[j] = z.leerSeit[n];
                break;
            }
        }
        z.anzahlGeparkt -= 1;
        let n = z.anzahlGeparkt as usize;
        z.geparkt[i] = z.geparkt[n]; z.geparktGrund[i] = z.geparktGrund[n];
        return;
    }
}

/// C++: void leerlauf(Zustand& z, const Lage& l)  (op2_ki.hpp:2505)
pub fn leerlauf(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    if standardVolk(welt, l) { return; }
    let t = welt.Tick();
    let mangel = l.arbFrei <= 0 || l.arbFehlt > 0 || l.wissFehlt > 0;
    // Untaetige Gebaeude erfassen
    let mut ids = [0i32; 16];
    let mut seit = [0i32; 16];
    let mut n = 0usize;
    let typen = [mapBasicLab, mapStandardLab, mapAdvancedLab, mapStructureFactory, mapSpaceport];
    for &typ in typen.iter() {
        for u in welt.PlayerBuildingEnum(z.spieler, typ) {
            if welt.GetCurAction(u) != moDone || welt.IsBusy(u) || istGeparkt(welt, z, u) { continue; } // arbeitet / im Bau / geparkt
            // Mit bekanntem Feind bleibt die Strukturfabrik an: Tuerme und Schmelzen muessen sofort nachgebaut werden koennen
            if typ == mapStructureFactory && z.bedrohung > 0 { continue; }
            if typ == mapStructureFactory {
                let mut offen = false;
                for i in 0..z.anzahlPlaetze as usize { if z.plaetze[i].zustand == P_FAHRT || z.plaetze[i].zustand == P_BAUT { offen = true; } }
                let mut kits = false;
                for bay in 0..6 { let k = welt.GetFactoryCargo(u, bay); if k > 0 && k != mapNone { kits = true; } }
                if offen || kits { continue; }
            }
            if typ == mapSpaceport && z.ziel == 1 && z.zielErreicht == 0 && hatTech(welt, z, TECH_RAUMFAHRT) { continue; } // gleich gebraucht
            if typ == mapSpaceport && !welt.IsEden(z.spieler) && hatTech(welt, z, 12201) { continue; } // baut EMP-Raketen
            if (typ == mapBasicLab || typ == mapStandardLab || typ == mapAdvancedLab) && welt.GetLabCurrentTopic(u) > 0 { continue; }
            // Ein Labor, das die Forschung braucht, ist nicht "untaetig" -- es wartet auf Wissenschaftler
            if (typ == mapStandardLab || typ == mapAdvancedLab) && z.laborNutzen[if typ == mapAdvancedLab { 3 } else { 2 }] > 0 { continue; }
            // merke(u)
            if n < 16 {
                let mut s0 = t;
                for i in 0..z.anzahlLeer as usize { if z.leerId[i] == u { s0 = z.leerSeit[i]; } }
                ids[n] = u; seit[n] = s0; n += 1;
            }
        }
    }
    for i in 0..n { z.leerId[i] = ids[i]; z.leerSeit[i] = seit[i]; }
    z.anzahlLeer = n as i32;
    if !mangel || z.anzahlGeparkt >= 16 { return; }
    for i in 0..n {
        if t - seit[i] < LEERLAUF_TICKS { continue; }
        let u: Id = ids[i];
        welt.DoIdle(u);
        let g = z.anzahlGeparkt as usize;
        z.geparktGrund[g] = 1;
        z.geparkt[g] = u; z.anzahlGeparkt += 1;
        let ut = welt.GetType(u);
        let lo = welt.Location(u);
        let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
        melde(welt, z, "arbeit", ut, x, y, &format!("Leerlauf seit {} Ticks bei Personalmangel -- stillgelegt", t - seit[i]));
        return; // eins je Runde
    }
}

/// C++: bool kitGesperrt(const Zustand& z, int typ)  (op2_ki.hpp:2556)
pub fn kitGesperrt(welt: &mut dyn Welt, z: &Zustand, typ: i32) -> bool {
    let t = welt.Tick();
    for i in 0..8 { if z.kitGesperrtTyp[i] == typ && z.kitGesperrtBis[i] > t { return true; } }
    false
}

/// C++: void kitSperren(Zustand& z, int typ, int dauer)  (op2_ki.hpp:2561)
pub fn kitSperren(welt: &mut dyn Welt, z: &mut Zustand, typ: i32, dauer: i32) {
    let mut frei = 0usize;
    for i in 0..8 { if z.kitGesperrtBis[i] < z.kitGesperrtBis[frei] { frei = i; } }
    z.kitGesperrtTyp[frei] = typ;
    z.kitGesperrtBis[frei] = welt.Tick() + dauer;
}
