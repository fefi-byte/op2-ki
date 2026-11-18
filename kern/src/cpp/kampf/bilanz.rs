//! Angriffsbilanz: Einsatz, Ankunft, Kontakt, Ende (was kostete ein Angriff, was kam zurueck).
use super::*;

// --- Angriffsbilanz: was hat ein Angriff gekostet, was kam zurueck, was wurde zerstoert. Nur Messung.
/// C++: void mixDazu(short* typ, short* wf, short* anz, int& n, int t, int w)  (op2_ki.hpp:3982)
pub fn mixDazu(_welt: &mut dyn Welt, typ: &mut [i16], wf: &mut [i16], anz: &mut [i16], n: &mut i32, t: i32, w: i32) {
    for i in 0..*n as usize { if typ[i] as i32 == t && wf[i] as i32 == w { anz[i] += 1; return; } }
    if *n < 12 { let k = *n as usize; typ[k] = t as i16; wf[k] = w as i16; anz[k] = 1; *n += 1; }
}

/// C++: void fzKosten(int spieler, UnitEx& v, int& erz, int& sel)  (op2_ki.hpp:3986)
pub fn fzKosten(welt: &mut dyn Welt, spieler: i32, v: &mut Id, erz: &mut i32, sel: &mut i32) {
    let c = welt.GetType(*v);
    *erz += welt.GetOreCost(c, spieler); *sel += welt.GetRareOreCost(c, spieler);
    let w = welt.GetWeapon(*v);
    if w > 0 && w != mapNone && w < 128 { *erz += welt.GetOreCost(w, spieler); *sel += welt.GetRareOreCost(w, spieler); }
}

/// C++: void angriffsGruppe(Zustand& z, int& erz, int& sel, short* typ, short* wf, short* anz, int& n)  (op2_ki.hpp:3993)
/// typ == nullptr in C++ -> leeres Feld.
pub fn angriffsGruppe(welt: &mut dyn Welt, z: &mut Zustand, erz: &mut i32, sel: &mut i32, typ: &mut [i16], wf: &mut [i16], anz: &mut [i16], n: &mut i32) {
    *erz = 0; *sel = 0; *n = 0;
    let mut ids = [0i32; 64]; let mut ni = 0usize;
    for v0 in AngriffEnum(welt, z) {
        let mut v = v0;
        if !welt.IsLive(v) || !welt.IsVehicle(v) { continue; }
        let mut schon = false;
        for i in 0..ni { if ids[i] == v { schon = true; break; } }
        if schon { continue; }
        if ni < 64 { ids[ni] = v; ni += 1; }
        fzKosten(welt, z.spieler, &mut v, erz, sel);
        if !typ.is_empty() { let t = welt.GetType(v); let w = welt.GetWeapon(v); mixDazu(welt, typ, wf, anz, n, t, w); }
    }
}

/// C++: void bilanzStart(Zustand& z)  (op2_ki.hpp:4008)
pub fn bilanzStart(welt: &mut dyn Welt, z: &mut Zustand) {
    z.bilStart = welt.Tick(); z.bilEmp = if z.empRakete != 0 { 1 } else { 0 };
    {
        let (mut e, mut s, mut n) = (z.bilEinsatzErz, z.bilEinsatzSelten, z.bilN);
        let (mut ty, mut wf, mut an) = (z.bilTyp, z.bilWaffe, z.bilAnz);
        angriffsGruppe(welt, z, &mut e, &mut s, &mut ty, &mut wf, &mut an, &mut n);
        z.bilEinsatzErz = e; z.bilEinsatzSelten = s; z.bilN = n; z.bilTyp = ty; z.bilWaffe = wf; z.bilAnz = an;
    }
    { let mut nn = 0; z.bilE = angriffWert(welt, z, &mut nn); z.bilGruppe = nn; }
    z.bilArmee = z.feindArmee; z.bilKontakt = 0; z.bilErste = 0; z.bilNach = -1;
    z.bilZerHeim = z.zerstoertHeim; z.bilVerHeim = z.verlorenHeim;
    for i in 0..6 { z.bilK[i] = -1; }
    for i in 0..3 { z.bilK2[i] = -1; }
    z.bilKTick = 0;
    z.bilRueck = 0; z.bilRueckErz = 0; z.bilUeber = 0;
    z.bilD = z.feindArmee;
    z.bilFN = 0; z.bilFeindWT = 0;
    let mut p = 0;
    while p < welt.NoPlayers() {
        if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { p += 1; continue; }
        for v in welt.PlayerVehicleEnum(p) {
            if welt.IsLive(v) && kampfWert(welt, v) > 0 {
                let t = welt.GetType(v); let w = welt.GetWeapon(v);
                mixDazu(welt, &mut z.bilFTyp, &mut z.bilFWaffe, &mut z.bilFAnz, &mut z.bilFN, t, w);
            }
        }
        for g in welt.PlayerBuildingEnum(p, mapGuardPost) { if welt.IsLive(g) { z.bilFeindWT += 1; z.bilD += kampfWert(welt, g); } }
        p += 1;
    }
}

/// C++: void bilanzAnkunft(Zustand& z)  (op2_ki.hpp:4026)
pub fn bilanzAnkunft(welt: &mut dyn Welt, z: &mut Zustand) {
    if z.bilStart == 0 || z.angriffZiel == 0 { return; }
    let ziel: Id = z.angriffZiel;
    if !welt.IsLive(ziel) { return; }
    let zl = welt.Location(ziel);
    let mut nah = 0;
    for v in AngriffEnum(welt, z) {
        if welt.IsLive(v) && welt.IsVehicle(v) { let l = welt.Location(v); if abstand2(welt, l.x, l.y, zl.x, zl.y) <= 9 * 9 { nah += 1; } }
    }
    let t = welt.Tick();
    if z.bilKontakt == 0 { if nah != 0 { z.bilKontakt = t; z.bilErste = nah; } }
    else if z.bilNach < 0 && t - z.bilKontakt >= 200 { z.bilNach = nah; }
}

/// C++: void kontaktStaerken(Zustand& z, int kx, int ky, int& nah, int& alle, int& e, int& d)  (op2_ki.hpp:4039)
pub fn kontaktStaerken(welt: &mut dyn Welt, z: &mut Zustand, kx: i32, ky: i32, nah: &mut i32, alle: &mut i32, e: &mut i32, d: &mut i32) {
    *nah = 0; *alle = 0; *e = 0; *d = 0;
    let mut ids = [0i32; 64]; let mut ni = 0usize;
    for v in AngriffEnum(welt, z) {
        if !welt.IsLive(v) || !welt.IsVehicle(v) { continue; }
        let mut schon = false; for i in 0..ni { if ids[i] == v { schon = true; } }
        if schon { continue; }
        if ni < 64 { ids[ni] = v; ni += 1; }
        *alle += 1; let l = welt.Location(v);
        if abstand2(welt, l.x, l.y, kx, ky) <= 12 * 12 { *nah += 1; *e += kampfWert(welt, v); }
    }
    let mut p = 0;
    while p < welt.NoPlayers() {
        if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { p += 1; continue; }
        for u in welt.PlayerVehicleEnum(p) {
            if welt.IsLive(u) && kampfWert(welt, u) > 0 { let l = welt.Location(u); if abstand2(welt, l.x, l.y, kx, ky) <= 12 * 12 { *d += kampfWert(welt, u); } }
        }
        for u in welt.PlayerBuildingEnum(p, mapGuardPost) {
            if welt.IsLive(u) { let l = welt.Location(u); if abstand2(welt, l.x, l.y, kx, ky) <= 12 * 12 { *d += kampfWert(welt, u); } }
        }
        p += 1;
    }
}

/// C++: void bilanzKontakt(Zustand& z)  (op2_ki.hpp:4056)
pub fn bilanzKontakt(welt: &mut dyn Welt, z: &mut Zustand) {
    if z.bilStart == 0 || z.bilK2[0] >= 0 { return; }
    let t = welt.Tick();
    if z.bilK[0] >= 0 {
        if t - z.bilKTick < 300 { return; }
        let (mut nah, mut alle, mut e, mut d) = (0, 0, 0, 0);
        let (kx, ky) = (z.bilK[4] >> 16, z.bilK[4] & 0xffff);
        kontaktStaerken(welt, z, kx, ky, &mut nah, &mut alle, &mut e, &mut d);
        z.bilK2[0] = nah; z.bilK2[1] = e; z.bilK2[2] = d;
        return;
    }
    let mut fund: Id = 0; let mut gef = false;
    for v in AngriffEnum(welt, z) {
        if gef { break; }
        if !welt.IsLive(v) || !welt.IsVehicle(v) { continue; }
        let l = welt.Location(v);
        let mut p = 0;
        while p < welt.NoPlayers() && !gef {
            if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { p += 1; continue; }
            for u in welt.PlayerVehicleEnum(p) {
                if gef { break; }
                if welt.IsLive(u) && kampfWert(welt, u) > 0 { let m = welt.Location(u); if abstand2(welt, l.x, l.y, m.x, m.y) <= 9 * 9 { gef = true; fund = v; } }
            }
            for u in welt.PlayerBuildingEnum(p, mapGuardPost) {
                if gef { break; }
                if welt.IsLive(u) { let m = welt.Location(u); if abstand2(welt, l.x, l.y, m.x, m.y) <= 9 * 9 { gef = true; fund = v; } }
            }
            p += 1;
        }
    }
    if !gef { return; }
    let fl = welt.Location(fund);
    let (mut nah, mut alle, mut e, mut d) = (0, 0, 0, 0);
    kontaktStaerken(welt, z, fl.x, fl.y, &mut nah, &mut alle, &mut e, &mut d);
    z.bilK[0] = t - z.bilStart; z.bilK[1] = z.kampfModus; z.bilK[2] = nah; z.bilK[3] = alle; z.bilK[4] = (fl.x << 16) | (fl.y & 0xffff); z.bilK[5] = e | (d << 16);
    z.bilKTick = t;
}


/// C++: void mixSchreiben(Puffer& p, const short* typ, const short* wf, const short* anz, int n)  (op2_ki.hpp:4190)
pub fn mixSchreiben(welt: &mut dyn Welt, p: &mut Puffer, typ: &[i16], wf: &[i16], anz: &[i16], n: i32) {
    p.roh("[");
    for i in 0..n as usize {
        if i != 0 { p.roh(","); }
        p.roh("["); let nt = name(welt, typ[i] as i32); p.str(nt); p.roh(",");
        let nw = if wf[i] > 0 && wf[i] as i32 != mapNone { name(welt, wf[i] as i32) } else { "-" };
        p.str(nw); p.f(&format!(",{}]", anz[i]));
    }
    p.roh("]");
}

/// C++: void bilanzEnde(Zustand& z, int zer, int ver, const char* abbruch)  (op2_ki.hpp:4198)
pub fn bilanzEnde(welt: &mut dyn Welt, z: &mut Zustand, zer: i32, ver: i32, abbruch: &str) {
    if z.bilStart == 0 { return; }
    let (mut erz, mut sel, mut n) = (0, 0, 0);
    angriffsGruppe(welt, z, &mut erz, &mut sel, &mut [], &mut [], &mut [], &mut n);
    erz += z.bilRueckErz;                                   // heimgekehrte zaehlen als zurueck
    let mut p = Puffer::default();
    let tk = welt.Tick();
    p.f(&format!("{{\"t\":{},\"art\":\"angriff\",\"p\":{},\"start\":{},\"dauer\":{},\"angriffArt\":{},\"emp\":{},\
\"einsatz\":[{},{}],\"zurueck\":[{},{}],\"zerstoert\":{},\"verloren\":{},\"uebernommen\":{},\"feindWT\":{},\"E\":{},\"D\":{},\"armee\":[{},{}],\"ankunft\":[{},{},{},{}],\"heim\":[{},{}],\"kontakt\":[{},{},{},{},{},{},{},{},{}],\"eigene\":",
        tk, z.spieler, z.bilStart, welt.Tick() - z.bilStart, z.angriffArt, z.bilEmp,
        z.bilEinsatzErz, z.bilEinsatzSelten, erz, sel, zer, ver, z.bilUeber, z.bilFeindWT, z.bilE, z.bilD, z.bilArmee, z.feindArmee,
        if z.bilKontakt != 0 { z.bilKontakt - z.bilStart } else { -1 }, z.bilErste, z.bilNach, z.bilGruppe, z.zerstoertHeim - z.bilZerHeim, z.verlorenHeim - z.bilVerHeim,
        z.bilK[0], z.bilK[1], z.bilK[2], z.bilK[3], if z.bilK[0] >= 0 { z.bilK[5] & 0xffff } else { -1 }, if z.bilK[0] >= 0 { z.bilK[5] >> 16 } else { -1 }, z.bilK2[0], z.bilK2[1], z.bilK2[2]));
    let (ty, wf, an, bn) = (z.bilTyp, z.bilWaffe, z.bilAnz, z.bilN);
    mixSchreiben(welt, &mut p, &ty, &wf, &an, bn);
    p.roh(",\"feind\":");
    let (ty, wf, an, bn) = (z.bilFTyp, z.bilFWaffe, z.bilFAnz, z.bilFN);
    mixSchreiben(welt, &mut p, &ty, &wf, &an, bn);
    p.roh(",\"ende\":"); p.str(abbruch);
    p.roh("}");
    p.schreiben(welt);
    z.bilStart = 0;
}
