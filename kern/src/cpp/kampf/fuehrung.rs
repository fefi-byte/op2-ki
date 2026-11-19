//! Kampffuehrung je SCAN_TAKT, aufgeteilt nach Schritten (Rust 2026-11-18; vorher eine Funktion kampfFuehrungInnen mit
//! 560 Zeilen, Verhalten unveraendert -- geprueft mit ki-test/gleich.sh). Reihenfolge der Schritte wie in der C++-KI.
use super::*;
use super::{GRUPPE_MAX, WERT_WT, WERT_LYNX, EMP_FLUG};

/// Warteplatz der Wache (zum Feind hin), Sammelplatz (hinter der Kolonie), Basisrechteck
#[derive(Clone, Copy)]
pub(super) struct Plaetze { pub wx: i32, pub wy: i32, pub sx: i32, pub sy: i32, pub basis: Rect }
/// Sollzahlen: Fahrgestell, Waffe, Wache und Sammelgruppe (Fahrzeuge)
#[derive(Clone, Copy)]
pub(super) struct Soll { pub fz: i32, pub waffe: i32, pub wacheZiel: i32, pub sammelZiel: i32 }
/// Bestand nach dem Einsammeln: Fahrzeuge in Wache (nw), Sammelgruppe (ns), Angriff (na) und Angriffswert (wa)
#[derive(Clone, Copy)]
pub(super) struct Bestand { pub nw: i32, pub ns: i32, pub na: i32, pub wa: i32 }

pub(super) fn kampfFuehrungInnen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32) {
    let pl = plaetzeBestimmen(welt, z);
    gruppenAnlegen(welt, z, &pl);
    fabrikGruppe(welt, z, l);
    let soll = sollzahlen(welt, z, l);
    beimischungSetzen(welt, z, l, &soll);
    reserveFuehren(welt, z);
    aussenpostenVerteilen(welt, z, t, &pl);
    let mut b = streunerAufnehmen(welt, z, t, &pl, &soll);
    // 3. Angriff
    let (feindWTWert, feindWTBasis) = feindTuerme(welt, z);
    verfolgen(welt, z);
    spinnenEinsatz(welt, z);
    if z.kampfModus >= 2 {
        bilanzKontakt(welt, z);
        let wg = z.wache; rueckzugPruefen(welt, z, wg);
    }
    if z.kampfModus == 3 || z.kampfModus == 4 { aufmarschUndRakete(welt, z, t, &mut b); return; }
    if z.kampfModus == 2 { angriffLaeuft(welt, z, &pl, &mut b); return; }
    super::zermuerben::neuerAngriff(welt, z, l, t, &soll, &mut b, feindWTWert, feindWTBasis);
}

/// Plaetze von Wache und Sammelgruppe
fn plaetzeBestimmen(welt: &mut dyn Welt, z: &mut Zustand) -> Plaetze {
    let (mut eux, mut euy) = (0.0f64, 0.0f64);
    feindRichtung(welt, z, z.bx, z.by, &mut eux, &mut euy);
    let (wx, wy) = ((z.bx as f64 + eux * 8.0 + 0.5) as i32, (z.by as f64 + euy * 8.0 + 0.5) as i32);   // Warteplatz der Wache zum Feind hin
    let sx = (z.bx as f64 - eux * 6.0 + 0.5) as i32; let sy = (z.by as f64 - euy * 6.0 + 0.5) as i32;   // Sammelplatz hinter der Kolonie
    let basis = Rect::new(engine(welt, z, z.bx - 16, z.by - 14), engine(welt, z, z.bx + 16, z.by + 14));
    Plaetze { wx, wy, sx, sy, basis }
}

/// Gruppen anlegen; alle 2000 Ticks Plaetze pruefen und verlegen
fn gruppenAnlegen(welt: &mut dyn Welt, z: &mut Zustand, pl: &Plaetze) {
    let Plaetze { wx, wy, sx, sy, basis } = *pl;
    let sp = z.spieler;
    if z.wacheAn == 0 { z.wache = welt.CreateFightGroup(z.spieler); z.wacheAn = 1; heim(welt, z, wx, wy, basis, None); }
    if z.sammelAn == 0 { z.sammel = welt.CreateFightGroup(z.spieler); z.sammelAn = 1; sammelPlatz(welt, z, sx, sy, basis); }
    // Die Basis waechst: alle 2000 Ticks pruefen, ob die Plaetze noch frei sind, sonst verlegen.
    if welt.Tick() % 2000 < TAKT {
        let mut hx = wx; let mut hy = wy; freierWarteplatz(welt, z, &mut hx, &mut hy, 3, true);
        if hx != platzLesen(sp, 0) || hy != platzLesen(sp, 1) { heim(welt, z, wx, wy, basis, Some("Warteplatz verlegt (alter Platz belegt)")); }
        let mut qx = sx; let mut qy = sy; freierWarteplatz(welt, z, &mut qx, &mut qy, 2, false);
        if qx != platzLesen(sp, 2) || qy != platzLesen(sp, 3) {
            sammelPlatz(welt, z, sx, sy, basis);
            melde(welt, z, "kampf", mapLynx, qx, qy, &format!("Sammelplatz verlegt nach {},{} (alter Platz belegt)", qx, qy));
        }
        if z.fabrikAn != 0 {
            let mut fx = z.bx; let mut fy = z.by + 7; freierWarteplatz(welt, z, &mut fx, &mut fy, 2, false);
            if fx != platzLesen(sp, 8) || fy != platzLesen(sp, 9) {
                platzSetzen(sp, 8, fx); platzSetzen(sp, 9, fy);
                let r = Rect::new(engine(welt, z, fx - 3, fy - 1), engine(welt, z, fx + 3, fy + 2)); welt.SetRect(z.fabrik, r);
            }
        }
    }
    if z.angriffAn == 0 { z.angriff = welt.CreateFightGroup(z.spieler); z.angriffAn = 1; }
    if z.angriff2An == 0 { z.angriff2 = welt.CreateFightGroup(z.spieler); z.angriff2An = 1; }
    if z.reserveAn == 0 { z.eingreif = welt.CreateFightGroup(z.spieler); z.reserveAn = 1; z.reserveOrt = -2; }
}

/// Fabrikgruppe mit der ersten fertigen Fahrzeugfabrik, weitere Fabriken dazu
fn fabrikGruppe(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    let sp = z.spieler;
    // Fabrikgruppe mit der (fertigen, ruhigen) Fahrzeugfabrik
    if l.vf != 0 && (z.fabrikAn == 0 || z.fabrikVf != l.vf) && bereit(welt, l.vf) {
        z.fabrik = welt.CreateBuildingGroup(z.spieler);
        let mut fx = z.bx; let mut fy = z.by + 7; freierWarteplatz(welt, z, &mut fx, &mut fy, 2, false);   // frei von Gebaeuden und Erzwegen
        platzSetzen(sp, 8, fx); platzSetzen(sp, 9, fy);
        let r = Rect::new(engine(welt, z, fx - 3, fy - 1), engine(welt, z, fx + 3, fy + 2));
        welt.SetRect(z.fabrik, r);
        welt.TakeUnit(z.fabrik, l.vf);
        welt.RecordVehReinforceGroup(z.fabrik, z.wache, 3000);
        welt.RecordVehReinforceGroup(z.fabrik, z.sammel, 1000);
        welt.RecordVehReinforceGroup(z.fabrik, z.eingreif, 500);
        z.fabrikAn = 1; z.fabrikVf = l.vf;
        for i in 0..MAX_VF { z.fabrikDrin[i] = 0; }
        z.fabrikDrin[0] = l.vf;
        let vl = welt.Location(l.vf); let (ex, ey) = (edX(welt, z, vl), edY(welt, z, vl));
        melde(welt, z, "kampf", mapVehicleFactory, ex, ey,
              "Fabrikgruppe angelegt: die Fahrzeugfabrik verstaerkt Wache (Prioritaet 3000) und Sammelgruppe (1000)");
    }
    // Weitere Fahrzeugfabriken (fertig, ruhig) in dieselbe Fabrikgruppe
    if z.fabrikAn != 0 {
        for j in 1..l.vfAnzahl.max(0) as usize {
            let v: Id = l.vfs[j];
            if !bereit(welt, v) { continue; }
            let mut frei: i32 = -1; let mut drin = false;
            for i in 0..MAX_VF {
                if z.fabrikDrin[i] == v { drin = true; }
                let alt: Id = z.fabrikDrin[i];
                if frei < 0 && (z.fabrikDrin[i] == 0 || !welt.IsLive(alt) || welt.GetType(alt) != mapVehicleFactory) { frei = i as i32; }
            }
            if drin || frei < 0 { continue; }
            welt.TakeUnit(z.fabrik, v);
            z.fabrikDrin[frei as usize] = v;
            let vl = welt.Location(v); let (ex, ey) = (edX(welt, z, vl), edY(welt, z, vl));
            melde(welt, z, "kampf", mapVehicleFactory, ex, ey, &format!("Weitere Fahrzeugfabrik in die Fabrikgruppe ({} Fabriken)", l.vfAnzahl));
        }
    }
}

/// Sollzahlen von Wache und Sammelgruppe aus Kampfwert-Bedarf, Erzvorrat und Zufluss; Reservegruppe
fn sollzahlen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) -> Soll {
    // Sollzahlen: Wache deckt den fehlenden Kampfwert (ohne Tuerme) plus die Aussenposten, Sammelgruppe die Angriffstruppe
    let fz = fahrgestellWahl(welt, z, l.selten);
    let mut wacheZiel = 0; let mut sammelZiel = 0; let mut waffe = mapNone;
    if fz != 0 {
        let fiSel = welt.GetRareOreCost(fz, z.spieler);
        waffe = waffeWahl(welt, z, fz, l.selten - fiSel);
        let wert = if sdkWertT(welt, fz, waffe) > 0 { sdkWertT(welt, fz, waffe) } else { WERT_LYNX };
        let turm = z.eigeneWTWert + (geplant(welt, z, mapGuardPost) + l.kitsImSpiel[mapGuardPost as usize] + l.imBau[mapGuardPost as usize]) * WERT_WT;
        let fehlt = z.sollArmee - turm;
        wacheZiel = if fehlt > 0 { (fehlt + wert - 1) / wert } else { 0 };
        for i in 0..6 { wacheZiel += aussenFahrzeuge(welt, z, i); }
        let angriffSoll = mobilSoll(welt, z, true) - mobilSoll(welt, z, false);
        sammelZiel = if angriffSoll > 0 { (angriffSoll + wert - 1) / wert } else { 0 };
        // Wehrloser Feind (keine Armee, keine Tuerme) -- daheim droht nichts, also kommt alles in die Angriffstruppe.
        // Sonst blieb sie bei 1 Lynx, alles Weitere ging in die Reserve, und der Angriff (braucht 2) kam nie.
        if z.feindGeb > 0 && z.feindArmee == 0 && z.feindWT == 0 { sammelZiel = sammelZiel.max(GRUPPE_MAX); }
        // Keine feste Obergrenze: vorhandene Fahrzeuge plus was Vorrat und Zufluss in der Reaktionszeit hergeben.
        let wacheOhne = wacheZiel; let sammelOhne = sammelZiel;
        let ks = welt.GetOreCost(fz, z.spieler) + welt.GetOreCost(waffe, z.spieler);
        let kosten = if ks > 0 { ks } else { 1 };
        let bl = bilanz(welt, z, l);
        let mut zufluss = erzRouten(welt, z) as f64 * bl.routeErz as f64 * 1000.0 / PAR(z.spieler).horizont;
        if z.zuflussErz > zufluss { zufluss = z.zuflussErz; }
        let neu = ((l.erz as f64 + zufluss * PAR(z.spieler).reaktion / 1000.0) / kosten as f64) as i32;
        let mut nw = 0; let mut ns = 0;
        gruppenWert(welt, z.wache, &mut nw); gruppenWert(welt, z.sammel, &mut ns);
        if wacheZiel > nw + neu { wacheZiel = nw + neu; }
        let rest = neu - if wacheZiel > nw { wacheZiel - nw } else { 0 };
        if sammelZiel > ns + if rest > 0 { rest } else { 0 } { sammelZiel = ns + if rest > 0 { rest } else { 0 }; }
        // Die Erz-Grenze bremst nur das Erhoehen: ein schon gesetztes Ziel faellt nicht.
        if wacheZiel < z.wacheZiel && wacheOhne >= z.wacheZiel { wacheZiel = z.wacheZiel; }
        if sammelZiel < z.sammelZiel && sammelOhne >= z.sammelZiel { sammelZiel = z.sammelZiel; }
        // Seltenerz-Schmelze geht vor neuen Kampffahrzeugen, solange keine akute Bedrohung besteht.
        let sparen = (l.seltenSpart || l.erzSpart) && z.bedrohung < 50;
        if sparen { if wacheZiel > nw { wacheZiel = nw; } if sammelZiel > ns { sammelZiel = ns; } }
        // Reserve: erst wenn Wache und Sammelgruppe stehen, und nur so viel, wie Vorrat und Zufluss hergeben
        {
            let mut nr = 0; gruppenWert(welt, z.eingreif, &mut nr);
            let mut rz = nr;
            if !sparen && nw >= wacheZiel && ns >= sammelZiel { rz = nr + neu; if rz > GRUPPE_MAX { rz = GRUPPE_MAX; } }
            if rz != z.reserveZiel { welt.ClearTargCount(z.eingreif); if rz > 0 { welt.SetTargCount(z.eingreif, fz, waffe, rz); } z.reserveZiel = rz; }
        }
        // Eine Gruppe fasst 32, die Fabrik haelt dort an.
        { if wacheZiel > GRUPPE_MAX { wacheZiel = GRUPPE_MAX; } if sammelZiel > GRUPPE_MAX { sammelZiel = GRUPPE_MAX; } }
    }
    Soll { fz, waffe, wacheZiel, sammelZiel }
}

/// EMP-Beimischung der Angriffstruppe; Sollzahlen an die Gruppen geben, wenn sich etwas aendert
fn beimischungSetzen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, soll: &Soll) {
    let Soll { fz, waffe, wacheZiel, sammelZiel } = *soll;
    let mut empN = 0;
    let mut empFz = fz;
    // EMP-Anteil q und EMP-Traeger nach Staerke je Erz (Seltenerz doppelt) der ganzen Angriffstruppe:
    // ((1-q) g x (1 + a q) + q g_EMP) / ((1-q) Kosten + q Kosten_EMP), q in 0 .. 50 % (Arena 8: a je Hauptwaffe).
    if fz != 0 && sammelZiel >= 2 && waffe != mapEMP && einheitErforscht(welt, z, mapEMP) {
        let s = z.spieler;
        let kosten = |welt: &mut dyn Welt, f: i32, w: i32| (welt.GetOreCost(f, s) + welt.GetOreCost(w, s) + 2 * (welt.GetRareOreCost(f, s) + welt.GetRareOreCost(w, s))).max(1) as f64;
        let gm = gegenwert(welt, s, fz, waffe, 1.0); let km = kosten(welt, fz, waffe);
        let a = empVerstaerkung(waffe);
        let mut best = gm / km; let mut bestQ = 0.0;
        for ec in [mapLynx, fz] {
            if !einheitErforscht(welt, z, ec) || welt.GetRareOreCost(ec, s) + welt.GetRareOreCost(mapEMP, s) > l.selten { continue; }
            let ge = gegenwert(welt, s, ec, mapEMP, 1.0); let ke = kosten(welt, ec, mapEMP);
            for i in 1..=5 {
                let q = i as f64 / 10.0;
                let v = ((1.0 - q) * gm * (1.0 + a * q) + q * ge) / ((1.0 - q) * km + q * ke);
                if v > best { best = v; bestQ = q; empFz = ec; }
            }
        }
        empN = (bestQ * sammelZiel as f64 + 0.5) as i32;
        if empN == 0 { empFz = fz; }
    }
    if z.wirtVorrang != 0 {
        // fabrikWirtschaft: die Fabrik baut gleich ein Wirtschaftsfahrzeug -- keine Kampf-Auftraege; -2 erzwingt das Neusetzen danach
        if z.wacheZiel != -2 { welt.ClearTargCount(z.wache); welt.ClearTargCount(z.sammel); z.wacheZiel = -2; }
    } else if fz != z.zielFz || waffe != z.zielWaffe || wacheZiel != z.wacheZiel || sammelZiel != z.sammelZiel || empN != z.zielEmp || (empN > 0 && empFz != z.zielEmpFz) {
        welt.ClearTargCount(z.wache); welt.ClearTargCount(z.sammel);
        if fz != 0 && wacheZiel > 0 { welt.SetTargCount(z.wache, fz, waffe, wacheZiel); }
        if fz != 0 && sammelZiel > 0 { welt.SetTargCount(z.sammel, fz, waffe, sammelZiel - empN); }
        if fz != 0 && (empN > 0 || z.zielEmp > 0) { welt.SetTargCount(z.sammel, empFz, mapEMP, empN); }
        if empN != z.zielEmp || (empN > 0 && empFz != z.zielEmpFz) {
            let ne = name(welt, empFz);
            melde(welt, z, "kampf", fz, z.bx, z.by, &format!("EMP-Beimischung: {} von {} Fahrzeugen der Angriffstruppe ({})", empN, sammelZiel, ne));
        }
        z.zielEmp = empN; z.zielEmpFz = empFz;
        let (nf, nwf) = (name(welt, fz), name(welt, waffe));
        let ms = mobilSoll(welt, z, true) - mobilSoll(welt, z, false);
        melde(welt, z, "kampf", if fz != 0 { fz } else { mapLynx }, z.bx, z.by,
              &format!("Sollzahlen: Wache {}, Sammelgruppe {} {} mit {} (Soll {}, Tuerme {}, Angriffstruppe {})",
                       wacheZiel, sammelZiel, nf, nwf, z.sollArmee, z.eigeneWTWert, ms));
        z.zielFz = fz; z.zielWaffe = waffe; z.wacheZiel = wacheZiel; z.sammelZiel = sammelZiel;
    }
}

/// Aussenposten: Fahrzeuge aus der Wache abziehen bzw. zurueckgeben
fn aussenpostenVerteilen(welt: &mut dyn Welt, z: &mut Zustand, t: i32, pl: &Plaetze) {
    let Plaetze { wx, wy, basis, .. } = *pl;
    // 1. Aussenposten: Fahrzeuge aus der Wache abziehen bzw. zurueckgeben (nur lebende)
    for i in 0..6usize {
        let soll = aussenFahrzeuge(welt, z, i as i32);
        if z.aussenAn[i] == 0 && soll == 0 { continue; }
        if z.aussenAn[i] == 0 { z.aussenGruppe[i] = welt.CreateFightGroup(z.spieler); z.aussenAn[i] = 1; }
        let mut n = 0; let mut nw = 0;
        gruppenWert(welt, z.aussenGruppe[i], &mut n);
        gruppenWert(welt, z.wache, &mut nw);
        let mut geaendert = false;
        while n > soll {
            let mut v: Id = 0; let mut da = false;
            for u in welt.GroupEnumerator(z.aussenGruppe[i]) { v = u; if welt.IsLive(u) && kaempfer(welt, u) { da = true; break; } }
            if !da { break; }
            if welt.TotalUnitCount(z.wache) >= GRUPPE_MAX { break; }
            welt.RemoveUnit(z.aussenGruppe[i], v); welt.TakeUnit(z.wache, v); n -= 1; geaendert = true;
        }
        while n < soll && nw > 1 {
            let mut v: Id = 0; let mut da = false;
            for u in welt.GroupEnumerator(z.wache) { v = u; if welt.IsLive(u) && kaempfer(welt, u) { da = true; break; } }
            if !da { break; }
            if welt.TotalUnitCount(z.aussenGruppe[i]) >= GRUPPE_MAX { break; }
            welt.RemoveUnit(z.wache, v); welt.TakeUnit(z.aussenGruppe[i], v); n += 1; nw -= 1; geaendert = true;
        }
        if geaendert {
            let r = Rect::new(engine(welt, z, z.aussenX[i] - 2, z.aussenY[i] - 2), engine(welt, z, z.aussenX[i] + 2, z.aussenY[i] + 2));
            let g = Rect::new(engine(welt, z, z.aussenX[i] - 7, z.aussenY[i] - 7), engine(welt, z, z.aussenX[i] + 7, z.aussenY[i] + 7));
            wachen(welt, z.aussenGruppe[i], r, &[g], 1);
            heim(welt, z, wx, wy, basis, None);
            let grund = if z.aussenAngriff[i] != 0 && t - z.aussenAngriff[i] < 10000 { "wurde angegriffen" } else if soll != 0 { "naeher am Feind als die Basis" } else { "keine Gefahr mehr" };
            melde(welt, z, "kampf", mapLynx, z.aussenX[i], z.aussenY[i],
                  &format!("Aussenposten bei {},{}: {} Fahrzeug(e) ({})", z.aussenX[i], z.aussenY[i], n, grund));
        }
        if z.aussenId[i] == 0 && n == 0 { z.aussenAn[i] = 0; }
    }
}

/// Kampffahrzeuge ohne Gruppe zur Wache; Bestand der Gruppen zaehlen
fn streunerAufnehmen(welt: &mut dyn Welt, z: &mut Zustand, t: i32, pl: &Plaetze, soll: &Soll) -> Bestand {
    let Plaetze { wx, wy, basis, .. } = *pl;
    let Soll { fz, wacheZiel, sammelZiel, .. } = *soll;
    // 2. Kampffahrzeuge ohne Gruppe (z.B. nach Neuanlage) zur Wache -- nur fertige, ruhige
    let mut neu = 0; let mut alle = 0;
    for u in welt.PlayerVehicleEnum(z.spieler) {
        if !welt.IsLive(u) || !kaempfer(welt, u) { continue; }
        alle += 1;
        if u == z.garDran || u == z.garRaus { continue; }   // gehoert gerade der Garage
        if inGruppeG(welt, z.wache, u) || inGruppeG(welt, z.sammel, u) || inGruppeG(welt, z.angriff, u) || inGruppeG(welt, z.angriff2, u) || inGruppeG(welt, z.eingreif, u) { continue; }
        let mut aussen = false;
        for i in 0..6 { if aussen { break; } if z.aussenAn[i] != 0 && inGruppeG(welt, z.aussenGruppe[i], u) { aussen = true; } }
        if aussen || welt.GetCurAction(u) != moDone || welt.IsBusy(u) { continue; }
        let mut aufGebaeude = false;
        for o in welt.LocationEnumerator(welt.Location(u)) { if welt.IsBuilding(o) { aufGebaeude = true; break; } }
        if aufGebaeude { continue; }
        if sicherNehmen(welt, z, z.wache, u) || sicherNehmen(welt, z, z.eingreif, u) { neu += 1; }
    }
    if alle > z.kampfFzAnzahl { z.letztesKampfFz = t; }                 // fuer die Selbstpruefung
    z.kampfFzAnzahl = alle;
    if neu != 0 { heim(welt, z, wx, wy, basis, Some("Kampffahrzeuge ohne Gruppe aufgenommen")); }
    let mut nw = 0; let mut ns = 0; let mut na = 0;
    gruppenWert(welt, z.wache, &mut nw);
    gruppenWert(welt, z.sammel, &mut ns);
    let wa = angriffWert(welt, z, &mut na);
    z.wacheAnzahl = nw;
    { let mut nr = 0; gruppenWert(welt, z.eingreif, &mut nr); z.gruppenBedarf = if fz != 0 && (nw < wacheZiel || ns < sammelZiel || nr < z.reserveZiel) { 1 } else { 0 }; }
    Bestand { nw, ns, na, wa }
}

/// Feindliche Tuerme: Wert aller und Wert der Tuerme, die die Feindbasis decken (15 Kacheln)
fn feindTuerme(welt: &mut dyn Welt, z: &mut Zustand) -> (i32, i32) {
    let mut feindWTWert = 0;
    let mut feindWTBasis = 0;
    {                                                  // echte Turmwerte statt Schaetzung
        let mut p = 0;
        while p < welt.NoPlayers() {
            if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { p += 1; continue; }
            for g in welt.PlayerBuildingEnum(p, mapGuardPost) {
                if !welt.IsLive(g) { continue; }
                let w = kampfWert(welt, g);
                feindWTWert += w;
                // Fuer den Basisangriff zaehlen nur Tuerme, die die Basis decken (15 Kacheln um ihre
                // Mitte, wie feindArmeeZurueck) -- Tuerme an fernen Aussenposten oder Paessen stehen nicht im Weg.
                let gl = welt.Location(g);
                let (gx, gy) = (edX(welt, z, gl), edY(welt, z, gl));
                if abstand2(welt, gx, gy, z.feindX, z.feindY) <= 15 * 15 { feindWTBasis += w; }
            }
            p += 1;
        }
    }
    (feindWTWert, feindWTBasis)
}

/// Modus 3 (Aufmarsch) und 4 (EMP-Rakete im Flug)
fn aufmarschUndRakete(welt: &mut dyn Welt, z: &mut Zustand, t: i32, b: &mut Bestand) {
    let na = b.na; let mut nw = b.nw;
    // Aufmarsch / Rakete im Flug. Bricht etwas weg, wird daraus ein normaler Angriff (Modus 2).
    let a = gruppenWert(welt, z.wache, &mut nw); let b = gruppenWert(welt, z.eingreif, &mut nw);
    let kolonieInNot = z.feindDrohung > z.eigeneWTWert + a + b;
    if na == 0 || z.feindGeb == 0 || kolonieInNot { z.kampfModus = 2; return; }
    if z.kampfModus == 3 {
        let mut dort = 0;
        for v in AngriffEnum(welt, z) {
            if welt.IsLive(v) { let lo = welt.Location(v); let (ex, ey) = (edX(welt, z, lo), edY(welt, z, lo)); if abstand2(welt, ex, ey, z.aufmarschX, z.aufmarschY) <= 5 * 5
                || (z.aufmarsch2X != 0 && abstand2(welt, ex, ey, z.aufmarsch2X, z.aufmarsch2Y) <= 5 * 5) { dort += 1; } }
        }
        // Frist nach der Entfernung (Lynx ~50 Ticks je Kachel im Pulk)
        let mut aufD = abstand2(welt, z.bx, z.by, z.aufmarschX, z.aufmarschY);
        if z.aufmarsch2X != 0 { aufD = aufD.max(abstand2(welt, z.bx, z.by, z.aufmarsch2X, z.aufmarsch2Y)); }
        let aufFrist = 1000 + (70.0 * (aufD as f64).sqrt()) as i32;
        if dort * 10 >= na * 7 || t - z.aufmarschSeit > aufFrist {
            let mut sp: Id = 0;
            for u in welt.PlayerBuildingEnum(z.spieler, mapSpaceport) { if welt.IsLive(u) { sp = u; break; } }
            if z.empRakete == 0 {                                            // ohne Rakete: die Truppe steht, geschlossen los
                melde(welt, z, "kampf", mapLynx, z.aufmarschX, z.aufmarschY, &format!("Truppe steht ({} von {} nach {} Ticks) -- Angriff", dort, na, t - z.aufmarschSeit));
                z.kampfModus = 2; z.angriffZiel = 0; let (ax, ay) = (z.aufmarschX, z.aufmarschY); angriffZielSetzen(welt, z, ax, ay); return;
            }
            let wert = { let (mut ex, mut ey) = (z.empZielX, z.empZielY); let w = empZiel(welt, z, &mut ex, &mut ey); z.empZielX = ex; z.empZielY = ey; w };   // Ziel neu bestimmen
            // Start nur, wenn das Ziel die Rakete noch wert ist -- sonst ohne EMP angreifen
            if sp == 0 || z.empRakete == 0 || (wert as f64) < empKostenWert(welt, z) as f64 * PAR(z.spieler).empSchwelle {
                melde(welt, z, "kampf", mapLynx, z.aufmarschX, z.aufmarschY, &format!("Aufmarsch steht -- Ziel ist die Rakete nicht wert (Abwehr {}), Angriff ohne EMP", wert));
                z.kampfModus = 2; let (ax, ay) = (z.aufmarschX, z.aufmarschY); angriffZielSetzen(welt, z, ax, ay); return;
            }
            let ziel = engine(welt, z, z.empZielX, z.empZielY);
            welt.DoLaunch(sp, ziel.x * 32 + 16, ziel.y * 32 + 16, true);
            z.empRakete = 0; z.empStart = t; z.kampfModus = 4;
            melde(welt, z, "kampf", mapEMPMissile, z.empZielX, z.empZielY,
                  &format!("EMP-Rakete gestartet auf {},{} (Abwehr Wert {}, {} von {} Fahrzeugen am Aufmarschpunkt)", z.empZielX, z.empZielY, wert, dort, na));
        }
        return;
    }
    // Modus 4: nach dem Einschlag angreifen, waehrend die Abwehr gelaehmt ist
    if t - z.empStart >= EMP_FLUG {
        let mut gelaehmt = 0;
        let mut p = 0;
        while p < welt.NoPlayers() {
            if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { p += 1; continue; }
            for u in welt.PlayerVehicleEnum(p) { if welt.IsLive(u) && welt.isEMPed(u) { gelaehmt += 1; } }
            p += 1;
        }
        melde(welt, z, "kampf", mapLynx, z.empZielX, z.empZielY, &format!("EMP eingeschlagen ({} feindliche Fahrzeuge gelaehmt) -- Angriff", gelaehmt));
        z.kampfModus = 2; z.angriffZiel = 0;
        let (ax, ay) = (z.aufmarschX, z.aufmarschY); angriffZielSetzen(welt, z, ax, ay);
    }
    return;
}

/// Modus 2: laufender Angriff -- Abbruch pruefen, Route abfahren, Ziel setzen
fn angriffLaeuft(welt: &mut dyn Welt, z: &mut Zustand, pl: &Plaetze, b: &mut Bestand) {
    let Plaetze { wx: _, wy: _, basis: _, .. } = *pl;
    let (wa, na) = (b.wa, b.na); let mut nw = b.nw;
    bilanzAnkunft(welt, z);
    // kein Abbruch bei halber Staerke -- gekaempft wird bis zum Ende
    let _ = wa;
    let abbruch = abbruchGrund(welt, z, na, &mut nw);
    if let Some(abbruch) = abbruch { angriffBeenden(welt, z, na, pl, abbruch); return; }
    let (mut mx, mut my, mut k) = (0, 0, 0);
    for v in AngriffEnum(welt, z) { if welt.IsLive(v) { let lo = welt.Location(v); mx += edX(welt, z, lo); my += edY(welt, z, lo); k += 1; } }
    if k != 0 { angriffZielSetzen(welt, z, mx / k, my / k); }
    return;
}

/// Grund, den laufenden Angriff abzubrechen (Feindbasis weg, Gruppe aufgerieben, Kolonie in Not)
fn abbruchGrund(welt: &mut dyn Welt, z: &mut Zustand, na: i32, nw: &mut i32) -> Option<&'static str> {
    let mut abbruch: Option<&'static str> = None;
    if z.feindGeb == 0 { abbruch = Some("Feindbasis vernichtet"); }
    else if na == 0 { abbruch = Some("Angriffsgruppe aufgerieben"); }
    else {
        let a = gruppenWert(welt, z.wache, nw); let b = gruppenWert(welt, z.eingreif, nw);
        if z.feindDrohung > z.eigeneWTWert + a + b { abbruch = Some("Kolonie wird angegriffen, Wache und Tuerme zu schwach"); }
    }
    abbruch
}

/// Angriff beenden: Bilanz lernen (Zermuerbungsabstand), Gruppe heim
fn angriffBeenden(welt: &mut dyn Welt, z: &mut Zustand, na: i32, pl: &Plaetze, abbruch: &'static str) {
    let Plaetze { wx, wy, basis, .. } = *pl;
    // Lernen: was hat dieser Angriff gebracht und gekostet?
    let zer = z.zerstoertWert - z.angriffZerstoertStart; let ver = z.verlorenWert - z.angriffVerlorenStart;
    let quote = zer as f64 / (if ver > 0 { ver } else { 1 }) as f64;
    let art = z.angriffArt;
    let au = art as usize;
    z.angriffQuote[au] = z.angriffQuote[au] * 0.7 + (if quote > 5.0 { 5.0 } else { quote }) * 0.3;
    if art == 2 {
        let alt = z.zermuerbAbstand;
        z.zermuerbAbstand = if z.angriffQuote[2] < 0.7 { z.zermuerbAbstand * 3 / 2 } else { z.zermuerbAbstand * 4 / 5 };
        if z.zermuerbAbstand < 4000 { z.zermuerbAbstand = 4000; }
        if z.zermuerbAbstand > 30000 { z.zermuerbAbstand = 30000; }
        melde(welt, z, "lernen", mapLynx, z.bx, z.by, &format!("Zermuerben: zerstoert {}, verloren {} (Quote {:.2}, gleitend {:.2}) -- Abstand {} -> {} Ticks",
              zer, ver, quote, z.angriffQuote[2], alt, z.zermuerbAbstand));
    } else {
        melde(welt, z, "lernen", mapLynx, z.bx, z.by, &format!("{}: zerstoert {}, verloren {} (Quote {:.2}, gleitend {:.2})",
              if art == 1 { "Zielangriff" } else { "Angriff auf die Basis" }, zer, ver, quote, z.angriffQuote[au]));
    }
    bilanzEnde(welt, z, zer, ver, abbruch);
    z.naechstesSpeichern = 0;
    if na != 0 && { let a = sicherAlle(welt, z, z.wache, z.angriff); let b = sicherAlle(welt, z, z.wache, z.angriff2); a + b > 0 } {   // kein Platz in der Wache: heim
        let daheim = engine(welt, z, z.bx, z.by + 4);
        for v in AngriffEnum(welt, z) { if welt.IsLive(v) && welt.IsVehicle(v) { welt.DoMove(v, daheim); } }
    }
    z.kampfModus = 0; z.angriffWunsch = 0;
    heim(welt, z, wx, wy, basis, Some(abbruch));
}

