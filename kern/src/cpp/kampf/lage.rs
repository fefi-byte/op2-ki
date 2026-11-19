//! Laufendes Gefecht: Rueckzug pruefen, Verfolgen, Spinnen-Einsatz.
use super::*;
use super::GRUPPE_MAX;

/// C++: void rueckzugPruefen(Zustand& z, FightGroup& wache)  (op2_ki.hpp:4084)
/// Rueckzug: sobald die Truppe vor Ort unterliegt, fahren alle heim.
pub fn rueckzugPruefen(welt: &mut dyn Welt, z: &mut Zustand, wache: Gruppe) {
    if z.bilRueck != 0 || z.bilStart == 0 || z.bilK[0] < 0 { return; }
    let t = welt.Tick();
    if t % 100 >= TAKT { return; }
    let (mut mx, mut my, mut k) = (0, 0, 0);
    for v in AngriffEnum(welt, z) { if welt.IsLive(v) && welt.IsVehicle(v) { let l = welt.Location(v); mx += l.x; my += l.y; k += 1; } }
    if k == 0 { return; }
    let (mut nah, mut alle, mut e, mut d) = (0, 0, 0, 0);
    kontaktStaerken(welt, z, mx / k, my / k, &mut nah, &mut alle, &mut e, &mut d);
    // Zwei getrennte Haelften (zwei Wege) -- der gemeinsame Schwerpunkt liegt zwischen ihnen (oft hinter Felsen).
    // Je Haelfte am eigenen Schwerpunkt messen; eigene Staerke zusammen, Abwehr die groessere der beiden.
    if z.aufmarsch2X != 0 && z.angriff2An != 0 {
        let schwer = |welt: &mut dyn Welt, g: Gruppe| -> Option<(i32, i32)> {
            let (mut sx, mut sy, mut n) = (0, 0, 0);
            for v in welt.GroupEnumerator(g) { if welt.IsLive(v) && welt.IsVehicle(v) { let l = welt.Location(v); sx += l.x; sy += l.y; n += 1; } }
            if n == 0 { None } else { Some((sx / n, sy / n)) }
        };
        if let (Some(c1), Some(c2)) = (schwer(welt, z.angriff), schwer(welt, z.angriff2)) {
            if abstand2(welt, c1.0, c1.1, c2.0, c2.1) > 24 * 24 {
                let (mut n1, mut a1, mut e1, mut d1) = (0, 0, 0, 0);
                let (mut n2, mut a2, mut e2, mut d2) = (0, 0, 0, 0);
                kontaktStaerken(welt, z, c1.0, c1.1, &mut n1, &mut a1, &mut e1, &mut d1);
                kontaktStaerken(welt, z, c2.0, c2.1, &mut n2, &mut a2, &mut e2, &mut d2);
                alle = a1; e = e1 + e2; d = d1.max(d2);
            }
        }
    }
    if d <= 0 || e as f64 >= d as f64 * PAR(z.spieler).rueckzugAb { return; }
    z.bilRueck = 1;
    let mut raus = [0 as Id; 64]; let mut n = 0usize;
    for v in AngriffEnum(welt, z) {
        if n >= 64 { break; }
        if !welt.IsLive(v) || !welt.IsVehicle(v) || !kaempfer(welt, v) { continue; }
        let mut schon = false; for i in 0..n { if raus[i] == v { schon = true; } }
        if schon { continue; }
        // alle fahren heim -- wer zurueckbleibt, stirbt gegen die Ueberzahl ohne Gegenwert; daheim repariert die Garage
        raus[n] = v; n += 1;
    }
    let heim = engine(welt, z, z.bx, z.by + 4);
    for i in 0..n {
        let (mut erz, mut sel) = (0, 0); fzKosten(welt, z.spieler, &mut raus[i], &mut erz, &mut sel); z.bilRueckErz += erz;
        if welt.TotalUnitCount(wache) >= GRUPPE_MAX { welt.DoMove(raus[i], heim); continue; }   // bleibt in der Angriffsgruppe
        angriffRaus(welt, z, raus[i]); welt.TakeUnit(wache, raus[i]); welt.DoMove(raus[i], heim);
    }
    melde(welt, z, "kampf", mapLynx, mx / k - z.ux, my / k - z.uy,
          &format!("Rueckzug: {} von {} Fahrzeugen fahren heim (vor Ort eigene Staerke {} gegen {})", n, alle, e, d));
}

/// C++: void verfolgen(Zustand& z)  (op2_ki.hpp:4113)
/// Verfolgung beschaedigter Feinde in Basisnaehe durch wenige gesunde Wachfahrzeuge.
pub fn verfolgen(welt: &mut dyn Welt, z: &mut Zustand) {
    let t = welt.Tick();
    if t - z.jagdTakt < 100 { return; }
    z.jagdTakt = t;
    let mut ziel: Id = 0; let mut hat = false; let mut best = 1 << 30;
    let mut p = 0;
    while p < welt.NoPlayers() {
        if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { p += 1; continue; }
        for u in welt.PlayerVehicleEnum(p) {
            if !welt.IsLive(u) || kampfWert(welt, u) <= 0 { continue; }
            let hp = welt.GetHitPoints(welt.GetType(u), p);
            if hp <= 0 || welt.GetDamage(u) * 10 < hp * 4 { continue; }              // unter 60 % HP
            let l = welt.Location(u); let x = edX(welt, z, l); let y = edY(welt, z, l);
            let d = abstand2(welt, x, y, z.bx, z.by);
            if d > 40 * 40 || d >= best { continue; }
            let mut turm = false;
            for w in welt.PlayerBuildingEnum(p, mapGuardPost) {
                if turm { break; }
                if welt.IsLive(w) { let wl = welt.Location(w); if abstand2(welt, wl.x, wl.y, l.x, l.y) <= 8 * 8 { turm = true; } }
            }
            if turm { continue; }
            best = d; ziel = u; hat = true;
        }
        p += 1;
    }
    if !hat { z.jagdZiel = 0; return; }
    if z.jagdZiel != ziel { z.jagdZiel = ziel; }
    let zl = welt.Location(ziel);
    // bis zu 4 gesunde, freie Wachfahrzeuge, die naechsten zuerst
    // ABWEICHUNG: dj[3] ist in C++ beim 4. Kandidaten uninitialisiert; hier i32::MAX (der 4. wird aufgenommen).
    let mut jg = [0 as Id; 4]; let mut dj = [i32::MAX; 4]; let mut nj = 0usize;
    for v in welt.GroupEnumerator(z.wache) {
        if !welt.IsLive(v) || !kaempfer(welt, v) { continue; }
        let hp = welt.GetHitPoints(welt.GetType(v), z.spieler);
        if hp <= 0 || welt.GetDamage(v) * 2 > hp { continue; }
        let l = welt.Location(v); let d = abstand2(welt, l.x, l.y, zl.x, zl.y);
        if nj == 4 && dj[3] <= d { continue; }   // volle Liste: nur Naehere (wie jetzt C++)
        let mut k = if nj < 4 { nj += 1; nj - 1 } else { 3 };
        jg[k] = v; dj[k] = d;
        while k > 0 && dj[k] < dj[k - 1] { jg.swap(k, k - 1); dj.swap(k, k - 1); k -= 1; }
    }
    for i in 0..nj {
        let wi = welt.GetWeapon(jg[i]);
        let rw = if welt.GetWeaponRange(wi, z.spieler) > 0 { welt.GetWeaponRange(wi, z.spieler) } else { 4 };
        if dj[i] <= rw * rw { welt.DoAttack(jg[i], ziel); } else { welt.DoMove(jg[i], zl); }
    }
}

/// C++: void spinnenEinsatz(Zustand& z)  (op2_ki.hpp:4158)
/// Spinnen folgen einem EMP-Angriff und uebernehmen gelaehmte Feinde; die erste bleibt daheim.
pub fn spinnenEinsatz(welt: &mut dyn Welt, z: &mut Zustand) {
    if welt.IsEden(z.spieler) { return; }
    let t = welt.Tick();
    if t - z.spinneTakt < 40 { return; }
    z.spinneTakt = t;
    // Uebernahmen zaehlen
    for i in 0..8 {
        if z.spinneZiel[i] != 0 {
            let u: Id = z.spinneZiel[i];
            if welt.IsLive(u) && welt.IsVehicle(u) && welt.OwnerID(u) == z.spieler {
                z.bilUeber += 1; z.spinneZiel[i] = 0;
                melde(welt, z, "kampf", mapSpider, z.bx, z.by, &format!("Spinne hat ein Feindfahrzeug uebernommen ({} in diesem Angriff)", z.bilUeber));
            } else if !welt.IsLive(u) || !welt.IsVehicle(u) { z.spinneZiel[i] = 0; }
        }
    }
    let mut sp = [0 as Id; 8]; let mut ns = 0usize;
    for u in welt.PlayerVehicleEnum(z.spieler) { if ns >= 8 { break; } if welt.IsLive(u) && welt.GetType(u) == mapSpider { sp[ns] = u; ns += 1; } }
    if ns < 2 { return; }
    let angriff = z.kampfModus >= 2 && (z.zielEmp > 0 || z.empRakete != 0 || z.kampfModus == 4);
    let (mut mx, mut my, mut k) = (0, 0, 0);
    if angriff { for v in AngriffEnum(welt, z) { if welt.IsLive(v) && welt.IsVehicle(v) { let l = welt.Location(v); mx += l.x; my += l.y; k += 1; } } }
    for i in 1..ns {
        let s = sp[i];
        if welt.GetCurAction(s) != moDone || welt.IsBusy(s) { continue; }
        let sl = welt.Location(s);
        let mut ziel: Id = 0; let mut best = 12 * 12 + 1;
        let mut p = 0;
        while p < welt.NoPlayers() {
            if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { p += 1; continue; }
            for u in welt.PlayerVehicleEnum(p) {
                if !welt.IsLive(u) || !welt.isEMPed(u) { continue; }
                let l = welt.Location(u); let d = abstand2(welt, sl.x, sl.y, l.x, l.y);
                if d < best { best = d; ziel = u; }
            }
            p += 1;
        }
        if ziel != 0 { welt.DoReprogram(s, ziel); z.spinneZiel[i] = ziel; continue; }
        if angriff && k != 0 { let hin = Loc::new(mx / k, my / k); if abstand2(welt, sl.x, sl.y, hin.x, hin.y) > 6 * 6 { welt.DoMove(s, hin); } }
        else if !angriff {
            let (ex, ey) = (edX(welt, z, sl), edY(welt, z, sl));
            if abstand2(welt, ex, ey, z.bx, z.by) > 25 * 25 { let hin = engine(welt, z, z.bx, z.by + 5); welt.DoMove(s, hin); }
        }
    }
}
