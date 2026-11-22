//! Eine Mission fuehren: Phase 0 Anmarsch, 1 Wirken (angreifen bzw. halten), 2 Rueckzug. Abbruch unter missionRueck des
//! Startwerts oder bei Uebermacht vor Ort; Halten weicht aus und kommt spaeter wieder (neue Wahl).
use super::*;

fn d2(a: (i32, i32), b: (i32, i32)) -> i32 { (a.0 - b.0).pow(2) + (a.1 - b.1).pow(2) }

/// Fahren wie die C++-KI: die Gruppe bekommt ein Wach-Rechteck am Ziel und faehrt selbst hin (Einzelbefehle an Fahrzeuge
/// einer Kampfgruppe ueberschreibt die Gruppe). Neu gesetzt bei neuem Ziel oder alle 2000 Ticks.
fn fahren(welt: &mut dyn Welt, z: &mut Zustand, i: usize, t: i32, ziel: (i32, i32)) {
    let g = z.missGruppe[i];
    let neu = z.missBefehl[i] == 0 || t - z.missBefehl[i] >= 2000;
    if !neu { return; }
    let r = Rect::new(engine(welt, z, ziel.0 - 2, ziel.1 - 2), engine(welt, z, ziel.0 + 2, ziel.1 + 2));
    let gr = Rect::new(engine(welt, z, ziel.0 - 6, ziel.1 - 6), engine(welt, z, ziel.0 + 6, ziel.1 + 6));
    wachen(welt, g, r, &[gr], 1);
    z.missBefehl[i] = t;
}

/// Angreifen wie angriffZielSetzen: DoAttack faehrt nicht in Reichweite. Wer eine Bedrohung in Reichweite hat, schiesst
/// auf sie; wer das Ziel in Reichweite hat, greift es an; die anderen fahren bis knapp in Reichweite heran.
fn angreifen(welt: &mut dyn Welt, z: &mut Zustand, i: usize, ziel: Id) {
    let g = z.missGruppe[i];
    let mut bedr: Vec<Id> = Vec::new();
    for p in 0..welt.NoPlayers() {
        if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { continue; }
        for u in welt.PlayerVehicleEnum(p) { if welt.IsLive(u) && kampfWert(welt, u) > 0 { bedr.push(u); } }
        for u in welt.PlayerBuildingEnum(p, mapGuardPost) { if welt.IsLive(u) { bedr.push(u); } }
    }
    let zl = welt.Location(ziel);
    for v in welt.GroupEnumerator(g) {
        if !welt.IsLive(v) || !kaempfer(welt, v) { continue; }
        let akt = welt.GetCurAction(v);
        if akt != moDone && akt != moMove { continue; }
        let wi = welt.GetWeapon(v);
        let rw = if welt.GetWeaponRange(wi, z.spieler) > 0 { welt.GetWeaponRange(wi, z.spieler) } else { 4 };
        let vl = welt.Location(v);
        let mut best = (rw + 1) * (rw + 1) + 1; let mut bi: Option<Id> = None;
        for &b in bedr.iter() {
            let bl = welt.Location(b); let d = abstand2(welt, vl.x, vl.y, bl.x, bl.y);
            if d < best { best = d; bi = Some(b); }
        }
        if let Some(b) = bi { if akt == moMove || !welt.IsBusy(v) { welt.DoAttack(v, b); } continue; }
        let d2 = abstand2(welt, vl.x, vl.y, zl.x, zl.y);
        if d2 <= (rw + 1) * (rw + 1) { if akt == moMove || !welt.IsBusy(v) { welt.DoAttack(v, ziel); } continue; }
        if akt != moDone || welt.IsBusy(v) { continue; }
        let d = (d2 as f64).sqrt(); let f = (d - (rw - 1) as f64) / d;
        let hin = Loc::new(vl.x + (((zl.x - vl.x) as f64) * f + 0.5) as i32, vl.y + (((zl.y - vl.y) as f64) * f + 0.5) as i32);
        welt.DoMove(v, hin);
    }
}

/// Feind-Kampfkraft in r Kacheln um o
fn feindBei(welt: &mut dyn Welt, z: &Zustand, o: (i32, i32), r: i32) -> i32 {
    let mut s = 0;
    for p in 0..welt.NoPlayers() {
        if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { continue; }
        for u in welt.PlayerVehicleEnum(p) {
            if !welt.IsLive(u) { continue; }
            let w = kampfWert(welt, u); if w <= 0 { continue; }
            let lo = welt.Location(u); if d2((edX(welt, z, lo), edY(welt, z, lo)), o) <= r * r { s += w; }
        }
        for u in welt.PlayerBuildingEnum(p, mapGuardPost) {
            if !welt.IsLive(u) { continue; }
            let lo = welt.Location(u); if d2((edX(welt, z, lo), edY(welt, z, lo)), o) <= r * r { s += kampfWert(welt, u); }
        }
    }
    s
}

/// Naechstes lohnendes Wirtschaftsziel in 10 Kacheln (fuer den Ueberfall nach dem ersten Ziel)
fn naechstesZiel(welt: &mut dyn Welt, z: &Zustand, o: (i32, i32)) -> Option<(Id, (i32, i32))> {
    let mut best: Option<(i32, Id, (i32, i32))> = None;
    for p in 0..welt.NoPlayers() {
        if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { continue; }
        for u in welt.PlayerVehicleEnum(p) {
            if !welt.IsLive(u) || welt.GetType(u) != mapCargoTruck { continue; }
            let lo = welt.Location(u); let q = (edX(welt, z, lo), edY(welt, z, lo));
            let dd = d2(q, o); if dd <= 100 && best.map_or(true, |b| dd < b.0) { best = Some((dd, u, q)); }
        }
        for &ty in [mapCommonOreMine, mapRareOreMine, mapCommonOreSmelter, mapRareOreSmelter, mapTokamak, mapMHDGenerator, mapSolarPowerArray, mapGeothermalPlant, mapCommonStorage, mapAgridome].iter() {
            for u in welt.PlayerBuildingEnum(p, ty) {
                if !welt.IsLive(u) { continue; }
                let lo = welt.Location(u); let q = (edX(welt, z, lo), edY(welt, z, lo));
                let dd = d2(q, o); if dd <= 100 && best.map_or(true, |b| dd < b.0) { best = Some((dd, u, q)); }
            }
        }
    }
    best.map(|b| (b.1, b.2))
}

/// Ein Takt der Mission auf Platz i
pub fn missionSchritt(welt: &mut dyn Welt, z: &mut Zustand, i: usize, t: i32) {
    let art = z.missArt[i]; let g = z.missGruppe[i];
    let (kw, erz, pos, n) = gruppenLage(welt, z, g);
    let ort = (z.missX[i], z.missY[i]);
    if n == 0 { beenden(welt, z, i, t, "keine Fahrzeuge mehr", true); return; }
    // Rueckzug: Verluste oder Uebermacht vor Ort
    if z.missPhase[i] < 2 {
        let feind = feindBei(welt, z, pos, 10);
        let rueck = PAR(z.spieler).missionRueck;
        let grund = if (erz as f64) < rueck * z.missStart[i] as f64 { Some(format!("nur noch {} von {} Erzwert", erz, z.missStart[i])) }
                    else if feind as f64 > kw as f64 * if art == ENTSCHEIDUNG { 1.6 } else { 1.3 } { Some(format!("Uebermacht vor Ort: Feind {} gegen eigene {}", feind, kw)) }
                    else if art == UEBERFALL && z.missPhase[i] == 1 && t - z.missSeit[i] > 8000 { Some("Ueberfall dauert vor Ort zu lange".to_string()) }
                    else if art == ENTSCHEIDUNG && z.missPhase[i] == 1 && t - z.missSeit[i] > 20000 { Some("Entscheidungsschlag kommt vor Ort nicht voran".to_string()) }
                    else if z.missPhase[i] == 0 && t - z.missSeit[i] > 20000 { Some("Anmarsch dauert zu lange".to_string()) }
                    else if art == ABWEHR && t - z.missSeit[i] > 6000 { Some("Abwehr nach 6000 Ticks neu bewerten".to_string()) }
                    else { None };
        if let Some(gr) = grund {
            z.missPhase[i] = 2; z.missBefehl[i] = 0; z.missSeit[i] = t;   // ab jetzt zaehlt die Rueckzugsfrist
            melde(welt, z, "kampf", mapLynx, pos.0, pos.1, &format!("Mission {} ({}): Rueckzug -- {}", i, artName(art), gr));
        }
    }
    match z.missPhase[i] {
        0 => {
            // Anmarsch; Ueberfall/Entscheidung/Abwehr: bis 7 Kacheln ans Ziel, Halten: an den Ort
            let zl = z.missZiel[i];
            let ziel = if zl != 0 && welt.IsLive(zl) { let lo = welt.Location(zl); (edX(welt, z, lo), edY(welt, z, lo)) } else { ort };
            let nah = if matches!(art, UEBERFALL | ENTSCHEIDUNG | ABWEHR) { 49 } else { 16 };
            // Ziel unterwegs gefallen: gleich wirken (naechstes Ziel oder Rueckzug)
            let gefallen = matches!(art, UEBERFALL | ENTSCHEIDUNG) && (zl == 0 || !welt.IsLive(zl));
            if gefallen || d2(pos, ziel) <= nah {
                let anmarsch = t - z.missSeit[i];
                z.missPhase[i] = 1; z.missBefehl[i] = 0; z.missSeit[i] = t;   // Fristen vor Ort zaehlen ab der Ankunft
                if matches!(art, HALTEN_ERZ | HALTEN_PASS | VORPOSTEN) {
                    let r = Rect::new(engine(welt, z, ort.0 - 2, ort.1 - 2), engine(welt, z, ort.0 + 2, ort.1 + 2));
                    let gr = Rect::new(engine(welt, z, ort.0 - 7, ort.1 - 7), engine(welt, z, ort.0 + 7, ort.1 + 7));
                    wachen(welt, g, r, &[gr], 1);
                }
                melde(welt, z, "kampf", mapLynx, pos.0, pos.1, &format!("Mission {} ({}): angekommen nach {} Ticks mit {} Fahrzeugen", i, artName(art), anmarsch, n));
            } else { fahren(welt, z, i, t, ziel); }
        }
        1 => match art {
            UEBERFALL | ENTSCHEIDUNG => {
                let zl = z.missZiel[i];
                if zl != 0 && welt.IsLive(zl) && welt.OwnerID(zl) != z.spieler { angreifen(welt, z, i, zl); return; }
                if art == UEBERFALL {
                    if let Some((u, q)) = naechstesZiel(welt, z, pos) {
                        z.missZiel[i] = u; z.missX[i] = q.0; z.missY[i] = q.1;
                        let ty = welt.GetType(u); let nm = name(welt, ty);
                        melde(welt, z, "kampf", mapLynx, q.0, q.1, &format!("Mission {} (Ueberfall): Ziel zerstoert -- weiter auf {} bei {},{}", i, nm, q.0, q.1));
                        return;
                    }
                }
                z.missPhase[i] = 2; z.missBefehl[i] = 0;
                melde(welt, z, "kampf", mapLynx, pos.0, pos.1, &format!("Mission {} ({}): Ziel zerstoert -- Rueckzug mit {} Fahrzeugen", i, artName(art), n));
            }
            ABWEHR => {
                // naechsten Feind an der Kolonie angreifen; keiner mehr -> fertig
                let mut best: Option<(i32, Id)> = None;
                for p in 0..welt.NoPlayers() {
                    if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { continue; }
                    for u in welt.PlayerVehicleEnum(p) {
                        if !welt.IsLive(u) || kampfWert(welt, u) <= 0 { continue; }
                        let lo = welt.Location(u); let q = (edX(welt, z, lo), edY(welt, z, lo));
                        let dd = d2(q, ort); if dd <= 18 * 18 && best.map_or(true, |b| dd < b.0) { best = Some((dd, u)); }
                    }
                }
                match best { Some((_, u)) => angreifen(welt, z, i, u), None => beenden(welt, z, i, t, "Kolonie frei", false) }
            }
            _ => {}   // Halten: die Gruppe bewacht den Ort (DoGuardRect); das Gefecht fuehrt gefecht::fuehren
        },
        _ => {
            let heim = poolPunkt(welt, z);
            let erfolglos = (erz as f64) < 0.8 * z.missStart[i] as f64;
            if d2(pos, heim) <= 64 { beenden(welt, z, i, t, "zurueck am Sammelpunkt", erfolglos); }
            else if z.missPhase[i] == 2 && t - z.missSeit[i] > 3000 { beenden(welt, z, i, t, "Rueckzugsfrist abgelaufen (Rest faehrt zum Sammelpunkt)", erfolglos); }
            else { fahren(welt, z, i, t, heim); }
        }
    }
}
