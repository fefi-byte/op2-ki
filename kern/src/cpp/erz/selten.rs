//! Seltenerz: Seltenerz-Mine anlegen und Routen zur Seltenerz-Schmelze.
use super::*;
use super::super::*;
use super::{TECH_SELTENERZ, WERT_LYNX, GEBAEUDE, ix, exy};

/// C++: void seltenErz(Zustand& z, const Lage& l)  (op2_ki.hpp:3592)
/// C++: void seltenErz(Zustand& z, const Lage& l). Seltenerz-Mine anlegen, Routen zur Seltenerz-Schmelze.
/// Rust 2026-11-18 aufgeteilt (Verhalten gleich).
pub fn seltenErz(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    if l.geb[ix(mapRareOreSmelter)] == 0 { return; }        // Mine erst, wenn die Schmelze steht
    let p = z.spieler;
    let t = welt.Tick();
    // Missionsmodus: jede Basis ihre eigene Seltenerz-Mine (die der zweiten Basis zaehlt daheim nicht mit), jede Mine zur
    // naechsten Seltenerz-Schmelze
    let mission = super::super::modus::mission(z);
    let mineDaheim = if mission {
        let r2 = { let r = PAR(p).minenUmkreis.max(35.0); (r * r) as i32 };
        welt.PlayerBuildingEnum(p, mapRareOreMine).into_iter().any(|m| { let ml = welt.Location(m); let (mx, my) = exy(welt, z, ml); abstand2(welt, mx, my, z.bx, z.by) <= r2 })
    } else { l.geb[ix(mapRareOreMine)] > 0 };
    if !mineDaheim { seltenMineAnlegen(welt, z, l, p, t); }
    // Mine -> Seltenerz-Schmelze, zwei Lastwagen je Mine
    let alle = welt.PlayerBuildingEnum(p, mapRareOreSmelter);
    let schmelze: Id = alle.first().copied().unwrap_or(0);
    if schmelze == 0 { return; }
    for mine in welt.PlayerBuildingEnum(p, mapRareOreMine) {
        let ziel = if mission {
            let ml = welt.Location(mine);
            alle.iter().copied().min_by_key(|&s| { let sl = welt.Location(s); abstand2(welt, ml.x, ml.y, sl.x, sl.y) }).unwrap_or(schmelze)
        } else { schmelze };
        if !seltenRoute(welt, z, p, mine, ziel) { break; }
    }
}

/// Seltenerz-Bake waehlen, vermessen lassen, Robo-Miner reservieren und einsetzen
pub(super) fn seltenMineAnlegen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, p: i32, t: i32) {
    let f = bakeSuchen(welt, z, 1);
    if f.beste == 0 {
        if selten(welt, z, 11, 5000) {
            let txt = format!("keine freie Seltenerz-Bake (gesehen {}, Seltenerz {}, frei {})", f.gesehen, f.gewoehnlich, f.frei);
            melde(welt, z, "warte", mapMiningBeacon, z.bx, z.by, &txt);
        }
    } else {
        let b: Id = f.beste;
        let bl = welt.Location(b);
        let (blx, bly) = exy(welt, z, bl);
        if z.seltenBake != b {
            z.seltenBake = b; z.seltenBakeSeit = t;
            let txt = format!("Seltenerz-Bake {} wird erschlossen", b);
            melde(welt, z, "info", mapMiningBeacon, blx, bly, &txt);
        }
        let vermessen = (welt.GetSurveyedBy(b) & (1 << p)) != 0;
        let surveyorDa = l.fz[ix(mapRoboSurveyor)] > 0;
        // Bauen, sobald vermessen -- oder ohne, wenn es keinen Surveyor gibt oder er zu lange braucht.
        let darf = vermessen || !surveyorDa || t - z.seltenBakeSeit > 1200;
        // Surveyor hinschicken, wenn er frei ist (wie beim gewoehnlichen Erz)
        if !vermessen && selten(welt, z, 14, 400) {
            for s in welt.PlayerVehicleEnum(p) {
                if welt.GetType(s) != mapRoboSurveyor || welt.IsBusy(s) || welt.GetCurAction(s) != moDone { continue; }
                let mut ziel = bl;
                let sl = welt.Location(s);
                if abstand2(welt, sl.x, sl.y, bl.x, bl.y) > 2 { ziel.x += 1; }
                welt.DoMove(s, ziel);
                let txt = format!("Surveyor {} faehrt zur Seltenerz-Bake", s);
                melde(welt, z, "vermessen", mapMiningBeacon, blx, bly, &txt);
                break;
            }
        }
        let m: Id = z.seltenMiner;
        let lebt = z.seltenMiner != 0 && welt.IsLive(m) && welt.GetType(m) == mapRoboMiner && welt.OwnerID(m) == p;
        if !lebt {
            z.seltenMiner = 0; z.seltenMinerSeit = 0;
            for u in welt.PlayerVehicleEnum(p) {
                if welt.GetType(u) != mapRoboMiner || welt.IsBusy(u) || welt.GetCurAction(u) != moDone { continue; }
                if u == z.bakeMiner && t - z.minerSeit < 300 { continue; }   // baut gerade gewoehnliches Erz
                z.seltenMiner = u;
                let ul = welt.Location(u);
                let (ux, uy) = exy(welt, z, ul);
                let txt = format!("Robo-Miner {} fuer Seltenerz reserviert", u);
                melde(welt, z, "info", mapRoboMiner, ux, uy, &txt);
                break;
            }
            if z.seltenMiner == 0 && selten(welt, z, 12, 3000) {
                melde(welt, z, "warte", mapRareOreMine, blx, bly, "kein freier Robo-Miner fuer die Seltenerz-Mine");
            }
        } else if darf && z.bauAn != 0 && z.seltenMinerSeit == 0 {
            // BuildingGroup setzt den Robo-Miner ein (Ort = Bake)
            welt.TakeUnit(z.bau, m);
            let ort = bl;
            welt.RecordBuilding(z.bau, ort, mapRareOreMine, mapNone);
            z.seltenMinerSeit = t;
            let txt = format!("BuildingGroup: Robo-Miner {} baut Seltenerz-Mine ({})", m,
                              if vermessen { "Bake vermessen" } else { "unvermessen" });
            melde(welt, z, "mine", mapRareOreMine, blx, bly, &txt);
        } else if z.seltenMinerSeit != 0 && t - z.seltenMinerSeit > 3000 && !welt.IsBusy(m) && welt.GetCurAction(m) == moDone {
            // Rueckfall: die Gruppe hat ihn nicht eingesetzt -> direkter Befehl wie beim gewoehnlichen Erz
            let ecke = super::minenEcke(welt, z, bl); welt.DoBuild(m, mapRareOreMine, ecke);
            z.seltenMinerSeit = t - 2000;
            let txt = format!("BuildingGroup setzt Robo-Miner {} nicht ein -- direkter DoBuild", m);
            melde(welt, z, "fehler", mapRareOreMine, blx, bly, &txt);
        }
    }
}

/// Route einer Seltenerz-Mine: Gruppe, Ziel Schmelze, zwei Lastwagen (notfalls von einer Erzroute). false = kein Platz.
pub(super) fn seltenRoute(welt: &mut dyn Welt, z: &mut Zustand, p: i32, mine: Id, schmelze: Id) -> bool {
    let mut g: i32 = -1;
    for i in 0..2 { if z.seltenMineId[i] == mine { g = i as i32; } }
    if g < 0 {
        for i in 0..2 { if z.seltenMineId[i] == 0 { g = i as i32; break; } }
        if g < 0 { return false; }
        let gu = g as usize;
        z.seltenGruppe[gu] = welt.CreateMiningGroup(p);
        z.seltenMineId[gu] = mine;
        z.seltenSchmelzeId[gu] = 0;
        let ml = welt.Location(mine);
        let (mx, my) = exy(welt, z, ml);
        melde(welt, z, "fertig", mapRareOreMine, mx, my, "Seltenerz-Mine steht");
    }
    let gu = g as usize;
    if z.seltenSchmelzeId[gu] != schmelze {
        let ml = welt.Location(mine);
        let sl = welt.Location(schmelze);
        let bereich = Rect::new(Loc::new(sl.x - 3, sl.y - 3), Loc::new(sl.x + 3, sl.y + 3));
        welt.Setup(z.seltenGruppe[gu], ml, sl, bereich);
        z.seltenSchmelzeId[gu] = schmelze;
        let (mx, my) = exy(welt, z, ml);
        let (sx, sy) = exy(welt, z, sl);
        let txt = format!("Seltenerz-Route Mine -> Schmelze {},{}", sx, sy);
        melde(welt, z, "mine", mapRareOreMine, mx, my, &txt);
    }
    if welt.TotalUnitCount(z.seltenGruppe[gu]) < 2 {
        let mut lkw = welt.PlayerVehicleEnum(p);
        // Missionsmodus: der naechste Lastwagen (die zweite Basis baut ihre eigenen)
        if super::super::modus::mission(z) { let sl = welt.Location(schmelze); lkw.sort_by_key(|&u| { let ul = welt.Location(u); abstand2(welt, ul.x, ul.y, sl.x, sl.y) }); }
        for u in lkw {
            if welt.GetType(u) != mapCargoTruck || welt.IsBusy(u) || !welt.IsLive(u) { continue; }
            if lastwagenVergeben(welt, z, u) { continue; }
            welt.TakeUnit(z.seltenGruppe[gu], u);
            let ul = welt.Location(u);
            let (ux, uy) = exy(welt, z, ul);
            let txt = format!("Lastwagen {} faehrt Seltenerz", u);
            melde(welt, z, "mine", mapCargoTruck, ux, uy, &txt);
            break;
        }
        // Kein freier Lastwagen und keiner baubar: eine Erzroute mit mindestens zwei Lastwagen gibt einen ab.
        if welt.TotalUnitCount(z.seltenGruppe[gu]) == 0 {
            for i in 0..MAX_MINEN {
                if z.minenId[i] == 0 || welt.TotalUnitCount(z.minenGruppe[i]) < 2 { continue; }
                let mut abgeben: Id = 0;
                for v in welt.GroupEnumerator(z.minenGruppe[i]) { if welt.IsLive(v) && welt.GetType(v) == mapCargoTruck { abgeben = v; } }
                if abgeben == 0 { continue; }
                welt.RemoveUnit(z.minenGruppe[i], abgeben);
                welt.TakeUnit(z.seltenGruppe[gu], abgeben);
                let al = welt.Location(abgeben);
                let (ax, ay) = exy(welt, z, al);
                let txt = format!("Lastwagen {} wechselt von der Erzroute zur Seltenerz-Route (kein Lastwagen baubar ohne Seltenerz)", abgeben);
                melde(welt, z, "mine", mapCargoTruck, ax, ay, &txt);
                break;
            }
        }
    }
    true
}
