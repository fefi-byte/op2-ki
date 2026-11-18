//! Selbstpruefung (Widersprueche erkennen), Stillstandsgruende, Vorhersage in die Spur.
use super::*;
use super::GEBAEUDE;

/// C++: const char* stillstandGrund(UnitEx u)  (op2_ki.hpp:5661)
// Rueckgabe "" steht fuer nullptr (kein Stillstand).
pub fn stillstandGrund(welt: &mut dyn Welt, u: Id) -> &'static str {
    if u == 0 || !welt.IsLive(u) { return "fehlt"; }
    if welt.GetCurAction(u) == moObjWait || welt.GetCurAction(u) == moDevelop { return "im Bau"; }
    // Gebaeude ohne Strom- und Personalbedarf (Minen, ...) koennen nicht stillstehen
    {
        let ty = welt.GetType(u);
        if welt.GetPowerRequired(ty, 0) <= 0 && welt.GetWorkersRequired(ty, 0) <= 0 && welt.GetScientistsRequired(ty, 0) <= 0 { return ""; }
    }
    if welt.IsBusy(u) && !welt.HasPower(u) && !welt.HasWorkers(u) && welt.GetCurAction(u) == moDone { return "stillgelegt"; }
    if !welt.HasPower(u) { return "ohne Strom"; }
    if !welt.HasWorkers(u) { return "ohne Personal"; }
    ""
}

/// C++: void selbstpruefung(Zustand& z, const Lage& l)  (op2_ki.hpp:5672)
// Selbstpruefung: Widersprueche zwischen "wird dringend gebraucht" und "tut sich nichts", mit Ursache. Meldet als "diagnose".
pub fn selbstpruefung(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    let t = welt.Tick();
    if t < z.naechstePruefung { return; }
    z.naechstePruefung = t + 2000;
    // 1. Labore, die die Forschung braucht
    const LABTYP: [i32; 4] = [0, mapBasicLab, mapStandardLab, mapAdvancedLab];
    for lt in 2..=3usize {
        if z.laborNutzen[lt] < 30 { continue; }
        let mut grund: &'static str = "fehlt";
        for lab in welt.PlayerBuildingEnum(z.spieler, LABTYP[lt]) {
            if !welt.IsLive(lab) { continue; } // gerade zerstoert
            grund = stillstandGrund(welt, lab);
            if grund.is_empty() { break; }
        }
        if !grund.is_empty() && grund != "im Bau" {
            let nm = name(welt, LABTYP[lt]);
            let ln = z.laborNutzen[lt];
            melde(welt, z, "diagnose", LABTYP[lt], z.bx, z.by, &format!("Blockade: die Forschung braucht ein {} (Dringlichkeit {}), es {}", nm, ln, grund));
        }
    }
    // 2. Kampfwert fehlt, aber seit langem kein Kampffahrzeug
    {
        let fehlt = kampfFehlt(welt, z, l);
        let mob = mobilSoll(welt, z, true) - z.eigeneMobil;
        if fehlt > 0 || mob > 0 { if z.kampfBedarfSeit == 0 { z.kampfBedarfSeit = t; } } else { z.kampfBedarfSeit = 0; }
        let seit = if z.letztesKampfFz > z.kampfBedarfSeit { z.letztesKampfFz } else { z.kampfBedarfSeit };
        if z.kampfBedarfSeit != 0 && t - seit > 5000 {
            let grund: String;
            let fz = fahrgestellWahl(welt, z, 1 << 20);
            if fz == 0 {
                let lab = laborImWeg(welt, z, l, 3851);
                let al: Id = welt.PlayerBuildingEnum(z.spieler, mapAdvancedLab).first().copied().unwrap_or(0);
                let lg = stillstandGrund(welt, al);
                let labName = if lab != 0 { name(welt, lab) } else { "" };
                grund = kurz(format!("kein Kampffahrzeug erforscht (Lynx 3851){}{}{}{}",
                    if lab != 0 { ", es fehlt ein " } else { "" }, labName,
                    if lab == 0 && !lg.is_empty() { ", Advanced Lab " } else { "" }, if lab == 0 && !lg.is_empty() { lg } else { "" }), 200);
            } else if l.vf == 0 {
                grund = "keine Fahrzeugfabrik".to_string();
            } else {
                let vg = stillstandGrund(welt, l.vf);
                if !vg.is_empty() {
                    grund = kurz(format!("Fahrzeugfabrik {}", vg), 200);
                } else {
                    let nm = name(welt, fz);
                    grund = kurz(format!("{} baubar, Erz {} / Seltenerz {} -- anderes Fahrzeug hatte Vorrang?", nm, l.erz, l.selten), 200);
                }
            }
            let ms = mobilSoll(welt, z, true);
            let text = format!("Blockade: Kampfwert fehlt seit {} Ticks (Soll {}, eigener {}, mobil {}/{}), kein Kampffahrzeug -- {}",
                t - z.kampfBedarfSeit, z.sollArmee, z.eigeneArmee, z.eigeneMobil, ms, grund);
            melde(welt, z, "diagnose", if fz != 0 { fz } else { mapLynx }, z.bx, z.by, &text);
        }
    }
    // 3. Hohe Bau-Dringlichkeit, aber seit langem kein Bau
    {
        let d = dringlichkeit(welt, z, l);
        let mut m = d.kolonie;
        let mut b = "Kolonie";
        if d.wirtschaft > m { m = d.wirtschaft; b = "Wirtschaft"; }
        if d.verteidigung > m { m = d.verteidigung; b = "Verteidigung"; }
        if m >= 60 { if z.bauDrangSeit == 0 { z.bauDrangSeit = t; } } else { z.bauDrangSeit = 0; }
        let seit = if z.letzterBau > z.bauDrangSeit { z.letzterBau } else { z.bauDrangSeit };
        if z.bauDrangSeit != 0 && t - seit > 5000 {
            let w = bauWahl(welt, z, l);
            let text = format!("Blockade: {} hat Dringlichkeit {} seit {} Ticks, nichts gebaut -- {}",
                b, m, t - z.bauDrangSeit, if !w.warum.is_empty() { w.warum.as_str() } else { "kein baubarer Kandidat" });
            melde(welt, z, "diagnose", w.typ, z.bx, z.by, &text);
        }
    }
    // 4. Wichtige Gebaeude (Rang >= 60), die stillstehen
    for &ty in GEBAEUDE.iter() {
        if ty == mapGuardPost { continue; }
        let r = rang(welt, z, l, ty);
        if r < 60 { continue; }
        for u in welt.PlayerBuildingEnum(z.spieler, ty) {
            let g = stillstandGrund(welt, u);
            if !g.is_empty() && g != "im Bau" {
                let lo = welt.Location(u);
                let x = edX(welt, z, lo);
                let y = edY(welt, z, lo);
                let nm = name(welt, ty);
                melde(welt, z, "diagnose", ty, x, y, &format!("Stillstand: {} (Rang {}) {}", nm, r, g));
            }
        }
    }
}

/// C++: void vorhersageMelden(Zustand& z, const Lage& l)  (op2_ki.hpp:5743)
// Spur: die besten Vorhersagen je Bereich (alle 10000 Ticks)
pub fn vorhersageMelden(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    let t = welt.Tick();
    if t < z.naechsteVorhersage { return; }
    z.naechsteVorhersage = t + 10000;
    for b in 0..=2i32 {
        let mut v = [Vorhersage::default(); 4];
        let n = bereichZiele(welt, z, l, b, &mut v, 4);
        // char text[400]; pos = Laenge (snprintf-Rueckgabe), am Ende auf 399 Bytes gekuerzt
        let mut text = format!("Vorhersage {}:", tb::BEREICH_NAME[b as usize]);
        let mut pos = text.len();
        let mut i = 0usize;
        while (i as i32) < n && pos < 330 {
            let idx = techIndex(welt, v[i].id);
            let nm: &str = if idx >= 0 { techName(welt, idx).unwrap_or("?") } else { "?" };
            let nm24: String = nm.chars().take(24).collect();
            let s = format!(" {} {} +{:.1} fuer {};", v[i].id, nm24, v[i].nutzen, v[i].kosten);
            pos += s.len();
            text.push_str(&s);
            i += 1;
        }
        if n == 0 { text.push_str(" nichts mit Nutzen"); }
        let text = kurz(text, 400);
        melde(welt, z, "vorhersage", mapNone, z.bx, z.by, &text);
    }
}
