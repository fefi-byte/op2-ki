//! EMP-Rakete bauen, Staerke an einem Ort, Reservegruppe (Eingreifen) fuehren.
use super::*;

/// C++: void empRaketeBauen(Zustand& z, const Lage& l)  (op2_ki.hpp:4376)
/// EMP-Rakete bauen, sobald Erz und Seltenerz reichen und sie sich gegen die jetzige Abwehr lohnt.
pub fn empRaketeBauen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    let t = welt.Tick();
    if !empMoeglich(welt, z) || t % 200 >= TAKT { return; }
    let mut sp: Id = 0;
    for u in welt.PlayerBuildingEnum(z.spieler, mapSpaceport) { if welt.IsLive(u) && welt.GetCurAction(u) != moObjWait { sp = u; break; } }
    if sp == 0 { z.empRakete = 0; z.empAuftrag = 0; return; }   // kein (fertiger) Raumhafen: auch keine Rakete
    if z.empRakete != 0 { return; }
    if z.empAuftrag != 0 {
        // fertig, sobald der Raumhafen nicht mehr baut
        if t - z.empAuftragSeit > 200 && welt.GetCurAction(sp) != moDevelop && !welt.IsBusy(sp) {
            z.empAuftrag = 0; z.empRakete = 1;
            let sl = welt.Location(sp); let (ex, ey) = (edX(welt, z, sl), edY(welt, z, sl));
            melde(welt, z, "kampf", mapEMPMissile, ex, ey, "EMP-Rakete steht auf der Rampe");
        }
        return;
    }
    if welt.IsBusy(sp) || welt.GetCurAction(sp) != moDone { return; }
    if l.erz < welt.GetOreCost(mapEMPMissile, z.spieler) || l.selten < welt.GetRareOreCost(mapEMPMissile, z.spieler) { return; }
    let mut wert = 0;
    if !empLohnt(welt, z, Some(&mut wert)) { return; }
    welt.DoProduce(sp, mapEMPMissile, mapNone);
    z.empAuftrag = 1; z.empAuftragSeit = t;
    let sl = welt.Location(sp); let (ex, ey) = (edX(welt, z, sl), edY(welt, z, sl));
    let (ko, ks) = (welt.GetOreCost(mapEMPMissile, z.spieler), welt.GetRareOreCost(mapEMPMissile, z.spieler));
    let kw = empKostenWert(welt, z);
    melde(welt, z, "kampf", mapEMPMissile, ex, ey,
          &format!("EMP-Rakete wird gebaut ({} Erz, {} Seltenerz; dichteste Abwehr Wert {}, Rakete ~{})", ko, ks, wert, kw));
}

/// C++: int feindStaerkeBei(const Zustand& z, int x, int y, int r)  (op2_ki.hpp:4402)
pub fn feindStaerkeBei(welt: &mut dyn Welt, z: &Zustand, x: i32, y: i32, r: i32) -> i32 {
    let mut d = 0;
    let mut p = 0;
    while p < welt.NoPlayers() {
        if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { p += 1; continue; }
        for u in welt.PlayerVehicleEnum(p) {
            if welt.IsLive(u) && kampfWert(welt, u) > 0 {
                let l = welt.Location(u); let (ex, ey) = (edX(welt, z, l), edY(welt, z, l));
                if abstand2(welt, ex, ey, x, y) <= r * r { d += kampfWert(welt, u); }
            }
        }
        p += 1;
    }
    d
}

/// C++: int eigeneGruppeBei(const Zustand& z, ScGroup& g, int x, int y, int r)  (op2_ki.hpp:4411)
pub fn eigeneGruppeBei(welt: &mut dyn Welt, z: &Zustand, g: Gruppe, x: i32, y: i32, r: i32) -> i32 {
    let mut e = 0;
    for v in welt.GroupEnumerator(g) {
        if welt.IsLive(v) && welt.IsVehicle(v) {
            let l = welt.Location(v); let (ex, ey) = (edX(welt, z, l), edY(welt, z, l));
            if abstand2(welt, ex, ey, x, y) <= r * r { e += kampfWert(welt, v); }
        }
    }
    e
}

/// C++: int bereichWert(const Zustand& z, int x, int y, int r)  (op2_ki.hpp:4417)
pub fn bereichWert(welt: &mut dyn Welt, z: &Zustand, x: i32, y: i32, r: i32) -> i32 {
    let mut w = 0;
    for b in welt.PlayerBuildingEnum(z.spieler, mapNone) {
        if welt.IsLive(b) {
            let l = welt.Location(b); let (ex, ey) = (edX(welt, z, l), edY(welt, z, l));
            if abstand2(welt, ex, ey, x, y) <= r * r { w += welt.GetOreCost(welt.GetType(b), z.spieler); }
        }
    }
    w
}

/// C++: void reserveFuehren(Zustand& z)  (op2_ki.hpp:4425)
/// Eingreifreserve: geht dorthin, wo Luecke x Wert am groessten ist -- nur, wenn sie die Luecke schlagen kann.
pub fn reserveFuehren(welt: &mut dyn Welt, z: &mut Zustand) {
    if z.reserveAn == 0 { return; }
    let t = welt.Tick();
    let mut nr = 0; let er = gruppenWert(welt, z.eingreif, &mut nr);
    if nr == 0 { z.reserveOrt = -2; return; }
    const R: i32 = 14;
    let mut besterOrt = -1; let mut bx = z.bx; let mut by = z.by; let mut beste = 0.0f64;
    for i in -1i32..6 {
        if i >= 0 && z.aussenId[i as usize] == 0 { continue; }
        let x = if i < 0 { z.bx } else { z.aussenX[i as usize] }; let y = if i < 0 { z.by } else { z.aussenY[i as usize] };
        let d = feindStaerkeBei(welt, z, x, y, R);
        if d == 0 { continue; }
        let mut vorOrt = eigeneGruppeBei(welt, z, z.wache, x, y, R);
        if i >= 0 && z.aussenAn[i as usize] != 0 { vorOrt += eigeneGruppeBei(welt, z, z.aussenGruppe[i as usize], x, y, R); }
        for g in welt.PlayerBuildingEnum(z.spieler, mapGuardPost) {
            if welt.IsLive(g) {
                let l = welt.Location(g); let (ex, ey) = (edX(welt, z, l), edY(welt, z, l));
                if abstand2(welt, ex, ey, x, y) <= R * R { vorOrt += kampfWert(welt, g); }
            }
        }
        let luecke = d - vorOrt;
        if luecke <= 0 || er < luecke { continue; }
        let score = luecke as f64 * (bereichWert(welt, z, x, y, R) + 1) as f64;
        if score > beste { beste = score; besterOrt = i; bx = x; by = y; }
    }
    // Nicht bei jedem Schwanken umdrehen: mindestens 300 Ticks bleiben
    if besterOrt < 0 && z.reserveOrt >= -1 && t - z.reserveSeit < 300 { return; }
    let ort = if besterOrt >= 0 { besterOrt } else { -3 };
    if ort == z.reserveOrt { return; }
    if ort == -3 { bx = z.bx; by = z.by + 4; freierWarteplatz(welt, z, &mut bx, &mut by, 2, false); }   // daheim
    let warte = Rect::new(engine(welt, z, bx - 2, by - 2), engine(welt, z, bx + 2, by + 2));
    let schutz = Rect::new(engine(welt, z, bx - 8, by - 8), engine(welt, z, bx + 8, by + 8));
    wachen(welt, z.eingreif, warte, &[schutz], 1);
    {
        let zl = engine(welt, z, bx, by);
        for v in welt.GroupEnumerator(z.eingreif) { if welt.IsLive(v) && welt.IsVehicle(v) { welt.DoMove(v, zl); } }
    }
    z.reserveOrt = ort; z.reserveSeit = t;
    let text = if ort == -3 { format!("Reserve ({} Fahrzeuge) zurueck in die Basis", nr) }
               else { format!("Reserve ({} Fahrzeuge) verteidigt den Bereich bei {},{}", nr, bx, by) };
    melde(welt, z, "kampf", mapLynx, bx, by, &text);
}
