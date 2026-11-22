//! Missionen statt festem Basisangriff (Nutzer 2026-11-18, PAR missionen). Die KI waehlt ihre Ziele selbst: Ueberfall
//! (Hit-and-Run auf Wirtschaft), Gebiet halten (Erzfelder/Baken, Paesse/Zugaenge, Vorposten vor der Feindbasis),
//! Entscheidungsschlag (Zentrale/Strukturfabrik) und Abwehr. Jede Mission wird in derselben Waehrung bewertet -- Erz ueber
//! den Horizont (Zielwert, Erzfluss, Verluste) -- und bekommt die Kampfkraft, die sie vor Ort braucht. Es gibt keine
//! Reserve: alles, was die Fabrik baut, kommt in den Pool und von dort in Missionen ("an die Front").
use super::*;

mod ausfuehren;
mod kandidaten;
mod lernen;
pub use ausfuehren::*;
pub use kandidaten::*;

pub const UEBERFALL: i32 = 1;
pub const HALTEN_ERZ: i32 = 2;
pub const HALTEN_PASS: i32 = 3;
pub const VORPOSTEN: i32 = 4;
pub const ENTSCHEIDUNG: i32 = 5;
pub const ABWEHR: i32 = 6;
/// Erz je Kampfwert-Punkt (Laser-Lynx: Kampfwert 5, rund 750 Erz) -- rechnet Kampfkraft in die gemeinsame Waehrung um
pub const ERZ_JE_KW: f64 = 150.0;
const WAHL_TAKT: i32 = 1000;

pub fn artName(a: i32) -> &'static str {
    match a { UEBERFALL => "Ueberfall", HALTEN_ERZ => "Erz halten", HALTEN_PASS => "Pass halten", VORPOSTEN => "Vorposten",
              ENTSCHEIDUNG => "Entscheidungsschlag", ABWEHR => "Abwehr", _ => "?" }
}

pub fn an(z: &Zustand) -> bool { kampfModellNeu() && PAR(z.spieler).missionen >= 0.5 }

/// Kampfwert und Erzwert der lebenden Kaempfer einer Gruppe, dazu ihr Schwerpunkt (Kacheln) und die Anzahl
pub fn gruppenLage(welt: &mut dyn Welt, z: &Zustand, g: Gruppe) -> (i32, i32, (i32, i32), i32) {
    let (mut kw, mut erz, mut sx, mut sy, mut n) = (0, 0, 0, 0, 0);
    for u in welt.GroupEnumerator(g) {
        if !welt.IsLive(u) || !kaempfer(welt, u) { continue; }
        kw += kampfWert(welt, u); erz += super::truppplan::fzWert(welt, u);
        let lo = welt.Location(u); sx += edX(welt, z, lo); sy += edY(welt, z, lo); n += 1;
    }
    (kw, erz, if n > 0 { (sx / n, sy / n) } else { (z.bx, z.by) }, n)
}

/// Sammelpunkt des Pools: vor der eigenen Basis, zum Feind hin
pub fn poolPunkt(welt: &mut dyn Welt, z: &Zustand) -> (i32, i32) {
    let (mut ux, mut uy) = (0.0f64, 0.0f64);
    feindRichtung(welt, z, z.bx, z.by, &mut ux, &mut uy);
    ((z.bx as f64 + ux * 8.0 + 0.5) as i32, (z.by as f64 + uy * 8.0 + 0.5) as i32)
}

/// Alle Kaempfer aus Wache, Sammelgruppe und Reserve in den Pool; der Pool wartet am Sammelpunkt
fn poolFuellen(welt: &mut dyn Welt, z: &mut Zustand) {
    if z.missPoolAn == 0 {
        z.missPool = welt.CreateFightGroup(z.spieler); z.missPoolAn = 1;
        let (px, py) = poolPunkt(welt, z);
        let r = Rect::new(engine(welt, z, px - 2, py - 2), engine(welt, z, px + 2, py + 2));
        let g = Rect::new(engine(welt, z, px - 8, py - 8), engine(welt, z, px + 8, py + 8));
        let pool = z.missPool;
        wachen(welt, pool, r, &[g], 1);
    }
    for quelle in [z.wache, z.sammel, z.eingreif] {
        for u in welt.GroupEnumerator(quelle) {
            if !welt.IsLive(u) || !kaempfer(welt, u) || welt.GetCurAction(u) != moDone { continue; }
            if welt.TotalUnitCount(z.missPool) >= 32 { return; }
            welt.RemoveUnit(quelle, u); welt.TakeUnit(z.missPool, u);
        }
    }
}

/// Je Kampftakt: Pool fuellen, laufende Missionen fuehren, alle WAHL_TAKT Ticks neue Missionen waehlen, Rest an die Front
pub fn fuehren(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32) {
    poolFuellen(welt, z);
    for i in 0..8 { if z.missArt[i] != 0 { missionSchritt(welt, z, i, t); } }
    if t >= z.missNaechste {
        z.missNaechste = t + WAHL_TAKT;
        let ks = kandidaten(welt, z, l, t);
        zuteilen(welt, z, t, &ks);
    }
    restAnDieFront(welt, z, t);
}

fn frei(z: &Zustand) -> Option<usize> { (0..8).find(|&i| z.missArt[i] == 0) }

fn aktiv(z: &Zustand) -> usize { (0..8).filter(|&i| z.missArt[i] != 0).count() }

/// Kandidaten nach Wertung je verlangter Kraft; starten, solange der Pool die verlangte Kraft hat. Abwehr verdraengt
/// die schwaechste Halten-Mission, wenn kein Platz ist.
fn zuteilen(welt: &mut dyn Welt, z: &mut Zustand, t: i32, ks: &[Kandidat]) {
    let max = PAR(z.spieler).missionMax.max(1.0) as usize;
    let mut reihe: Vec<&Kandidat> = ks.iter().filter(|k| k.netto > 0.0).collect();
    reihe.sort_by(|a, b| (b.netto / b.soll.max(5) as f64).partial_cmp(&(a.netto / a.soll.max(5) as f64)).unwrap_or(std::cmp::Ordering::Equal));
    if std::env::var("OP2KI_MISSION_SPUR").is_ok() {
        let pool = z.missPool; let (pkw, _, _, pn) = gruppenLage(welt, z, pool);
        let txt: Vec<String> = reihe.iter().take(6).map(|k| format!("{} {},{} netto {:.0} soll {}", artName(k.art), k.x, k.y, k.netto, k.soll)).collect();
        melde(welt, z, "diagnose", mapNone, 0, 0, &format!("Missionswahl: {} Kandidaten ({} insgesamt), Pool {} Fahrzeuge / Kampfkraft {}, aktiv {}: {}", reihe.len(), ks.len(), pn, pkw, aktiv(z), txt.join(" | ")));
    }
    for k in reihe {
        // schon eine Mission an diesem Ort?
        if (0..8).any(|i| z.missArt[i] != 0 && (z.missX[i] - k.x).pow(2) + (z.missY[i] - k.y).pow(2) <= 36) { continue; }
        // hoechstens 2 Halten-Missionen: lieber stark an wenigen Orten
        let halten = |a: i32| matches!(a, HALTEN_ERZ | HALTEN_PASS | VORPOSTEN);
        if halten(k.art) && (0..8).filter(|&i| halten(z.missArt[i])).count() >= 2 { continue; }
        let pool = z.missPool;
        let (pkw, _, _, pn) = gruppenLage(welt, z, pool);
        if pn == 0 { break; }
        if pkw < k.soll { continue; }
        if aktiv(z) >= max && !(pkw >= 2 * k.soll && aktiv(z) < 8) {
            // kein Platz: Abwehr oder eine klar bessere Mission (1,5-fache Wertung) verdraengt die schwaechste Halten-Mission
            let wert = (k.netto * 1000.0 / k.soll.max(5) as f64) as i32;
            // Hysterese: nur Missionen, die schon 4000 Ticks laufen, und nur fuer die doppelte Wertung (Abwehr: immer)
            let opfer = (0..8).filter(|&i| matches!(z.missArt[i], HALTEN_ERZ | HALTEN_PASS | VORPOSTEN) && z.missPhase[i] < 2 && t - z.missSeit[i] > 4000).min_by_key(|&i| z.missWert[i]);
            match opfer {
                Some(i) if k.art == ABWEHR || wert as f64 > 2.0 * z.missWert[i] as f64 => {
                    let grund = format!("verdraengt von {} (Wertung {} gegen {})", artName(k.art), wert, z.missWert[i]);
                    beenden(welt, z, i, t, &grund, false)
                }
                _ => continue,
            }
        }
        let Some(i) = frei(z) else { break };
        starten(welt, z, i, t, k);
    }
}

/// Mission auf Platz i beginnen: Gruppe, Kaempfer aus dem Pool (die naechsten zum Ziel, bis zur verlangten Kraft)
fn starten(welt: &mut dyn Welt, z: &mut Zustand, i: usize, t: i32, k: &Kandidat) {
    if z.missGruppeAn[i] == 0 { z.missGruppe[i] = welt.CreateFightGroup(z.spieler); z.missGruppeAn[i] = 1; }
    let g = z.missGruppe[i]; let pool = z.missPool;
    let mut us: Vec<(i32, Id, i32)> = Vec::new();
    for u in welt.GroupEnumerator(pool) {
        if !welt.IsLive(u) || !kaempfer(welt, u) { continue; }
        let lo = welt.Location(u); let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
        us.push(((x - k.x).pow(2) + (y - k.y).pow(2), u, kampfWert(welt, u)));
    }
    us.sort();
    let (mut kw, mut n) = (0, 0);
    for &(_, u, w) in us.iter() {
        if kw >= k.soll || n >= 32 { break; }
        welt.RemoveUnit(pool, u); welt.TakeUnit(g, u); kw += w; n += 1;
    }
    let (_, erz, _, _) = gruppenLage(welt, z, g);
    z.missArt[i] = k.art; z.missX[i] = k.x; z.missY[i] = k.y; z.missZiel[i] = k.ziel; z.missPhase[i] = 0; z.missSeit[i] = t;
    z.missStart[i] = erz.max(1); z.missSoll[i] = k.soll; z.missDd[i] = k.dd; z.missKwStart[i] = kw; z.missWert[i] = (k.netto * 1000.0 / k.soll.max(5) as f64) as i32; z.missBefehl[i] = 0;
    melde(welt, z, "kampf", mapLynx, k.x, k.y, &format!("Mission {}: {} bei {},{} -- {} (Wert {:.0}, erwarteter Verlust {:.0} Erz, verlangt Kampfkraft {}, mit {} Fahrzeugen / Kampfkraft {})",
          i, artName(k.art), k.x, k.y, k.warum, k.wert, k.verlust, k.soll, n, kw));
}

/// Mission beenden: Kaempfer zurueck in den Pool; erfolglos -> Ort fuer 6000 Ticks sperren
pub fn beenden(welt: &mut dyn Welt, z: &mut Zustand, i: usize, t: i32, warum: &str, erfolglos: bool) {
    let g = z.missGruppe[i]; let pool = z.missPool;
    let (kw, erz, _, n) = gruppenLage(welt, z, g);
    for u in welt.GroupEnumerator(g) { welt.RemoveUnit(g, u); if welt.IsLive(u) { welt.TakeUnit(pool, u); } }
    if erfolglos {
        let k = (0..16).min_by_key(|&j| z.missSperrBis[j]).unwrap_or(0);
        z.missSperrX[k] = z.missX[i]; z.missSperrY[k] = z.missY[i]; z.missSperrBis[k] = t + 6000;
    }
    let (x, y) = (z.missX[i], z.missY[i]);
    melde(welt, z, "kampf", mapLynx, x, y, &format!("Mission {} ({}) beendet nach {} Ticks: {} -- zurueck {} Fahrzeuge, Erzwert {} von {}",
          i, artName(z.missArt[i]), t - z.missSeit[i], warum, n, erz, z.missStart[i]));
    lernen::buchen(welt, z, i, kw);
    z.missArt[i] = 0;
}

/// Alles an die Front: was im Pool steht, geht an die laufende Mission mit dem groessten Fehlbedarf (ausser Abwehr/
/// Ueberfall, die starten mit ihrer Kraft); ohne Mission bleibt es am Sammelpunkt vor der Basis.
fn restAnDieFront(welt: &mut dyn Welt, z: &mut Zustand, t: i32) {
    let pool = z.missPool;
    let (pkw, _, _, pn) = gruppenLage(welt, z, pool);
    if pn == 0 { return; }
    let mut best: Option<(i32, usize)> = None;
    for i in 0..8 {
        if !matches!(z.missArt[i], HALTEN_ERZ | HALTEN_PASS | VORPOSTEN | ENTSCHEIDUNG | ABWEHR) || z.missPhase[i] >= 2 { continue; }
        let (kw, _, _, n) = gruppenLage(welt, z, z.missGruppe[i]);
        if n >= 32 { continue; }
        let fehlt = z.missSoll[i] - kw;
        if best.map_or(true, |b| fehlt > b.0) { best = Some((fehlt, i)); }
    }
    let Some((fehlt, i)) = best else { return };
    // nur gebuendelt (mindestens 3 Fahrzeuge oder die ganze Luecke) -- einzeln quer ueber die Karte werden sie abgeschossen
    if fehlt <= 0 || (pn < 3 && pkw < fehlt) { return; }
    let g = z.missGruppe[i];
    let (mut n, mut kw) = (0, 0);
    for u in welt.GroupEnumerator(pool) {
        if !welt.IsLive(u) || !kaempfer(welt, u) { continue; }
        if welt.TotalUnitCount(g) >= 32 || kw >= fehlt { break; }
        kw += kampfWert(welt, u); z.missStart[i] += super::truppplan::fzWert(welt, u); z.missKwStart[i] += kampfWert(welt, u);
        welt.RemoveUnit(pool, u); welt.TakeUnit(g, u); n += 1;
    }
    if n > 0 {
        z.missBefehl[i] = 0;   // neue Fahrzeuge brauchen Befehle
        if selten(welt, z, 30, 2000) {
            let (x, y) = (z.missX[i], z.missY[i]);
            melde(welt, z, "kampf", mapLynx, x, y, &format!("an die Front: {} Fahrzeuge (Kampfkraft {}) verstaerken Mission {} ({})", n, pkw, i, artName(z.missArt[i])));
        }
    }
    let _ = t;
}
