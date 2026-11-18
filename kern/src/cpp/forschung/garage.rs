//! Garage (Fahrzeuge einlagern/reparieren) und Reparatur beschaedigter Gebaeude und Fahrzeuge.
use super::*;
use super::GEBAEUDE;

/// C++: void garageAblauf(Zustand& z, const Lage& l)  (op2_ki.hpp:4990)
pub fn garageAblauf(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    let t = welt.Tick();
    let mut g: Id = 0;
    let mut da = false;
    for u in welt.PlayerBuildingEnum(z.spieler, mapGarage) {
        if welt.IsLive(u) && welt.GetCurAction(u) != moObjWait && welt.GetCurAction(u) != moDevelop { g = u; da = true; break; }
    }
    if !da {
        for i in 0..6 { z.garBucht[i] = 0; z.garFrei[i] = 0; }
        z.garDran = 0; z.garRaus = 0;
        return;
    }
    if !welt.HasPower(g) || !welt.HasWorkers(g) { return; }
    let dock = welt.GetDockLocation(g);
    // 1. Frisch herausgeholt: vom Dock weg und zurueck zur Wache
    if z.garRaus != 0 {
        // Erst wenn es wirklich wieder auf der Karte steht (Befehl waehrend der Ausfahrt stuerzt ab, 00475EF5).
        let mut draussen = false;
        let mut v: Id = 0;
        for u in welt.PlayerVehicleEnum(z.spieler) { if u == z.garRaus { v = u; draussen = true; break; } }
        let rl = if draussen { welt.Location(v) } else { dock };
        if draussen && t - z.garRausSeit >= 100 && welt.IsLive(v) && welt.GetCurAction(v) == moDone && rl.x == dock.x && rl.y == dock.y {
            z.garRaus = 0; // zuerst: ein Absturz darf den Ablauf nicht festhalten
            let hin = engine(welt, z, z.bx, z.by + 4);
            welt.DoMove(v, hin);
            let wache = z.wache;
            sicherNehmen(welt, z, wache, v);
            let dx = edX(welt, z, dock);
            let dy = edY(welt, z, dock);
            let d = welt.GetDamage(v);
            melde(welt, z, "reparatur", mapGarage, dx, dy, &format!("Garage: Fahrzeug repariert (Schaden jetzt {}) zurueck zur Wache", d));
        } else if t - z.garRausSeit > 800 {
            z.garRaus = 0;
        }
        return;
    }
    // Die Reparatur braucht Erz: ohne Erz steht sie still, die Frist laeuft dann nicht weiter
    if l.erz < 50 { for i in 0..6 { if z.garBucht[i] != 0 { z.garFrei[i] += TAKT; } } }
    // 2. Fertig reparierte Bucht leeren, wenn das Dock frei ist
    if z.garDran == 0 {
        for i in 0..6 {
            if z.garBucht[i] != 0 && t >= z.garFrei[i] {
                welt.DoTransferCargo(g, i as i32);
                z.garRaus = z.garBucht[i]; z.garRausSeit = t; z.garBucht[i] = 0;
                return;
            }
        }
    }
    // 3. Unterwegs zum Dock: dort angekommen in eine freie Bucht
    if z.garDran != 0 {
        let v: Id = z.garDran;
        if !welt.IsLive(v) || !welt.IsVehicle(v) || t - z.garDranSeit > 2500 {
            if welt.IsLive(v) && welt.IsVehicle(v) { let wache = z.wache; sicherNehmen(welt, z, wache, v); }
            z.garDran = 0;
            return;
        }
        let vl = welt.Location(v);
        if welt.GetCurAction(v) == moDone && vl.x == dock.x && vl.y == dock.y {
            for i in 0..6 {
                if z.garBucht[i] == 0 {
                    let d = welt.GetDamage(v);
                    welt.DoTransferCargo(g, i as i32);
                    z.garBucht[i] = v; z.garFrei[i] = t + d * 100 / 70 + 200;
                    z.garDran = 0;                     let vt = welt.GetType(v);
                    let dx = edX(welt, z, dock);
                    let dy = edY(welt, z, dock);
                    let nm = name(welt, vt);
                    let frei = z.garFrei[i];
                    melde(welt, z, "reparatur", vt, dx, dy, &format!("Garage: {} (Schaden {}) in Bucht {}, frei ab Tick {}", nm, d, i, frei));
                    return;
                }
            }
        } else if welt.GetCurAction(v) == moDone {
            welt.DoDock(v, dock);
        }
        return;
    }
    // 4. Naechstes Fahrzeug: am staerksten beschaedigt, aus der Wache, Erz fuer die Reparatur da, kein Feind in der Naehe
    if t % 100 >= TAKT || z.feindUnterwegs > 0 || z.feindNah > 0 { return; }
    let mut freiB = false;
    for i in 0..6 { if z.garBucht[i] == 0 { freiB = true; } }
    if !freiB { return; }
    let mut beste: Id = 0;
    let mut besteD = -1;
    for v in welt.GroupEnumerator(z.wache) {
        if !welt.IsLive(v) || !kaempfer(welt, v) || welt.GetType(v) == mapScorpion || welt.GetCurAction(v) != moDone { continue; }
        let hp = welt.GetHitPoints(welt.GetType(v), z.spieler);
        if hp <= 0 || welt.GetDamage(v) * 100 / hp < 30 { continue; }
        if welt.GetDamage(v) > besteD { besteD = welt.GetDamage(v); beste = v; }
    }
    if besteD < 0 || l.erz < 200 { return; }
    welt.RemoveUnit(z.wache, beste);
    welt.DoDock(beste, dock);
    z.garDran = beste; z.garDranSeit = t;
}

/// C++: void reparatur(Zustand& z, const Lage& l)  (op2_ki.hpp:5067)
// Reparatur: ein freies Reparaturfahrzeug bzw. eine Spinne; sonst ein ConVec (nur Gebaeude, nicht, wenn ein Bau ihn braucht).
// Tokamaks ab 5 % Schaden, alle anderen Gebaeude sofort. Nicht mitten im Gefecht.
/// Reparaturziel: Gebaeude oder Fahrzeug, Wertung, Schaden in %
#[derive(Clone, Copy, Default)]
pub(super) struct RepZiel { u: Id, wert: i32, proz: i32, fz: bool }

/// In die nach Wertung sortierte Liste der 6 besten einreihen (volle Liste: nur Bessere, wie jetzt C++)
pub(super) fn einreihen(beste: &mut [RepZiel; 6], nb: &mut usize, zl: RepZiel) {
    if *nb == 6 && beste[5].wert >= zl.wert { return; }
    let mut k = if *nb < 6 { *nb += 1; *nb - 1 } else { 5 };
    beste[k] = zl;
    while k > 0 && beste[k].wert > beste[k - 1].wert { beste.swap(k, k - 1); k -= 1; }
}

/// C++: void reparatur(Zustand& z, const Lage& l). Rust 2026-11-18 aufgeteilt (Verhalten gleich).
pub fn reparatur(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    let t = welt.Tick();
    if t % 100 >= TAKT { return; }
    let mut frei: [Id; 6] = [0; 6];
    let mut nf: usize = 0;
    for u in welt.PlayerVehicleEnum(z.spieler) {
        let ty = welt.GetType(u);
        if ty != mapRepairVehicle && ty != mapSpider { continue; }
        if welt.GetCurAction(u) == moDone && !welt.IsBusy(u) && nf < 6 { frei[nf] = u; nf += 1; }
    }
    // C++: Ziel beste[6] uninitialisiert (wert = Stapelmuell); hier 0.
    let mut beste = [RepZiel::default(); 6];
    let mut nb: usize = 0;
    gebaeudeSchaeden(welt, z, l, &mut beste, &mut nb);
    if nf != 0 { fahrzeugSchaeden(welt, z, &mut beste, &mut nb); }
    if nb == 0 { return; }
    if nf == 0 && !convecAushelfen(welt, z, l, &mut beste, &mut nb, &mut frei, &mut nf) { return; }
    let mut i = 0;
    while i < nf && i < nb {
        welt.DoRepair(frei[i], beste[i].u);
        let lo = welt.Location(beste[i].u);
        if selten(welt, z, 26 + (i as i32 & 1), 300) {
            let bt = welt.GetType(beste[i].u);
            let x = edX(welt, z, lo);
            let y = edY(welt, z, lo);
            let nm = name(welt, bt);
            let wer = if welt.GetType(frei[i]) == mapConVec { "ConVec" } else { "Reparaturfahrzeug" };
            melde(welt, z, "reparatur", bt, x, y, &format!("{} hat {} % Schaden -- {} repariert", nm, beste[i].proz, wer));
        }
        i += 1;
    }
}

/// Beschaedigte fertige Gebaeude ausserhalb von Gefechten (Tokamak erst ab 5 %)
pub(super) fn gebaeudeSchaeden(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, beste: &mut [RepZiel; 6], nb: &mut usize) {
    for &ty in GEBAEUDE.iter() {
        for b in welt.PlayerBuildingEnum(z.spieler, ty) {
            // Im Bau zaehlt die fehlende Bauleistung als Schaden -- das ist keine Reparatur.
            let akt = welt.GetCurAction(b);
            if akt == moObjWait || akt == moDevelop || akt == moOperationalWait { continue; }
            if imAufbau(welt, z, b, ty) { continue; }
            let dmg = welt.GetDamage(b);
            if dmg <= 0 { continue; }
            let hp = welt.GetHitPoints(ty, z.spieler);
            let proz = if hp > 0 { dmg * 100 / hp } else { 0 };
            if ty == mapTokamak && proz < 5 { continue; }
            let lo = welt.Location(b);
            let bx = edX(welt, z, lo);
            let by = edY(welt, z, lo);
            let mut gefecht = false;
            let mut f = 0;
            while f < z.feindPosN && !gefecht {
                if abstand2(welt, z.feindPosX[f as usize], z.feindPosY[f as usize], bx, by) < 8 * 8 { gefecht = true; }
                f += 1;
            }
            if gefecht { continue; }
            let wert = proz * 2 + rang(welt, z, l, ty) / 2 + if ty == mapGuardPost { 40 } else { 0 };
            einreihen(beste, nb, RepZiel { u: b, wert, proz, fz: false });
        }
    }
}

/// Kampffahrzeuge in Basisnaehe: nur ueber 50 % Schaden, nur mit echtem Reparaturgeraet, nicht im Gefecht
pub(super) fn fahrzeugSchaeden(welt: &mut dyn Welt, z: &mut Zustand, beste: &mut [RepZiel; 6], nb: &mut usize) {
    for v in welt.PlayerVehicleEnum(z.spieler) {
        if !welt.IsLive(v) || !kaempfer(welt, v) || welt.GetType(v) == mapScorpion { continue; }
        let hp = welt.GetHitPoints(welt.GetType(v), z.spieler);
        let proz = if hp > 0 { welt.GetDamage(v) * 100 / hp } else { 0 };
        if proz <= 50 { continue; }
        let lo = welt.Location(v);
        let vx = edX(welt, z, lo);
        let vy = edY(welt, z, lo);
        if abstand2(welt, vx, vy, z.bx, z.by) > 30 * 30 { continue; }
        let mut gefecht = false;
        let mut f = 0;
        while f < z.feindPosN && !gefecht {
            if abstand2(welt, z.feindPosX[f as usize], z.feindPosY[f as usize], vx, vy) < 10 * 10 { gefecht = true; }
            f += 1;
        }
        if gefecht { continue; }
        let wert = (proz - 50) * 2 + 40;
        einreihen(beste, nb, RepZiel { u: v, wert, proz, fz: true });
    }
}

/// Ohne Reparaturgeraet: ein ConVec repariert ein Gebaeude, wenn mehr ConVecs frei sind, als Bauauftraege warten.
/// false = keiner frei.
#[allow(clippy::too_many_arguments)]
pub(super) fn convecAushelfen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, beste: &mut [RepZiel; 6], nb: &mut usize, frei: &mut [Id; 6], nf: &mut usize) -> bool {
    // ConVec: nur Gebaeude, und nur, wenn mehr ConVecs frei sind, als Bauauftraege warten
    let mut g: i32 = -1;
    let mut i = 0;
    while i < *nb && g < 0 { if !beste[i].fz { g = i as i32; } i += 1; }
    if g < 0 { return false; }
    beste[0] = beste[g as usize];
    *nb = 1;
    let mut auftraege = l.kitsFabrik;
    let mut ruhig = 0;
    for i in 0..z.anzahlPlaetze as usize {
        if z.plaetze[i].zustand == P_FAHRT && z.plaetze[i].convec == 0 { auftraege += 1; }
    }
    let mut kandidat: Id = 0;
    for u in welt.PlayerVehicleEnum(z.spieler) {
        if welt.GetType(u) == mapConVec && welt.GetCurAction(u) == moDone && !welt.IsBusy(u) && (welt.GetCargo(u) <= 0 || welt.GetCargo(u) == mapNone) {
            ruhig += 1;
            if kandidat == 0 { kandidat = u; }
        }
    }
    if kandidat == 0 || ruhig <= auftraege { return false; }
    frei[*nf] = kandidat;
    *nf += 1;
    true
}
