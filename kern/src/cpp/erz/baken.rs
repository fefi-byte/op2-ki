//! Erzbaken: Minen und Baken finden, Balken (Ertrag), beste freie Bake suchen.
use super::*;
use super::super::*;
use super::{TECH_SELTENERZ, WERT_LYNX, GEBAEUDE, ix, exy};

/// C++: int hoechsteEinheit()  (op2_ki.hpp:3229)
pub fn hoechsteEinheit(welt: &mut dyn Welt) -> i32 {
    let n = welt.hoechsteEinheit();
    if n > 0 && n < 4096 { n } else { 0 }
}

/// C++: bool mineAuf(const LOCATION& l)  (op2_ki.hpp:3235)
pub fn mineAuf(welt: &mut dyn Welt, l: Loc) -> bool {
    // ueber die Gebaeudelisten (wie jetzt C++): der LocationEnumerator des Spiels liefert eine Mine erst nach einer
    // unbekannten Weile (gemessen 2026-11-12: 840 Ticks lang nicht)
    for p in 0..welt.NoPlayers() {
        for typ in [mapCommonOreMine, mapRareOreMine] {
            for u in welt.PlayerBuildingEnum(p, typ) {
                if welt.IsLive(u) { let m = welt.Location(u); if m.x == l.x && m.y == l.y { return true; } }
            }
        }
    }
    false
}

/// C++: int bakenBalken(const Zustand& z, UnitEx& b, bool* bekannt = nullptr)  (op2_ki.hpp:3244)
// Ertrag einer Bake in Balken (1..3), wie ein Spieler ihn kennt: erst nach dem Vermessen; vorher die Erwartung 2.
// Engine: barYield 0 = Bar3, 1 = Bar2, 2 = Bar1.
pub fn bakenBalken(welt: &mut dyn Welt, z: &Zustand, b: &mut Id, bekannt: Option<&mut bool>) -> i32 {
    let vermessen = (welt.GetSurveyedBy(*b) & (1 << z.spieler)) != 0;
    if let Some(k) = bekannt { *k = vermessen; }
    if !vermessen { return 2; }
    let y = welt.GetBarYield(*b);
    if y == 0 { 3 } else if y == 1 { 2 } else if y == 2 { 1 } else { 2 }
}

pub(super) struct Suche { f: BakenFund, ids: [i32; 64], nids: usize, naechsteUnverm: i32 }

// Lambda `pruefe` aus bakeSuchen.
pub(super) fn bakePruefe(welt: &mut dyn Welt, z: &Zustand, erzTyp: i32, s: &mut Suche, u: Id) {
    if welt.GetType(u) != mapMiningBeacon { return; }
    for i in 0..s.nids { if s.ids[i] == u { return; } }
    if s.nids < 64 { s.ids[s.nids] = u; s.nids += 1; }
    s.f.gesehen += 1;
    if welt.GetOreType(u) != erzTyp { return; }                         // 0 gewoehnlich, 1 Seltenerz
    s.f.gewoehnlich += 1;
    let l = welt.Location(u);
    if mineAuf(welt, l) { return; }
    for i in 0..8 { if z.sperrBake[i] == u && welt.Tick() < z.sperrBis[i] { return; } }
    // Missionsmodus: Minen nur im Umkreis minenUmkreis der Basis und hoechstens minenJeSorte je Sorte (die zweite Basis
    // erschliesst ihre Baken selbst, zweitbasis.rs)
    let pm = PAR(z.spieler);
    if pm.minenUmkreis > 0.0 || pm.minenJeSorte > 0.0 {
        let (bx, by) = exy(welt, z, l);
        let r2 = (pm.minenUmkreis * pm.minenUmkreis) as i32;
        if pm.minenUmkreis > 0.0 && abstand2(welt, bx, by, z.bx, z.by) > r2 { return; }
        if pm.minenJeSorte > 0.0 {
            let typ = if erzTyp == 1 { mapRareOreMine } else { mapCommonOreMine };
            let umkreis = if pm.minenUmkreis > 0.0 { r2 } else { 35 * 35 };
            let n = welt.PlayerBuildingEnum(z.spieler, typ).into_iter().filter(|&m| { let ml = welt.Location(m); let (mx, my) = exy(welt, z, ml); abstand2(welt, mx, my, z.bx, z.by) <= umkreis }).count();
            if n as f64 >= pm.minenJeSorte { return; }
        }
    }
    s.f.frei += 1;
    let mut bekannt = false;
    let mut uu = u;
    let balken = bakenBalken(welt, z, &mut uu, Some(&mut bekannt));
    let (bx, by) = exy(welt, z, l);
    let d = (abstand2(welt, bx, by, z.bx, z.by) as f64).sqrt();
    let mut wert = balken as f64 * 1000.0 / (300.0 + 60.0 * d);       // Erz je Lastwagen und Zeit, relativ
    if z.feindGeb != 0 {
        let df = (abstand2(welt, bx, by, z.feindX, z.feindY) as f64).sqrt();
        if df < d { wert *= df / (if d > 1.0 { d } else { 1.0 }); }    // naeher am Feind: gefaehrdet
    }
    if wert > s.f.wert { s.f.wert = wert; s.f.beste = u; s.f.balken = balken; }
    // vermessen nur, was nicht naeher am Feind liegt als an der eigenen Basis
    let feindSeite = z.feindGeb != 0 && abstand2(welt, bx, by, z.feindX, z.feindY) < abstand2(welt, bx, by, z.bx, z.by);
    if !bekannt && !feindSeite && abstand2(welt, bx, by, z.bx, z.by) < s.naechsteUnverm {
        s.naechsteUnverm = abstand2(welt, bx, by, z.bx, z.by);
        s.f.unvermessen = u;
    }
}

/// C++: BakenFund bakeSuchen(const Zustand& z, int erzTyp = 0)  (op2_ki.hpp:3258)
// Beste freie Erz-Bake der Sorte `erzTyp` nach erwartetem Ertrag; dazu die naechste unvermessene Bake.
// Drei Wege, weil nicht sicher ist, welcher Aufzaehler Baken liefert: alle Einheitennummern, Rechteck, Umkreis.
pub fn bakeSuchen(welt: &mut dyn Welt, z: &Zustand, erzTyp: i32) -> BakenFund {
    let mut f = BakenFund::default();
    f.gesehen = 0; f.gewoehnlich = 0; f.frei = 0; f.einheiten = 0; f.balken = 0; f.wert = -1.0;
    let mut s = Suche { f, ids: [0; 64], nids: 0, naechsteUnverm: 1 << 30 };
    // Die Aufzaehler der Engine liefern Baken offenbar nicht -- darum zuerst alle Einheitennummern durchgehen.
    {
        let bis = hoechsteEinheit(welt);
        s.f.einheiten = bis;
        for id in 1..=bis {
            let u: Id = id;
            if !welt.IsLive(u) { continue; }
            bakePruefe(welt, z, erzTyp, &mut s, u);
        }
    }
    {
        let a = engine(welt, z, 1, 1);
        let b = engine(welt, z, z.breite - 2, z.hoehe - 2);
        let alles = Rect::new(a, b);
        for u in welt.InRectEnumerator(alles) { bakePruefe(welt, z, erzTyp, &mut s, u); }
    }
    {
        let m = engine(welt, z, z.bx, z.by);
        for u in welt.InRangeEnumerator(m, 60) { bakePruefe(welt, z, erzTyp, &mut s, u); }
    }
    for i in 0..z.anzahlBaken as usize {
        let o = engine(welt, z, z.bakenX[i], z.bakenY[i]);
        for u in welt.LocationEnumerator(o) { bakePruefe(welt, z, erzTyp, &mut s, u); }
    }
    s.f
}

/// C++: bool seltenBakeDa(const Zustand& z)  (op2_ki.hpp:3320)
pub fn seltenBakeDa(welt: &mut dyn Welt, z: &Zustand) -> bool {
    if seltenRouten(welt, z) != 0 { return true; }
    if !welt.PlayerBuildingEnum(z.spieler, mapRareOreMine).is_empty() { return true; }
    bakeSuchen(welt, z, 1).beste != 0
}
