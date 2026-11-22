//! Ausbildung an der Universitaet (Wissenschaftler), mit PAR wissArbeit weniger, solange Arbeiter fehlen.
use super::*;
use super::super::*;
use super::{TECH_RAUMFAHRT, WERT_WT, WERT_LYNX, EMP_TECH, GEBAEUDE};

/// C++: void ausbildung(Zustand& z, const Lage& l)  (op2_ki.hpp:6174)
// Universitaet: Wissenschaftler ausbilden; die Ausbildung ist so wichtig wie das, wofuer sie ausbildet.
pub fn ausbildung(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    if standardVolk(welt, l) { return; }
    let wissBedarf = beansprucht(welt, l, M_WISS as i32, 0, -1);
    let fuerGebaeude = wissBedarf > l.wissFrei;
    let mut luecke = 0;
    let mut plaetze = 0;
    let mut rangLabor = 0;
    forschungsLuecke(welt, z, l, &mut luecke, &mut plaetze, &mut rangLabor);
    let rangForschung = if plaetze > 0 { rangLabor * luecke / plaetze } else { 0 };
    let ausbau = PAR(z.spieler).wissAusbau >= 0.5;
    let mut bedarf = fuerGebaeude || ((l.wissFrei == 0 || ausbau) && luecke > l.wissFrei);
    // Arbeiter, die unbesetzte Gebaeude brauchen (wissAusbau: nur so viele werden fuer die Industrie freigehalten)
    let mut fuerIndustrie = 0;
    // Rust (PAR wissArbeit): fuer die Forschung nur ausbilden, wenn kein Gebaeude auf Arbeiter wartet (unbesetzt oder
    // offener Arbeiterwunsch) -- die Industrie geht vor (Nutzer 2026-11-17: keine Wissenschaftler ausgebildet).
    if !fuerGebaeude && kampfModellNeu() && PAR(z.spieler).wissArbeit >= 0.5 {
        let wunsch = z.wunschArbeiter > 0 && welt.Tick() - z.wunschSeit < 1000;
        let unbesetzt = (0..128).any(|t| l.unbesetzt[t] > 0 && welt.GetWorkersRequired(t as i32, z.spieler) > 0);
        if ausbau {
            fuerIndustrie = (0..128).map(|t| l.unbesetzt[t] * welt.GetWorkersRequired(t as i32, z.spieler).max(0)).sum::<i32>()
                + if wunsch { z.wunschArbeiter } else { 0 };
        } else if wunsch || unbesetzt { bedarf = false; }
    }
    let rangAusb = if fuerGebaeude && hoechsterAnspruch(welt, l, M_WISS as i32) > rangForschung { hoechsterAnspruch(welt, l, M_WISS as i32) } else { rangForschung };
    let uebrig = l.arbFrei - beansprucht(welt, l, M_ARB as i32, rangAusb, -1) - fuerIndustrie;
    if !bedarf || uebrig < 3 { return; }
    for uni in welt.PlayerBuildingEnum(z.spieler, mapUniversity) {
        if welt.GetCurAction(uni) != moDone || welt.IsBusy(uni) || !welt.HasScientists(uni) || welt.GetWorkersInTraining(uni) > 0 { continue; }
        let mut n = uebrig - 2;
        if n > 5 { n = 5; }
        if n <= 0 { return; }
        welt.DoTrainScientists(uni, n);
        let lo = welt.Location(uni);
        let x = edX(welt, z, lo);
        let y = edY(welt, z, lo);
        melde(welt, z, "ausbildung", mapUniversity, x, y,
            &format!("{} Arbeiter werden zu Wissenschaftlern (Rang {}; frei: {} Arbeiter, {} Wissenschaftler; Gebaeude brauchen {} Wiss.)",
                n, rangAusb, l.arbFrei, l.wissFrei, wissBedarf));
        return;
    }
}
