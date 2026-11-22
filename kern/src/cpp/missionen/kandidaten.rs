//! Moegliche Missionen und ihre Bewertung in Erz: Wert (Zielwert, Erzfluss ueber den Horizont, Sieg) minus erwarteter
//! eigener Verlust (Lanchester gegen die Abwehr vor Ort). Abwehr vor Ort = Tuerme in 7 Kacheln + Feindarmee nach
//! Entfernung; wer schneller am Ort ist als unser Pool, zaehlt voll.
use super::*;

pub struct Kandidat { pub art: i32, pub x: i32, pub y: i32, pub ziel: Id, pub wert: f64, pub verlust: f64, pub netto: f64, pub soll: i32, pub dd: f64, pub warum: String }

/// Was die Kandidatensuche ueber die Lage braucht (Kacheln, Kampfwerte)
pub struct Bild { pub bauer: Vec<(i32, i32)>, pub feind: Vec<(i32, i32, i32)>, pub tuerme: Vec<(i32, i32, i32)>, pub geb: Vec<(Id, i32, i32, i32, i32)>, pub lkw: Vec<(Id, i32, i32)>,
                  pub eigenGeb: Vec<(i32, i32, i32)>, pub pool: (i32, i32), pub kraft: i32, pub basis: Option<(i32, i32)>, pub feindArmee: i32 }

fn d(a: (i32, i32), b: (i32, i32)) -> f64 { (((a.0 - b.0).pow(2) + (a.1 - b.1).pow(2)) as f64).sqrt() }

pub fn bild(welt: &mut dyn Welt, z: &Zustand) -> Bild {
    let sp = z.spieler;
    let (mut feind, mut tuerme, mut geb, mut lkw, mut bauer) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for p in 0..welt.NoPlayers() {
        if p == sp || welt.IsAlliedTo(sp, p) { continue; }
        for u in welt.PlayerVehicleEnum(p) {
            if !welt.IsLive(u) { continue; }
            let lo = welt.Location(u); let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
            let ty = welt.GetType(u);
            if ty == mapCargoTruck { lkw.push((u, x, y)); continue; }
            if ty == mapConVec || ty == mapRoboMiner || ty == mapRoboSurveyor { bauer.push((x, y)); continue; }
            let w = kampfWert(welt, u); if w > 0 { feind.push((x, y, w)); }
        }
        for &ty in GEBAEUDE.iter() {
            for b in welt.PlayerBuildingEnum(p, ty) {
                if !welt.IsLive(b) { continue; }
                let lo = welt.Location(b); let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
                if ty == mapGuardPost { let w = kampfWert(welt, b); tuerme.push((x, y, w)); continue; }
                let w = super::super::truppplan::zielWert(welt, ty, p);
                geb.push((b, ty, x, y, w));
            }
        }
    }
    let mut eigenGeb = Vec::new();
    for &ty in GEBAEUDE.iter() {
        for b in welt.PlayerBuildingEnum(sp, ty) {
            if !welt.IsLive(b) { continue; }
            let lo = welt.Location(b); let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
            eigenGeb.push((x, y, super::super::truppplan::zielWert(welt, ty, sp)));
        }
    }
    // eigene Lastwagen gehoeren zur Wirtschaft, die die Abwehr schuetzt
    for u in welt.PlayerVehicleEnum(sp) {
        if !welt.IsLive(u) || welt.GetType(u) != mapCargoTruck { continue; }
        let lo = welt.Location(u); eigenGeb.push((edX(welt, z, lo), edY(welt, z, lo), 1400));
    }
    let pool = z.missPool;
    let (pkw, _, ppos, pn) = gruppenLage(welt, z, pool);
    let mut kraft = pkw;
    for i in 0..8 { if z.missArt[i] != 0 { let (kw, _, _, _) = gruppenLage(welt, z, z.missGruppe[i]); kraft += kw; } }
    let feindArmee = feind.iter().map(|f| f.2).sum();
    Bild { bauer, feind, tuerme, geb, lkw, eigenGeb, pool: if pn > 0 { ppos } else { poolPunkt(welt, z) }, kraft, basis: super::super::erweiterung::feindBasis(welt, z), feindArmee }
}

/// Abwehr am Ort o: Tuerme in 7 Kacheln voll; Feindfahrzeuge in 10 Kacheln voll, naeher am Ort als unser Pool 0.7, sonst
/// abnehmend bis 40 Kacheln
pub fn abwehr(b: &Bild, o: (i32, i32)) -> f64 {
    let wir = d(b.pool, o);
    let mut a: f64 = b.tuerme.iter().filter(|t| d((t.0, t.1), o) <= 7.0).map(|t| t.2 as f64).sum();
    for &(x, y, w) in b.feind.iter() {
        let df = d((x, y), o);
        let f = if df <= 10.0 { 1.0 } else if df < wir { 0.7 } else { (1.0 - (df - 10.0) / 30.0).clamp(0.0, 1.0) * 0.7 };
        a += w as f64 * f;
    }
    a
}

/// Verlust in Kampfwert, wenn Kraft e auf Abwehr dd trifft (Lanchester, wie das Zermuerben)
pub fn verlustKw(e: f64, dd: f64) -> f64 {
    if dd <= 0.0 { return 0.0; }
    if e >= dd { e * (1.0 - (1.0 - (dd / e).powi(2)).sqrt()) } else { e }
}

fn gesperrt(z: &Zustand, x: i32, y: i32, t: i32) -> bool {
    (0..16).any(|j| z.missSperrBis[j] > t && (z.missSperrX[j] - x).pow(2) + (z.missSperrY[j] - y).pow(2) <= 64)
}

fn kandidat(art: i32, x: i32, y: i32, ziel: Id, wert: f64, dd: f64, warum: String) -> Kandidat {
    let ueb = 1.5;
    let soll = ((dd * ueb).ceil() as i32).max(if art == ABWEHR { 10 } else { 20 });   // keine 2-Lynx-Grueppchen
    let verlust = verlustKw(soll as f64, dd) * ERZ_JE_KW;
    Kandidat { art, x, y, ziel, wert, verlust, netto: wert - verlust, soll, dd, warum }
}

/// Alle Kandidaten der Lage
pub fn kandidaten(welt: &mut dyn Welt, z: &Zustand, l: &Lage, t: i32) -> Vec<Kandidat> {
    let b = bild(welt, z);
    let pm = PAR(z.spieler);
    let h = pm.missionHorizont / 1000.0;
    let ueb = pm.missionUeberleg.max(1.0);
    let mut ks: Vec<Kandidat> = Vec::new();
    let mut neu = |k: Kandidat| {
        let mut k = k;
        // gelernt: geschaetzte Abwehr mal Korrektur der Art (nur mit missionLernen), dann Soll und Verlust neu
        let f = super::lernen::korrektur(z, k.art);
        if f != 1.0 { let roh = k.dd; k = kandidat(k.art, k.x, k.y, k.ziel, k.wert, roh * f, std::mem::take(&mut k.warum)); k.dd = roh; }
        k.soll = ((k.soll as f64) * ueb / 1.5).ceil() as i32;
        if !gesperrt(z, k.x, k.y, t) { ks.push(k); }
    };
    // Ueberfall: Wirtschaft (Kraftwerke, Schmelzen, Minen, Lager, Fabriken) und Lastwagen
    for &(id, ty, x, y, w) in b.geb.iter() {
        let k = super::super::truppplan::zielKlasse(ty);
        if !matches!(k, Some(1) | Some(2) | Some(3) | Some(5)) && ty != mapCommonStorage && ty != mapRareStorage { continue; }
        let dd = abwehr(&b, (x, y));
        neu(kandidat(UEBERFALL, x, y, id, w as f64, dd, format!("{} (Abwehr vor Ort {:.0})", name(welt, ty), dd)));
    }
    for &(id, x, y) in b.lkw.iter() {
        let dd = abwehr(&b, (x, y));
        // Lastwagen: Kosten + die Ladung, die er in der Zeit nicht faehrt
        neu(kandidat(UEBERFALL, x, y, id, 1400.0 + 0.3 * 1400.0 * h, dd, format!("Lastwagen (Abwehr vor Ort {:.0})", dd)));
    }
    // Entscheidungsschlag: Zentrale und Strukturfabrik; Wert = Sieg anteilig (beide muessen fallen)
    for &(id, ty, x, y, _) in b.geb.iter() {
        if ty != mapCommandCenter && ty != mapStructureFactory { continue; }
        let n = b.geb.iter().filter(|g| g.1 == mapCommandCenter || g.1 == mapStructureFactory).count().max(1);
        // auch gegen das, was der Feind in der Reaktionszeit aufstellt (Armee + Fabriken)
        let dd = abwehr(&b, (x, y)).max(z.feindPotenzial as f64);
        neu(kandidat(ENTSCHEIDUNG, x, y, id, pm.missionSieg / n as f64, dd, format!("{} ({} von {} fuer den Sieg, Abwehr {:.0})", name(welt, ty), 1, n, dd)));
    }
    // Erz halten: Baken ausserhalb der eigenen Basis ohne Feindmine; Wert = Erzfluss ueber den Horizont, gewichtet mit dem
    // Druck des Feindes (naeher an seiner Basis = umkaempft)
    for k in 0..z.kbN.max(0) as usize {
        let (x, y) = (z.kbX[k] as i32, z.kbY[k] as i32);
        if d((x, y), (z.bx, z.by)) <= AUSSEN_R as f64 { continue; }
        if b.geb.iter().any(|g| g.1 == mapCommonOreMine && d((g.2, g.3), (x, y)) <= 3.0) { continue; }
        let eigen = b.eigenGeb.iter().any(|g| d((g.0, g.1), (x, y)) <= 3.0 && g.2 > 1400);
        // umkaempft: eigene Mine dort, oder ein feindlicher Bauer (ConVec, Robo-Miner, Surveyor) in 10 Kacheln
        let umkaempft = eigen || b.bauer.iter().any(|q| d(*q, (x, y)) <= 10.0);
        if !umkaempft { continue; }
        let druck = b.basis.map_or(0.5, |fb| (d((z.bx, z.by), (x, y)) / (d((z.bx, z.by), (x, y)) + d(fb, (x, y))).max(1.0)).clamp(0.0, 1.0));
        let dd = abwehr(&b, (x, y)).max(b.feindArmee as f64 * 0.25 * druck);
        let wert = 1400.0 * h * if eigen { 1.0 } else { 0.5 } * (0.3 + 0.7 * druck);
        neu(kandidat(HALTEN_ERZ, x, y, 0, wert, dd, format!("Bake{} (Druck {:.2})", if eigen { " mit eigener Mine" } else { ", Feind will dort bauen" }, druck)));
    }
    // Pass halten und Vorposten: auf den Wegen von der Feindbasis zu uns
    if let Some(fb) = b.basis {
        let rs = super::super::routen::routen(welt, z, fb, (z.bx, z.by));
        let r = super::super::routen::raster(welt, z);
        let kolonie: f64 = b.eigenGeb.iter().map(|g| g.2 as f64).sum();
        for ro in rs.iter() {
            let n = ro.weg.len();
            if n < 20 { continue; }
            // engste Stelle im mittleren Teil des Wegs
            let (mut best, mut bp) = (i32::MAX, ro.weg[n / 2]);
            for &(x, y) in ro.weg[n / 4..n * 3 / 4].iter().step_by(2) {
                let raum = super::super::routen::freiraum(&r, x, y);
                if raum < best { best = raum; bp = (x, y); }
            }
            let anteil = (b.feindArmee as f64 / (b.kraft.max(1) as f64)).min(2.0);
            let dd = (b.feindArmee as f64 * 0.4).max(abwehr(&b, bp));
            neu(kandidat(HALTEN_PASS, bp.0, bp.1, 0, 0.06 * kolonie * anteil, dd * 0.6, format!("engste Stelle {} Kacheln auf dem Weg der Feindbasis zu uns", 2 * best - 1)));
            // Vorposten: 16..24 Wegkacheln vor der Feindbasis, ausser Turmreichweite
            for a in 16..=24usize {
                if a >= n { break; }
                let p = ro.weg[a];
                if b.tuerme.iter().any(|t| d((t.0, t.1), p) <= 9.0) { continue; }
                let druck: f64 = b.geb.iter().filter(|g| d((g.2, g.3), p) <= 15.0 && g.1 != mapCommandCenter).map(|g| g.4 as f64).sum();
                let lkw = b.lkw.iter().filter(|w| d((w.1, w.2), p) <= 15.0).count() as f64;
                let dd = abwehr(&b, p);
                neu(kandidat(VORPOSTEN, p.0, p.1, 0, 0.15 * druck + 600.0 * lkw * h / 10.0, dd * 0.8, format!("vor der Feindbasis, {} Wirtschaftswert und {} Lastwagen in 15 Kacheln", druck as i32, lkw)));
                break;
            }
        }
    }
    // Abwehr: Feinde an unseren Gebaeuden
    let mut ziele: Vec<(i32, i32, i32)> = Vec::new();
    for &(x, y, w) in b.feind.iter() {
        if b.eigenGeb.iter().any(|g| d((g.0, g.1), (x, y)) <= 14.0) { ziele.push((x, y, w)); }
    }
    if !ziele.is_empty() {
        let n = ziele.len() as i32;
        let m = (ziele.iter().map(|e| e.0).sum::<i32>() / n, ziele.iter().map(|e| e.1).sum::<i32>() / n);
        let fw: i32 = ziele.iter().map(|e| e.2).sum();
        let bedroht: f64 = b.eigenGeb.iter().filter(|g| d((g.0, g.1), m) <= 14.0).map(|g| g.2 as f64).sum();
        let k = kandidat(ABWEHR, m.0, m.1, 0, bedroht.max(fw as f64 * ERZ_JE_KW), fw as f64 * 1.2 / 1.5, format!("{} Feindfahrzeuge (Kampfkraft {}) an der Kolonie", n, fw));
        ks.push(k);
    }
    let _ = l;
    ks
}
