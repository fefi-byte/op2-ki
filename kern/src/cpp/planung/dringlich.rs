//! Dringlichkeit der Bereiche (Kuchenanteile), Moralzahl, erwartete Fabrik/Seltenerz, Notfall.
use super::*;
use super::{B_ANZAHL, TECH_SELTENERZ, snp, ix};

/// C++: int moralZahl(const Zustand& z)  (op2_ki.hpp:2041)
pub fn moralZahl(welt: &mut dyn Welt, z: &Zustand) -> i32 {
    // C++: if (isInited && playerArray) { r[16] in 0..100 -> return r[16]; } (Welt::MoraleRoh)
    let r = welt.MoraleRoh(z.spieler);
    if (0..=100).contains(&r) { return r; }
    100 - 25 * welt.MoraleLevel(z.spieler)
}

pub(super) fn grenze(v: i32) -> i32 { if v < 0 { 0 } else if v > 100 { 100 } else { v } }
pub(super) fn kol(d: &mut Dringlichkeit, v: i32, g: &'static str) { let v = grenze(v); if v > d.kolonie { d.kolonie = v; d.kolonieGrund = g; } }
pub(super) fn wir(d: &mut Dringlichkeit, v: i32, g: &'static str) { let v = grenze(v); if v > d.wirtschaft { d.wirtschaft = v; d.wirtschaftGrund = g; } }

/// C++: Dringlichkeit dringlichkeit(const Zustand& z, const Lage& l)  (op2_ki.hpp:2052)
pub fn dringlichkeit(welt: &mut dyn Welt, z: &Zustand, l: &Lage) -> Dringlichkeit {
    let mut d = Dringlichkeit { kolonie: 0, ziel: 0, wirtschaft: 0, verteidigung: 0, kampfForschung: 0, kolonieGrund: "-", wirtschaftGrund: "-" };
    let par_ = PAR(z.spieler);
    let bew = if l.menschen > 0 { l.menschen } else { 1 };
    if hatGeb(welt, z, l, mapNursery) == 0 { kol(&mut d, 100, "kein Kindergarten -- keine Geburten"); }
    if hatGeb(welt, z, l, mapUniversity) == 0 { let v = if welt.Kids(z.spieler) > 0 { 80 } else { 40 }; kol(&mut d, v, "keine Universitaet -- Kinder werden nicht ausgebildet"); }
    if !standardVolk(welt, l) {
        if l.wohnraum <= 0 { kol(&mut d, 100, "kein Wohnraum"); }
        else { let g = par_.wohnraumGrenze as i32; kol(&mut d, (l.menschen * 100 / l.wohnraum - g) * 100 / (if 100 - g > 1 { 100 - g } else { 1 }), "Wohnraum-Auslastung"); }
        kol(&mut d, 100 - l.medKap * 100 / bew, "Medizin-Abdeckung");
        if kitMoeglich(welt, z, mapRecreationFacility) || hatGeb(welt, z, l, mapRecreationFacility) != 0 {
            kol(&mut d, 100 - l.freizeitKap * 100 / bew, "Freizeit-Abdeckung");
        }
        let mz = moralZahl(welt, z);
        kol(&mut d, 100 - mz, "Moral");
    }
    {
        // Ein Engpass zaehlt nur, wenn es eine Massnahme dagegen gibt (Schmelze, Seltenerz-Schmelze, neue Mine)
        let bl = bilanz(welt, z, l);
        let ew = erzWahl(welt, z, l, &bl);
        // ... und nur, wenn sich die Massnahme im Horizont bezahlt (dasselbe Kriterium wie beim Bauen)
        if ew.nutzen >= par_.schmelzeNutzen && ew.typ == mapCommonOreSmelter && bl.bedarfErz > 0 { wir(&mut d, bl.engpassErz * 100 / bl.bedarfErz, "Erz-Engpass, Schmelze lohnt sich"); }
        if ew.nutzen >= par_.schmelzeNutzen && ew.typ == mapRareOreSmelter && bl.bedarfSelten > 0 { wir(&mut d, bl.engpassSelten * 100 / bl.bedarfSelten, "Seltenerz-Engpass, Seltenerz-Schmelze lohnt sich"); }
        // Neue Mine: ab PAR schmelzenJeMine Schmelzen je Mine
        if l.geb[ix(mapCommonOreMine)] > 0 && l.geb[ix(mapCommonOreSmelter)] as f64 >= par_.schmelzenJeMine * l.geb[ix(mapCommonOreMine)] as f64
            && l.geb[ix(mapCommonOreMine)] < MAX_MINEN as i32 && bl.engpassErz > 0 && bl.bedarfErz > 0 && l.vf != 0 {
            wir(&mut d, bl.engpassErz * 100 / bl.bedarfErz / 2, "Erz-Engpass, Minen ausgelastet -- neue Mine");
        }
    }
    if hatGeb(welt, z, l, mapVehicleFactory) == 0 && kitMoeglich(welt, z, mapVehicleFactory) {
        let mut w: &'static str = "";
        let fb = fahrzeugBedarf(welt, z, l, &mut w);
        wir(&mut d, fb, "Fahrzeuge gebraucht, keine Fahrzeugfabrik");
    }
    if l.erzMax > 0 && l.vorErz > l.erzMax { wir(&mut d, (l.vorErz - l.erzMax) * 100 / l.vorErz, "Lager zu klein fuer ein Vorhaben"); }
    if z.laborWunsch != 0 && hatGeb(welt, z, l, z.laborWunsch) == 0 && kitMoeglich(welt, z, z.laborWunsch) { wir(&mut d, z.laborWert, "Labor fehlt fuer die Forschung"); }
    {
        let fehlt = kampfFehlt(welt, z, l);
        if fehlt > 0 && kitMoeglich(welt, z, mapGuardPost) {
            let anteil = fehlt * 100 / (if z.sollArmee > 0 { z.sollArmee } else { fehlt });
            d.verteidigung = grenze(z.bedrohung * (if anteil > 100 { 100 } else { anteil }) / 100);
        }
        if aussenTurmFehlt(welt, z) >= 0 && kitMoeglich(welt, z, mapGuardPost) && z.bedrohung > d.verteidigung { d.verteidigung = z.bedrohung; }
    }
    // Militaerforschung: Bedrohung -- oder Blockade
    d.kampfForschung = z.bedrohung;
    // Vorsorge: ohne erforschtes Kampffahrzeug ein Mindestanteil, sobald eine Feindkolonie bekannt ist
    if z.feindGeb > 0 && fahrgestellWahl(welt, z, 1 << 20) == 0 && par_.vorsorge > d.kampfForschung as f64 { d.kampfForschung = par_.vorsorge as i32; }
    {
        // Qualitaet: ist das staerkste gesehene Feindfahrzeug besser als unser bestes baubares, waechst die Militaerforschung mit der Luecke
        let fz = fahrgestellWahl(welt, z, 1 << 20);
        let eigen = if fz != 0 { let wf = waffeWahl(welt, z, fz, 1 << 20); sdkWertT(welt, fz, wf) } else { 0 };
        if z.feindBestWert > eigen {
            let v = if eigen > 0 { (z.feindBestWert - eigen) * 100 / eigen } else { 100 };
            if v > d.kampfForschung { d.kampfForschung = if v > 100 { 100 } else { v }; }
        }
    }
    // Proaktiv aufruesten: Anteil des Feind-Potenzials an der Summe aus Potenzial und eigener Staerke
    if z.feindPotenzial > 0 {
        let v = z.feindPotenzial * 100 / (z.feindPotenzial + z.eigeneArmee);
        if v > d.kampfForschung { d.kampfForschung = v; }
    }
    let mut blockiert = false;
    {
        let fehlt = kampfFehlt(welt, z, l);
        if fehlt > 0 && !kitMoeglich(welt, z, mapGuardPost) && fahrgestellWahl(welt, z, 1 << 20) == 0 {
            blockiert = true;
            let luecke = grenze(fehlt * 100 / (if z.sollArmee > 0 { z.sollArmee } else { fehlt }));
            // Hat der Feind Fabriken oder schon eine Armee, kann er jederzeit kommen
            let naehe = if z.feindFabriken != 0 || z.feindArmee != 0 { 100 } else { 50 };
            let v = luecke * naehe / 100;
            if v > d.kampfForschung { d.kampfForschung = v; }
        }
    }
    if z.ziel >= 1 && z.zielErreicht == 0 {
        let mut m = d.kolonie; if d.wirtschaft > m { m = d.wirtschaft; } if d.verteidigung > m { m = d.verteidigung; }
        if blockiert && d.kampfForschung > m { m = d.kampfForschung; } // Blockade der Verteidigung geht dem Ziel vor
        let rest = 100 - m;
        // Satellit: eigener Bereich. Bevoelkerung: der Rest geht an die Kolonie. Vernichtung: an Militaer und Angriffstruppe.
        if z.ziel == 1 { d.ziel = rest; }
        else if z.ziel == 2 { if rest > d.kolonie { d.kolonie = rest; d.kolonieGrund = "Ziel Bevoelkerung"; } }
        else if z.ziel == 3 { if rest > d.kampfForschung { d.kampfForschung = rest; } if rest > d.verteidigung && z.feindGeb != 0 { d.verteidigung = rest / 2; } }
    }
    d
}

/// C++: void bauKuchen(const Zustand& z, const Lage& l, int* anteil)  (op2_ki.hpp:2152)
pub fn bauKuchen(welt: &mut dyn Welt, z: &Zustand, l: &Lage, anteil: &mut [i32]) {
    // Anteile = Dringlichkeiten (siehe dringlichkeit()), normiert auf 100 -- keine Grundanteile mehr
    let d = dringlichkeit(welt, z, l);
    let roh = [d.kolonie, d.ziel, d.wirtschaft, d.verteidigung];
    let summe = roh[0] + roh[1] + roh[2] + roh[3];
    for i in 0..B_ANZAHL as usize { anteil[i] = if summe > 0 { roh[i] * 100 / summe } else { 0 }; }
}

/// C++: bool aufWegZu(int ziel, int id, int tiefe)  (op2_ki.hpp:2162)
pub fn aufWegZu(welt: &mut dyn Welt, ziel: i32, id: i32, tiefe: i32) -> bool {
    if ziel == id { return true; }
    let zi = techIndex(welt, ziel); if zi < 0 || tiefe > 6 { return false; }
    let ti = super::forschung::techInfo(welt, zi);
    for r in 0..ti.required.len() {
        let rid = super::forschung::techInfo(welt, ti.required[r]).id;
        if aufWegZu(welt, rid, id, tiefe + 1) { return true; }
    }
    false
}

/// C++: bool fabrikErwartet(const Zustand& z, const Lage& l)  (op2_ki.hpp:2175)
pub fn fabrikErwartet(welt: &mut dyn Welt, z: &Zustand, l: &Lage) -> bool {
    if hatGeb(welt, z, l, mapVehicleFactory) != 0 || l.imBau[ix(mapVehicleFactory)] != 0 || !kitMoeglich(welt, z, mapVehicleFactory) { return false; }
    let sp = (z.spieler & 7) as usize;
    let mut begonnen = g_waffeBegonnen.with(|v| v.borrow()[sp]);
    if welt.Tick() < 10 { begonnen = 0; } // neues Spiel
    if begonnen == 0 && hatTech(welt, z, 3851) { begonnen = 1; }
    const LABS: [i32; 2] = [mapStandardLab, mapAdvancedLab];
    for &typ in LABS.iter() {
        for lab in welt.PlayerBuildingEnum(z.spieler, typ) {
            if begonnen != 0 { break; }
            if !welt.IsLive(lab) { continue; }
            let th = welt.GetLabCurrentTopic(lab); if th <= 0 { continue; }
            let ii = if th < welt.GetTechCount() { th } else { techIndex(welt, th) }; if ii < 0 { continue; }
            let id = super::forschung::techInfo(welt, ii).id;
            if !hatTech(welt, z, id) && aufWegZu(welt, 3851, id, 0) { begonnen = 1; }
        }
    }
    g_waffeBegonnen.with(|v| v.borrow_mut()[sp] = begonnen);
    begonnen != 0
}

/// C++: bool seltenErwartet(const Zustand& z, const Lage& l)  (op2_ki.hpp:2196)
pub fn seltenErwartet(welt: &mut dyn Welt, z: &Zustand, l: &Lage) -> bool {
    if hatGeb(welt, z, l, mapRareOreSmelter) != 0 || l.imBau[ix(mapRareOreSmelter)] != 0 || l.geb[ix(mapVehicleFactory)] == 0 { return false; }
    if !hatTech(welt, z, TECH_SELTENERZ) || !kitMoeglich(welt, z, mapRareOreSmelter) || !seltenGewollt(welt, z) { return false; }
    seltenBakeDa(welt, z)
}

pub(super) fn notfall(w: &mut Wahl, typ: i32, text: String) {
    w.typ = typ; w.bereich = -1;
    let text = snp(text, 120);
    w.warum = snp(format!("Notfall: {}", text), 200);
}
