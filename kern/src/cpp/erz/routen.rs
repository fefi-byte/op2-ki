//! Gewoehnliches Erz: Mine anlegen (Surveyor, Robo-Miner), Erzrouten Mine -> Schmelze, Lastwagen je Route.
use super::*;
use super::{TECH_SELTENERZ, ix, exy};

/// C++: int lastwagenFuerAbstand(double kacheln)  (op2_ki.hpp:3335)
// Lastwagen je Erzroute aus der Entfernung Mine-Schmelze (Erztest): eine Fahrt ~300 + 60 Ticks je Kachel,
// eine Ladung ~500 Erz, die Schmelze verarbeitet hoechstens ~1400 Erz je 1000 Ticks.
pub fn lastwagenFuerAbstand(_welt: &mut dyn Welt, kacheln: f64) -> i32 {
    let fahrt = 300.0 + 60.0 * kacheln;                 // Ticks je Fahrt
    let jeLkw = (500 * 1000) as f64 / fahrt;             // Erz je 1000 Ticks und Lastwagen
    let n = (1400.0 / jeLkw + 0.85) as i32;              // bis die Schmelze (fast) ausgelastet ist
    if n < 1 { 1 } else if n > 8 { 8 } else { n }
}

/// C++: int lastwagenSoll(const Zustand& z, int i)  (op2_ki.hpp:3341)
pub fn lastwagenSoll(welt: &mut dyn Welt, z: &Zustand, i: i32) -> i32 {
    let i = i as usize;
    if z.minenId[i] == 0 || z.schmelzeId[i] == 0 { return 2; }
    let m: Id = z.minenId[i];
    let s: Id = z.schmelzeId[i];
    if !welt.IsLive(m) || !welt.IsLive(s) { return 2; }
    let ml = welt.Location(m);
    let sl = welt.Location(s);
    let d = (abstand2(welt, ml.x, ml.y, sl.x, sl.y) as f64).sqrt();
    lastwagenFuerAbstand(welt, d)
}

/// C++: bool lastwagenVergeben(Zustand& z, int id)  (op2_ki.hpp:3349)
pub fn lastwagenVergeben(welt: &mut dyn Welt, z: &mut Zustand, id: i32) -> bool {
    for i in 0..MAX_MINEN {
        if z.minenId[i] == 0 { continue; }
        for v in welt.GroupEnumerator(z.minenGruppe[i]) { if v == id { return true; } }
    }
    for i in 0..2 {
        if z.seltenMineId[i] == 0 { continue; }
        for v in welt.GroupEnumerator(z.seltenGruppe[i]) { if v == id { return true; } }
    }
    false
}

/// C++: void erz(Zustand& z, const Lage& l)  (op2_ki.hpp:3365)
/// Ergebnis der Routensuche fuer eine Schmelze
pub(super) enum Route { Uebersprungen, KeinPlatz, Gruppe(usize) }

/// C++: void erz(Zustand& z, const Lage& l). Rust 2026-11-18 in Schritte aufgeteilt (Verhalten gleich).
pub fn erz(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    let p = z.spieler;
    let t = welt.Tick();
    // Ziel: naechste freie Bake. Der Surveyor faehrt hin und vermisst; der Miner baut, sobald sie vermessen ist --
    // oder ohne, wenn es keinen Surveyor gibt oder er zu lange braucht.
    mineFertigMelden(welt, z, l);
    bakenErfassen(welt, z, t);
    if l.geb[ix(mapCommonOreMine)] < MAX_MINEN as i32 { mineAnlegen(welt, z, p, t); }
    // 3) Erzrouten: jede fertige Schmelze bekommt eine Mine, eine Mine versorgt bis zu 2 Schmelzen.
    //    Je Route eine MiningGroup mit zwei Lastwagen.
    let mut schmelzen: Vec<Id> = Vec::new();
    for u in welt.PlayerBuildingEnum(p, mapCommonOreSmelter) {
        if schmelzen.len() >= MAX_MINEN { break; }
        // nur lebende (wie C++): eine gerade zerstoerte Schmelze steht noch in der Liste
        if welt.IsLive(u) && welt.GetCurAction(u) != moObjWait && welt.GetCurAction(u) != moDevelop { schmelzen.push(u); }
    }
    let mut minen: Vec<Id> = Vec::new();
    for u in welt.PlayerBuildingEnum(p, mapCommonOreMine) {
        if minen.len() >= MAX_MINEN { break; }
        if welt.IsLive(u) && welt.GetCurAction(u) != moDevelop && welt.GetCurAction(u) != moObjWait { minen.push(u); }
    }
    let ns = schmelzen.len();
    let nm = minen.len();
    routenAufloesen(welt, z, &schmelzen, &minen);
    if ns == 0 || nm == 0 { return; }   // erst nach dem Aufloesen (wie C++)
    for k in 0..ns {
        let gu = match schmelzeRoute(welt, z, p, &schmelzen, &minen, k) { Route::Gruppe(g) => g, Route::Uebersprungen => continue, Route::KeinPlatz => break };
        lastwagenZuteilen(welt, z, p, gu);
    }
}

/// Meldung, wenn eine neue Mine steht
pub(super) fn mineFertigMelden(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    if l.geb[ix(mapCommonOreMine)] > z.minenGesehen {
        let txt = format!("Mine steht ({}) -- Weg {} hat funktioniert", l.geb[ix(mapCommonOreMine)], z.minerWeg);
        melde(welt, z, "fertig", mapCommonOreMine, z.bx, z.by, &txt);
    }
    z.minenGesehen = l.geb[ix(mapCommonOreMine)];
}

/// Erzbaken der Karte erfassen (alle 5000 Ticks, hoechstens 32)
pub(super) fn bakenErfassen(welt: &mut dyn Welt, z: &mut Zustand, t: i32) {
    if z.kbStand == 0 || t - z.kbStand > 5000 {
        z.kbStand = if t > 0 { t } else { 1 };
        z.kbN = 0;
        let bis = hoechsteEinheit(welt);
        let mut id = 1;
        while id <= bis && z.kbN < 32 {
            let u: Id = id;
            id += 1;
            if !welt.IsLive(u) || welt.GetType(u) != mapMiningBeacon { continue; }
            let bl = welt.Location(u);
            let (x, y) = exy(welt, z, bl);
            z.kbX[z.kbN as usize] = x as i16;
            z.kbY[z.kbN as usize] = y as i16;
            z.kbN += 1;
        }
    }
}

/// Naechste freie Bake waehlen, vermessen lassen und den Robo-Miner einsetzen
pub(super) fn mineAnlegen(welt: &mut dyn Welt, z: &mut Zustand, p: i32, t: i32) {
    let f = bakeSuchen(welt, z, 0);
    if f.beste == 0 {
        if selten(welt, z, 8, 5000) {
            let txt = format!("keine freie Erz-Bake (gesehen {}, gewoehnlich {}, frei {}, Einheiten bis {})",
                              f.gesehen, f.gewoehnlich, f.frei, f.einheiten);
            melde(welt, z, "warte", mapMiningBeacon, z.bx, z.by, &txt);
        }
    } else {
        let b: Id = f.beste;
        let bl = welt.Location(b);
        let (blx, bly) = exy(welt, z, bl);
        if z.bakeZiel != b {
            z.bakeZiel = b; z.bakeSeit = t;
            let mut bek = false;
            let mut bb = b;
            let bal = bakenBalken(welt, z, &mut bb, Some(&mut bek));
            let txt = format!("Bake {} gewaehlt: {} Balken{}, Wert {:.2} (gesehen {}, frei {})",
                              b, bal, if bek { "" } else { " (erwartet, unvermessen)" }, f.wert, f.gesehen, f.frei);
            melde(welt, z, "info", mapMiningBeacon, blx, bly, &txt);
        }
        let vermessen = (welt.GetSurveyedBy(b) & (1 << p)) != 0;
        let (_, surveyor) = surveyorFuehren(welt, z, p, &f, b, bl, blx, bly, vermessen);
        minerEinsetzen(welt, z, p, t, b, bl, blx, bly, vermessen, surveyor);
    }
}

/// Surveyor: zur Zielbake (bis sie vermessen ist), danach die naechste unvermessene Bake
#[allow(clippy::too_many_arguments)]
pub(super) fn surveyorFuehren(welt: &mut dyn Welt, z: &mut Zustand, p: i32, f: &BakenFund, _b: Id, bl: Loc, blx: i32, bly: i32, vermessen: bool) -> (Id, bool) {
    // Surveyor
    let mut s: Id = 0;
    let mut surveyor = false;
    for u in welt.PlayerVehicleEnum(p) { if welt.GetType(u) == mapRoboSurveyor { s = u; surveyor = true; break; } }
    // Ziel schon vermessen: der Surveyor vermisst die naechste unvermessene Bake
    if vermessen && surveyor && f.unvermessen != 0 && !welt.IsBusy(s) && welt.GetCurAction(s) == moDone && selten(welt, z, 11, 600) {
        let ul = welt.Location(f.unvermessen);
        let sl2 = welt.Location(s);
        let mut hin = ul;
        if abstand2(welt, sl2.x, sl2.y, ul.x, ul.y) > 2 { hin.x += 1; }   // erst daneben, dann drauf
        welt.DoMove(s, hin);
        let (ux, uy) = exy(welt, z, ul);
        let txt = format!("Surveyor {} vermisst die naechste Bake (Ertrag noch unbekannt)", s);
        melde(welt, z, "vermessen", mapMiningBeacon, ux, uy, &txt);
    }
    if !vermessen && surveyor && !welt.IsBusy(s) && welt.GetCurAction(s) == moDone && selten(welt, z, 9, 400) {
        let sl = welt.Location(s);
        let dort = abstand2(welt, sl.x, sl.y, bl.x, bl.y) <= 2;
        let mut ziel = bl;
        if !dort { ziel.x += 1; }
        welt.DoMove(s, ziel);
        let txt = if dort { format!("Surveyor {} steht an der Bake, noch nicht vermessen -- faehrt drauf", s) }
                  else { format!("Surveyor {} faehrt zur Bake", s) };
        melde(welt, z, "vermessen", mapMiningBeacon, blx, bly, &txt);
    }
    (s, surveyor)
}

/// Miner. HFLs DoDeployMiner faehrt nur hin; DoBuild mit der beim ConVec gemessenen Lesart baut die Mine.
/// Die BuildingGroup bleibt als Rueckfall; schlagen alle Wege fehl, wird die Bake gesperrt.
#[allow(clippy::too_many_arguments)]
pub(super) fn minerEinsetzen(welt: &mut dyn Welt, z: &mut Zustand, p: i32, t: i32, b: Id, bl: Loc, blx: i32, bly: i32, vermessen: bool, surveyor: bool) {
    let m: Id = z.bakeMiner;
    let minerLebt = z.bakeMiner != 0 && welt.IsLive(m) && welt.GetType(m) == mapRoboMiner;
    let minerAmWerk = minerLebt && (welt.IsBusy(m) || welt.GetCurAction(m) != moDone);
    let darf = vermessen || !surveyor || t - z.bakeSeit > 1200;
    let gruppeArbeitet = z.minerWeg == 2 && z.minenBauAn != 0 && welt.TotalUnitCount(z.minenBau) > 0;
    if gruppeArbeitet && t - z.minerSeit > 3000 {
        // Weg 1 und Weg 2 erfolglos: an dieser Bake geht es nicht -- Miner frei, Bake sperren
        if minerLebt { welt.RemoveUnit(z.minenBau, m); }
        welt.Destroy(z.minenBau); z.minenBauAn = 0;                      // alter Bauauftrag darf nicht mitwandern
        let mut k = 0usize;
        for i in 1..8 { if z.sperrBis[i] < z.sperrBis[k] { k = i; } }
        z.sperrBake[k] = b; z.sperrBis[k] = t + 40000;
        let txt = format!("Mine an Bake {} kommt nicht zustande -- Bake bis Tick {} gesperrt, naechste Bake", b, z.sperrBis[k]);
        melde(welt, z, "fehler", mapCommonOreMine, blx, bly, &txt);
        z.minerWeg = 1; z.bakeZiel = 0; z.bakeMiner = 0;
        if minerLebt { welt.DoStop(m); }
    } else if gruppeArbeitet {
        if selten(welt, z, 10, 2000) {
            let txt = format!("BuildingGroup soll den Miner einsetzen ({} Einheiten)", welt.TotalUnitCount(z.minenBau));
            melde(welt, z, "warte", mapCommonOreMine, blx, bly, &txt);
        }
    } else if darf && !minerAmWerk {
        // >= : der Einsatz laeuft alle 300 Ticks; mit > kam nie ein anderer Weg, nur derselbe Befehl erneut (mp4_41)
        if minerLebt && t - z.minerSeit >= 300 {
            let txt = format!("Robo-Miner {} steht, Mine nicht gebaut (Weg {}, vermessen: {})",
                              m, z.minerWeg, if vermessen { "ja" } else { "nein" });
            melde(welt, z, "fehler", mapCommonOreMine, blx, bly, &txt);
            z.minerWeg = if z.minerWeg == 1 { 2 } else if z.minerWeg == 2 { 0 } else { 1 };
        }
        // der vorgefahrene Miner zuerst (steht schon an der Bake)
        let mut miner = welt.PlayerVehicleEnum(p);
        if let Some(i) = miner.iter().position(|&u| u == z.minerVor) { let v = miner.remove(i); miner.insert(0, v); }
        for u in miner {
            // der vorgefahrene Miner zaehlt auch, solange er noch faehrt: sein einziger Auftrag ist das Vorfahren, und
            // bleibt er unterwegs haengen, waere er sonst fuer immer "beschaeftigt" (mp2_05: nie eine Mine)
            if welt.GetType(u) != mapRoboMiner || (welt.IsBusy(u) && u != z.minerVor) { continue; }
            if u == z.seltenMiner { continue; }                              // fuer Seltenerz reserviert
            if u == z.bakeMiner && t - z.minerSeit < 300 { break; }           // gleicher Auftrag, kurz warten
            let weg: &str;
            if z.minerWeg == 1 {
                // wie beim ConVec: Zielpunkt = Ecke unten rechts der Grundflaeche (2x1, Bake rechts) mit Rand
                welt.DoBuild(u, mapCommonOreMine, Loc::new(bl.x + 1, bl.y + 1));
                weg = "Weg 1: DoBuild";
            } else if z.minerWeg == 2 {
                if z.minenBauAn == 0 {
                    z.minenBau = welt.CreateBuildingGroup(p);
                    let a = engine(welt, z, z.bx - 3, z.by + 6);
                    let c = engine(welt, z, z.bx + 3, z.by + 9);
                    let ruhe = Rect::new(a, c);
                    welt.SetRect(z.minenBau, ruhe);
                    z.minenBauAn = 1;
                }
                welt.TakeUnit(z.minenBau, u);
                let ort = bl;
                welt.RecordBuilding(z.minenBau, ort, mapCommonOreMine, mapNone);
                weg = "Weg 2: BuildingGroup";
            } else {
                welt.DoDeployMiner(u, bl);
                weg = "Weg 0: HFL DoDeployMiner";
            }
            z.bakeMiner = u; z.minerSeit = t; z.minerVor = 0;
            lastwagenVorfahren(welt, z, p, b, bl);
            let txt = format!("Robo-Miner {} baut Mine ({}, {})", u, weg,
                              if vermessen { "Bake vermessen" } else if surveyor { "Surveyor braucht zu lange" } else { "ohne Surveyor" });
            melde(welt, z, "mine", mapCommonOreMine, blx, bly, &txt);
            break;
        }
    } else if !darf && !minerAmWerk && !gruppeArbeitet {
        // Vermessung laeuft: ein freier Robo-Miner faehrt schon zur Bake und wartet dort (spart den Anfahrtsweg)
        let vor = z.minerVor;
        let vorLebt = vor != 0 && welt.IsLive(vor) && welt.GetType(vor) == mapRoboMiner;
        if !vorLebt {
            for u in welt.PlayerVehicleEnum(p) {
                if welt.GetType(u) != mapRoboMiner || welt.IsBusy(u) || welt.GetCurAction(u) != moDone || u == z.seltenMiner { continue; }
                welt.DoMove(u, Loc::new(bl.x + 2, bl.y + 2));
                z.minerVor = u;
                melde(welt, z, "mine", mapRoboMiner, blx, bly, &format!("Robo-Miner {} faehrt schon zur Bake, waehrend sie vermessen wird", u));
                break;
            }
        }
    }
}

/// Mine geht in Bau: bis zu zwei freie Lastwagen fahren schon zur Bake (zugeteilt werden sie, wenn die Mine steht)
fn lastwagenVorfahren(welt: &mut dyn Welt, z: &mut Zustand, p: i32, b: Id, bl: Loc) {
    if z.lkwVorBake == b { return; }
    z.lkwVorBake = b;
    let mut n = 0;
    for u in welt.PlayerVehicleEnum(p) {
        if n >= 2 { break; }
        if welt.GetType(u) != mapCargoTruck || welt.IsBusy(u) || !welt.IsLive(u) || welt.GetCurAction(u) != moDone { continue; }
        if lastwagenVergeben(welt, z, u) { continue; }
        welt.DoMove(u, Loc::new(bl.x + 3, bl.y + 2 + n));
        z.lkwVor[n as usize] = u;
        n += 1;
    }
    if n > 0 {
        let (bx, by) = exy(welt, z, bl);
        melde(welt, z, "mine", mapCargoTruck, bx, by, &format!("{} Lastwagen fahren schon zur neuen Mine", n));
    }
}

/// Routen, deren Mine oder Schmelze nicht mehr steht, aufloesen -- die Lastwagen vorher aus der Gruppe nehmen
pub(super) fn routenAufloesen(welt: &mut dyn Welt, z: &mut Zustand, schmelzen: &[Id], minen: &[Id]) {
    let (ns, nm) = (schmelzen.len(), minen.len());
    for i in 0..MAX_MINEN {
        if z.minenId[i] == 0 { continue; }
        let mut m = false;
        let mut sm = false;
        for k in 0..nm { if minen[k] == z.minenId[i] { m = true; } }
        for k in 0..ns { if schmelzen[k] == z.schmelzeId[i] { sm = true; } }
        if !m || !sm {
            let mut raus: Vec<Id> = Vec::new();
            // nur lebende Lastwagen herausnehmen
            for v in welt.GroupEnumerator(z.minenGruppe[i]) {
                if raus.len() >= 8 { break; }
                if welt.IsLive(v) && welt.GetType(v) == mapCargoTruck { raus.push(v); }
            }
            let nr = raus.len() as i32;
            for &r in &raus { welt.RemoveUnit(z.minenGruppe[i], r); }
            z.minenId[i] = 0; z.schmelzeId[i] = 0;
            let txt = format!("Erzroute aufgeloest ({} zerstoert), {} Lastwagen frei", if !m { "Mine" } else { "Schmelze" }, nr);
            melde(welt, z, "mine", mapCommonOreMine, z.bx, z.by, &txt);
        }
    }
}

/// Route der Schmelze k: bestehende Gruppe oder neue Route zur Mine mit den wenigsten Routen (hoechstens 2)
pub(super) fn schmelzeRoute(welt: &mut dyn Welt, z: &mut Zustand, p: i32, schmelzen: &[Id], minen: &[Id], k: usize) -> Route {
    let nm = minen.len();
    let mut g: i32 = -1;
    for i in 0..MAX_MINEN { if z.minenId[i] != 0 && z.schmelzeId[i] == schmelzen[k] { g = i as i32; } }
    if g < 0 {
        // Mine mit den wenigsten Routen (hoechstens 2), bei Gleichstand die naechste
        let mut beste: i32 = -1;
        let mut besteN = 3;
        let mut besteD = 1 << 30;
        if !welt.IsLive(schmelzen[k]) { return Route::Uebersprungen; }              // gerade zerstoert
        let sl = welt.Location(schmelzen[k]);
        for m in 0..nm {
            let mut n = 0;
            for i in 0..MAX_MINEN { if z.minenId[i] == minen[m] { n += 1; } }
            if !welt.IsLive(minen[m]) { continue; }
            let ml = welt.Location(minen[m]);
            let d = abstand2(welt, ml.x, ml.y, sl.x, sl.y);
            // mit Zweitbasis: die naechste Mine, nicht die mit den wenigsten Routen (sonst faehrt die Schmelze der
            // Zweitbasis zur Mine der Hauptbasis)
            if n < 2 && d < besteD { beste = m as i32; besteN = n; besteD = d; }
        }
        if beste < 0 { return Route::Uebersprungen; }                                     // jede Mine hat schon zwei Schmelzen
        // Nicht mitten im Gefecht: Mine/Schmelze schwer beschaedigt oder Feind in der Naehe -> spaeter
        {
            let mut gefahr = false;
            let pr: [Id; 2] = [minen[beste as usize], schmelzen[k]];
            for &u2 in pr.iter() {
                let hp = welt.GetHitPoints(welt.GetType(u2), p);
                if !welt.IsLive(u2) || (hp > 0 && welt.GetDamage(u2) * 2 > hp) { gefahr = true; }
                let lo = welt.Location(u2);
                for f in 0..z.feindPosN as usize {
                    let (ox, oy) = exy(welt, z, lo);
                    if abstand2(welt, z.feindPosX[f], z.feindPosY[f], ox, oy) < 12 * 12 { gefahr = true; }
                }
            }
            if gefahr { return Route::Uebersprungen; }
        }
        for i in 0..MAX_MINEN { if z.minenId[i] == 0 { g = i as i32; break; } }
        if g < 0 { return Route::KeinPlatz; }
        let gu = g as usize;
        z.minenGruppe[gu] = welt.CreateMiningGroup(p);
        z.minenId[gu] = minen[beste as usize];
        z.schmelzeId[gu] = schmelzen[k];
        let ml = welt.Location(minen[beste as usize]);
        let bereich = Rect::new(Loc::new(sl.x - 3, sl.y - 3), Loc::new(sl.x + 3, sl.y + 3));
        welt.Setup(z.minenGruppe[gu], ml, sl, bereich);
        let (mx, my) = exy(welt, z, ml);
        let (sx, sy) = exy(welt, z, sl);
        let txt = format!("Erzroute Mine {},{} -> Schmelze {},{} ({}. Route dieser Mine)", mx, my, sx, sy, besteN + 1);
        melde(welt, z, "mine", mapCommonOreMine, mx, my, &txt);
    }
    Route::Gruppe(g as usize)
}

/// Lastwagen je Route nach Entfernung (gemessen, lastwagenSoll)
pub(super) fn lastwagenZuteilen(welt: &mut dyn Welt, z: &mut Zustand, p: i32, gu: usize) {
    let g = gu as i32;
    let hat = welt.TotalUnitCount(z.minenGruppe[gu]);
    let soll = lastwagenSoll(welt, z, g);
    if hat < soll {
        // die zur Mine vorgefahrenen zuerst, auch wenn sie noch fahren: ihr einziger Auftrag ist das Vorfahren, und bleiben
        // sie an der Mine haengen, waeren sie sonst fuer immer "beschaeftigt" (mp2_05: Route ohne Lastwagen, kein Erz)
        let mut lkw = welt.PlayerVehicleEnum(p);
        for &v in z.lkwVor.iter().rev() { if let Some(i) = lkw.iter().position(|&u| u == v && v != 0) { let x = lkw.remove(i); lkw.insert(0, x); } }
        for u in lkw {
            let vor = u != 0 && z.lkwVor.contains(&u);
            if welt.GetType(u) != mapCargoTruck || (welt.IsBusy(u) && !vor) || !welt.IsLive(u) { continue; }
            if lastwagenVergeben(welt, z, u) { continue; }
            for v in z.lkwVor.iter_mut() { if *v == u { *v = 0; } }
            welt.TakeUnit(z.minenGruppe[gu], u);
            let ul = welt.Location(u);
            let (ux, uy) = exy(welt, z, ul);
            let txt = format!("Lastwagen {} faehrt Erz", u);
            melde(welt, z, "mine", mapCargoTruck, ux, uy, &txt);
            break;
        }
    }
}

/// C++: bool seltenAktiv(const Zustand& z)  (op2_ki.hpp:3585)
// Seltenerz abbauen: fuers Satellitenziel oder sobald das gewaehlte Kampffahrzeug Seltenerz kostet.
pub fn seltenAktiv(welt: &mut dyn Welt, z: &Zustand) -> bool {
    if !hatTech(welt, z, TECH_SELTENERZ) { return false; }
    if z.ziel == 1 && z.zielErreicht == 0 { return true; }
    if empMoeglich(welt, z) && empLohnt(welt, z, None) { return true; }   // EMP-Rakete kostet 1500 Seltenerz
    seltenGewollt(welt, z)
}
