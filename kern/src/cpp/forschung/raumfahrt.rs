//! Raumfahrt: Spaceport-Auftraege (Satellit, EMP-Rakete).
use super::*;
use super::super::*;
use super::{TECH_RAUMFAHRT, WERT_WT, WERT_LYNX, EMP_TECH, GEBAEUDE};

// ---------------------------------------------------------------------------------------------------
// Raumfahrt / space: EDWARD-Satellit auf einer SULV-Rakete starten (schrittweise, jeder Zustand in der Spur).

/// C++: void raumAuftrag(Zustand& z, UnitEx& sp, const Lage& l, int typ, int schritt)  (op2_ki.hpp:6216)
pub fn raumAuftrag(welt: &mut dyn Welt, z: &mut Zustand, sp: &mut Id, l: &Lage, typ: i32, schritt: i32) {
    let kosten = welt.GetOreCost(typ, z.spieler);
    let kostenSelten = welt.GetRareOreCost(typ, z.spieler);
    let sl = welt.Location(*sp);
    if kosten > l.erz || kostenSelten > l.selten {
        if selten(welt, z, 15, 1500) {
            let x = edX(welt, z, sl);
            let y = edY(welt, z, sl);
            melde(welt, z, "warte", typ, x, y, &format!("Ziel Satellit: Erz {}/{}, Seltenerz {}/{}", l.erz, kosten, l.selten, kostenSelten));
        }
        return;
    }
    welt.DoProduce(*sp, typ, mapNone);
    z.raumAuftrag = typ; z.raumSeit = welt.Tick(); z.raumSchritt = schritt;
    let x = edX(welt, z, sl);
    let y = edY(welt, z, sl);
    melde(welt, z, "raum", typ, x, y, &format!("Ziel Satellit: Spaceport stellt her (Erz {}, Seltenerz {})", kosten, kostenSelten));
}

/// C++: void raumfahrt(Zustand& z, const Lage& l)  (op2_ki.hpp:6230)
pub fn raumfahrt(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    if z.ziel != 1 || z.zielErreicht != 0 { return; }
    let p = z.spieler;
    let t = welt.Tick();
    let sats = welt.GetSatelliteCount(p, mapEDWARDSatellite);
    if sats > 0 {
        z.zielErreicht = t;
        melde(welt, z, "ziel", mapEDWARDSatellite, z.bx, z.by, &format!("ZIEL ERREICHT: EDWARD-Satellit im All ({}), Startversuch {}", sats, z.raumVersuche));
        welt.logZeile(&format!("Aufbau-KI Spieler {}: ZIEL ERREICHT -- EDWARD-Satellit im All (Tick {})", p, t));
        return;
    }
    raumStart(welt, z, l, mapEDWARDSatellite, TECH_RAUMFAHRT);
}

/// Missionsmodus (PAR solar, Nutzer 2026-11-22: "teste auch mal, ob ein Solarkraftwerk moeglich ist, dazu braucht es einen
/// Satelliten"): einen Solarsatelliten auf einer SULV starten, wie den EDWARD; danach baut bauwahl das Solarkraftwerk.
pub fn solarSatellit(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    if PAR(z.spieler).solar < 0.5 || !super::super::modus::mission(z) || PAR(z.spieler).raumhafen < 0.5 { return; }
    if z.ziel == 1 && z.zielErreicht == 0 { return; }   // der Spaceport gehoert dem EDWARD
    let p = z.spieler;
    if welt.GetSatelliteCount(p, mapSolarSatellite) > 0 {
        if z.raumSchritt != 9 {
            z.raumSchritt = 9;
            melde(welt, z, "raum", mapSolarSatellite, z.bx, z.by, &format!("Solarsatellit im All (Startversuch {}) -- Solarkraftwerk moeglich", z.raumVersuche));
        }
        return;
    }
    if !hatTech(welt, z, 10204) || !hatTech(welt, z, 5503) { return; }
    raumStart(welt, z, l, mapSolarSatellite, 10204);
}

/// Satellit herstellen, mit SULV auf die Rampe und starten (schrittweise, jeder Zustand in der Spur).
fn raumStart(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, sat: i32, tech: i32) {
    let p = z.spieler;
    let t = welt.Tick();
    let sats = welt.GetSatelliteCount(p, sat);
    if !hatTech(welt, z, tech) { return; }
    let mut sp: Id = welt.PlayerBuildingEnum(p, mapSpaceport).first().copied().unwrap_or(0);
    if sp == 0 { return; }
    if istGeparkt(welt, z, sp) { leerlaufWecken(welt, z, sp, "Ziel Satellit braucht den Spaceport"); return; }
    let sl = welt.Location(sp);
    let sx = edX(welt, z, sl);
    let sy = edY(welt, z, sl);
    let mut buchtEdward = -1;
    // char buchten[96]: am Ende auf 95 Bytes gekuerzt
    let mut buchten = String::new();
    for bay in 0..6 {
        let k = welt.GetFactoryCargo(sp, bay);
        if k == sat && buchtEdward < 0 { buchtEdward = bay; }
        let nm = if k > 0 && k != mapNone { name(welt, k) } else { "-" };
        buchten.push_str(if bay != 0 { "," } else { "" });
        buchten.push_str(nm);
    }
    let buchten = kurz(buchten, 96);
    let rampe = welt.GetLaunchPadCargo(sp);
    let arbeitet = welt.IsBusy(sp) || welt.GetCurAction(sp) == moDevelop;
    let rampeName = |welt: &mut dyn Welt| -> &'static str { if rampe > 0 && rampe != mapNone { name(welt, rampe) } else { "-" } };
    if selten(welt, z, 13, 400) {
        let rn = rampeName(welt);
        let akt = welt.GetCurAction(sp);
        let strom = welt.HasPower(sp) as i32;
        let arb = welt.HasWorkers(sp) as i32;
        melde(welt, z, "raum", mapSpaceport, sx, sy, &format!("Schritt {}: Rampe {} ({}), Buchten [{}], Aktion {}, arbeitet {}, Strom {}, Arbeiter {}, Satelliten {}",
            z.raumSchritt, rn, rampe, buchten, akt, if arbeitet { 1 } else { 0 }, strom, arb, sats));
    }
    if arbeitet {
        if z.raumAuftrag != 0 && t - z.raumSeit > 2 * GEDULD_KIT && selten(welt, z, 16, 2000) {
            melde(welt, z, "fehler", z.raumAuftrag, sx, sy, &format!("Spaceport braucht schon {} Ticks", t - z.raumSeit));
        }
        return;
    }
    if z.raumAuftrag != 0 { // Auftrag beendet: festhalten, wo das Ergebnis liegt
        let rn = rampeName(welt);
        melde(welt, z, "raum", z.raumAuftrag, sx, sy, &format!("fertig nach {} Ticks: Rampe {}, Buchten [{}]", t - z.raumSeit, rn, buchten));
        z.raumAuftrag = 0;
        z.raumSeit = t; // Wartezeiten der Folgeschritte ab hier zaehlen
    }
    if !welt.HasPower(sp) || !welt.HasWorkers(sp) {
        if selten(welt, z, 17, 1500) { melde(welt, z, "warte", mapSpaceport, sx, sy, "Spaceport ohne Strom oder Arbeiter"); }
        return;
    }
    let edwardDa = buchtEdward >= 0 || rampe == sat;
    match z.raumSchritt {
        0 => { // Satellit herstellen
            if edwardDa { z.raumSchritt = 1; } else { raumAuftrag(welt, z, &mut sp, l, sat, 0); }
        }
        1 => { // SULV herstellen
            if !edwardDa { z.raumSchritt = 0; } else { raumAuftrag(welt, z, &mut sp, l, mapSULV, 2); }
        }
        2 => { // Satellit auf die Startrampe
            if rampe == sat || buchtEdward < 0 { z.raumSchritt = 3; z.raumSeit = t; }
            else if t - z.raumSeit < 200 {
                welt.DoTransferCargo(sp, buchtEdward);
                if selten(welt, z, 18, 200) {
                    melde(welt, z, "raum", sat, sx, sy, &format!("Satellit aus Bucht {} auf die Startrampe (TransferCargo)", buchtEdward));
                }
            } else {
                welt.SetLaunchPadCargo(sp, sat);
                welt.SetFactoryCargo(sp, buchtEdward, mapNone, mapNone);
                melde(welt, z, "raum", sat, sx, sy, &format!("TransferCargo greift nicht -- Satellit aus Bucht {} direkt auf die Rampe gesetzt", buchtEdward));
                z.raumSchritt = 3; z.raumSeit = t;
            }
        }
        3 => { // starten
            let px = sl.x * 32 + 16;
            let py = sl.y * 32 + 16;
            z.raumVersuche += 1;
            if z.raumVersuche % 2 != 0 { welt.DoLaunch(sp, px, py, true); } else { welt.DoLaunch(sp, 0, 0, false); }
            let rn = rampeName(welt);
            melde(welt, z, "raum", mapSULV, sx, sy, &format!("Start! DoLaunch (Versuch {}, {}), Rampe {}", z.raumVersuche,
                if z.raumVersuche % 2 != 0 { "Ziel Spaceport, erzwungen" } else { "ohne Ziel" }, rn));
            z.raumSchritt = 4; z.raumSeit = t;
        }
        4 => { // warten, ob er oben ankommt
            if t - z.raumSeit > 600 {
                let rn = rampeName(welt);
                melde(welt, z, "fehler", mapSULV, sx, sy, &format!("nach 600 Ticks kein Satellit im All -- Rampe {}, Buchten [{}]", rn, buchten));
                // Rakete noch da (Satellit auf der Rampe)? Erneut starten, sonst von vorn.
                z.raumSchritt = if rampe == sat && z.raumVersuche < 6 { 3 } else { 0 };
            }
        }
        _ => {}
    }
}
