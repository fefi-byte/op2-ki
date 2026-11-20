//! Rang der Gebaeude fuer die Arbeiter-Rangliste; Vormerkungen von Erz.
use super::*;
use super::ix;

/// C++: int rang(const Zustand& z, const Lage& l, int typ)  (op2_ki.hpp:1656)
pub fn rang(welt: &mut dyn Welt, z: &Zustand, l: &Lage, typ: i32) -> i32 {
    let ziel = z.ziel == 1 && z.zielErreicht == 0;
    match typ {
        mapCommandCenter => 100,
        mapTokamak | mapMHDGenerator | mapSolarPowerArray | mapGeothermalPlant => 95,
        mapStructureFactory => 90,
        mapRobotCommand => 60,
        mapGarage => 65,
        mapAgridome => if l.nahrungNetto > 60 && l.geb[ix(mapAgridome)] > 1 { 25 } else { 85 }, // ueberzaehlig = unwichtig
        // Nie wegen vollem Lager abwerten: die Schmelze IST das Lager
        mapCommonOreSmelter | mapCommonOreMine => 80,
        mapNursery => 78,
        mapUniversity => 74,
        // Das erste Medical Center ueber den Schmelzen -- ohne es sterben die Leute, und Erz-Ansprueche mit Rang 80 hielten es
        // zehntausende Ticks zurueck (Personal-Engpass frueh)
        mapMedicalCenter => if l.medKap == 0 && l.menschen >= 40 { 84 } else { 70 },
        // Labore: so wichtig, wie die Forschung sie gerade braucht (laborBedarf)
        mapAdvancedLab => { let r = 30 + z.laborNutzen[3] * 6 / 10; if ziel && r < 66 { 66 } else { r } }
        mapStandardLab => { let r = 30 + z.laborNutzen[2] * 6 / 10; if r < 64 { 64 } else { r } }
        // Seltenerz: so wichtig wie gewoehnliches Erz, sobald das gewaehlte Kampffahrzeug Seltenerz braucht
        mapRareOreSmelter | mapRareOreMine => {
            if seltenGewollt(welt, z) || (empMoeglich(welt, z) && empLohnt(welt, z, None)) { 80 } else if ziel { 62 } else { 30 } // auch fuer die EMP-Rakete
        }
        mapSpaceport => if ziel { 62 } else if !welt.IsEden(z.spieler) && hatTech(welt, z, 12201) { 78 } else { 30 }, // EMP-Rakete
        mapVehicleFactory => {
            // Fehlt Kampfwert gegen einen bekannten Feind, steht sie ueber GORF, Freizeit und Laboren (unter Medizin 70).
            if l.vfLaeuft == 0 { return if z.bedrohung >= 50 { 75 } else { 71 }; } // erste Fabrik vor Laboren und Medizin, unter Wohnraum
            if z.bedrohung >= 50 { 75 } else { 55 } // Panzernachschub unter Druck
        }
        mapGuardPost => 88,
        mapRecreationFacility | mapForum => 68, // Moral wie Medizin (Freizeit gehoert dazu)
        mapDIRT | mapGORF => 70, // wie Freizeit: erforscht, aber ohne Betrieb sinkt die Moral (Spieler)
        // Mehr als ein Haus Ueberschuss: unwichtig, damit die Arbeiter-Rangliste eins stilllegt
        mapResidence | mapReinforcedResidence | mapAdvancedResidence => {
            // Ueberschuss nur, wenn die Kolonie auch ohne ein Haus unter 85 % Auslastung bleibt
            let haeuser = l.geb[ix(mapResidence)] + l.geb[ix(mapReinforcedResidence)] + l.geb[ix(mapAdvancedResidence)];
            let mut proHaus = if haeuser > 0 { l.wohnraum / haeuser } else { 25 }; if proHaus < 25 { proHaus = 25; }
            if !standardVolk(welt, l) && (l.wohnraum - proHaus) as f64 * PAR(z.spieler).wohnraumGrenze / 100.0 >= l.menschen as f64 { 15 } else { 72 } // Wohnraum traegt die Moral
        }
        // Lager: immer zuerst stilllegen -- ein volles Lager schadet nicht, ein fehlender Arbeiter schon (Spieler).
        mapCommonStorage | mapRareStorage => 10,
        _ => 35,
    }
}

/// C++: int rangNeu(const Zustand& z, const Lage& l, int typ, int schon)  (op2_ki.hpp:1713)
pub fn rangNeu(welt: &mut dyn Welt, z: &Zustand, l: &Lage, typ: i32, schon: i32) -> i32 {
    let r = rang(welt, z, l, typ);
    if schon <= 0 { return r; }
    match typ {
        mapCommonOreSmelter | mapRareOreSmelter | mapVehicleFactory | mapNursery | mapUniversity
        | mapMedicalCenter | mapGarage | mapRobotCommand | mapSpaceport | mapStandardLab | mapAdvancedLab
        | mapGORF => r - 15,
        _ => r,
    }
}

/// C++: void vormerkungen(const Zustand& z, const Lage& l, int& erz, int& selten, const char** wofuer)  (op2_ki.hpp:1726)
pub fn vormerkungen(welt: &mut dyn Welt, z: &Zustand, l: &Lage, erz: &mut i32, selten: &mut i32, wofuer: &mut &'static str) {
    *erz = 0; *selten = 0; *wofuer = "-";
    if z.ziel != 1 || z.zielErreicht != 0 { return; }
    const REIHE: [i32; 3] = [mapStandardLab, mapAdvancedLab, mapSpaceport]; // Seltenerz entscheidet die Bilanz
    for &t in REIHE.iter() {
        if hatGeb(welt, z, l, t) != 0 { continue; }
        if !kitMoeglich(welt, z, t) { return; } // Technik fehlt noch: noch nichts vormerken
        *erz = welt.GetOreCost(t, z.spieler); *selten = welt.GetRareOreCost(t, z.spieler); *wofuer = name(welt, t);
        return;
    }
    if l.geb[ix(mapSpaceport)] != 0 { *erz = 2500; *selten = 700; *wofuer = "EDWARD+SULV"; } // UnitInfo kennt Raumfahrzeuge nicht sicher
}
