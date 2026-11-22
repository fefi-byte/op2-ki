//! Neuer Angriff: Truppe bemessen, Basisangriff, Zermuerben (Hand, Schwachstelle oder Truppnetz). Aufgeteilt aus
//! kampfFuehrungInnen (Rust 2026-11-18, Verhalten unveraendert).
use super::*;
use super::super::*;
use super::{GRUPPE_MAX, SCAN_TAKT, WERT_WT, WERT_LYNX, EMP_RADIUS, EMP_FLUG, EMP_AUFMARSCH, GEBAEUDE};
use super::fuehrung::{Bestand, Soll};

/// Kein Angriff laeuft: lohnt sich einer (ganze Feindbasis oder Zermuerben)?
#[allow(clippy::too_many_arguments)]
pub(super) fn neuerAngriff(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32, soll: &Soll, b: &mut Bestand, feindWTWert: i32, feindWTBasis: i32) {
    let Soll { fz, sammelZiel, .. } = *soll;
    // Missionsmodus: ein Angriffsauftrag der Mission (Trigger) geht vor -- auch bei eigenen Angriffen aus und in der Schonfrist
    if let Some(a) = super::super::modus::auftrag(welt, z) { auftragAngriff(welt, z, l, t, soll, b, feindWTBasis, a); return; }
    // Missionsmodus, Rolle defensiv: Gegenstoss nach einer abgewehrten Welle -- einmal je Welle, wenn die eigene Truppe mindestens
    // so stark ist wie die feindliche Armee, die uebrig ist
    if super::super::modus::gegenstossFaellig(z, t) {
        // die Kraft der defensiven Rolle steckt in der Wache: drei Viertel davon ziehen mit, ein Viertel bleibt daheim
        let (mut sw, _, _) = truppeBemessen(welt, z);
        let mut nw = 0; let wv = gruppenWert(welt, z.wache, &mut nw);
        let ueber = wv * 3 / 4; let gesamt = sw + ueber;
        let mut ns = b.ns;
        if gesamt > 0 && gesamt >= z.feindArmee && ns + nw * 3 / 4 >= 2 {
            let text = text160(format!("Gegenstoss nach abgewehrtem Angriff: Truppe {} gegen Feindarmee {}", gesamt, z.feindArmee));
            losschlagen(welt, z, l, t, soll.fz, ueber, &mut sw, &mut ns, 0, 0, &text);
            super::super::modus::gegenstossGenutzt(z);
            return;
        }
    }
    // Missionsmodus (Nutzer 2026-11-22): eigene Angriffe an/aus (Befehl der Mission, Rolle) und Schonfrist der Stufe
    if !super::super::modus::eigeneAngriffe(z, t) { return; }
    // PAR laserKeinAngriff (Nutzer 2026-11-20): Eden greift nicht an, solange es nur Laser hat (vor der Railgun) -- die Arena
    // braucht dafuer rund doppelt so viel Erz wie der Verteidiger; das Erz geht in Tuerme und Wirtschaft
    if PAR(z.spieler).laserKeinAngriff >= 0.5 && welt.IsEden(z.spieler) && !hatTechId(welt, z, 5506) { return; }
    let mut ns = b.ns;
    let (mut sw, ueber, gesamt) = truppeBemessen(welt, z);
    // "bereit": Sammelgruppe voll, oder die Wache hat Ueberschuss -- und die Truppe ist staerker als beim letzten Angriff.
    let mut eskalation = 0;
    if z.angriffStart > 0 {
        let warten = t - z.letzterAngriffStart;
        eskalation = if warten > 2 * z.zermuerbAbstand { 0 } else if warten > z.zermuerbAbstand { z.angriffStart } else { (z.angriffStart as f64 * PAR(z.spieler).steigerung) as i32 };
    }
    let bereit = (ns >= sammelZiel || ueber > 0) && gesamt >= eskalation;
    // Steht eine EMP-Rakete bereit, zaehlt die Abwehr, die sie laehmt, beim Kraftvergleich nicht mit
    let mut empAbzug = 0;
    if z.empRakete != 0 { let (mut ex, mut ey) = (0, 0); empAbzug = empZiel(welt, z, &mut ex, &mut ey); }
    if z.feindUnterwegs == 0 && z.feindGeb != 0 && gesamt > 0 && (ns + if ueber > 0 { 2 } else { 0 }) >= 2 {
        // a) ganze Feindbasis, wenn die Truppe Tuerme plus Armee mit Reserve schlaegt
        // Rust (kampfModell): zur Abwehr zaehlt nur die Feindarmee in ihrer Basis (15 Kacheln), nicht Wachen an fernen
        // Aussenposten oder Paessen.
        let feindAbwehrArmee = if !kampfModellNeu() { z.feindArmee }
                               else if PAR(z.spieler).abwehrRueckkehr >= 0.5 { z.feindArmeeZurueck } else { z.feindArmeeHeim };
        if basisangriff(welt, z, l, t, fz, ueber, &mut sw, &mut ns, gesamt, bereit, feindWTBasis, feindAbwehrArmee, empAbzug) { return; }
        // b') Rust (PAR truppPlan, Truppnetz): Ziel und Plan nach gelerntem Schaden - Risiko x Verlust; das Netz entscheidet
        // auch, ob sich gar kein Zermuerben lohnt
        if kampfModellNeu() && PAR(z.spieler).truppPlan >= 0.5 && super::truppplan::geladen(welt) {
            truppPlanZermuerben(welt, z, l, t, fz, ueber, &mut sw, &mut ns, gesamt, sammelZiel, eskalation);
            return;
        }
        zermuerbenHand(welt, z, l, t, fz, ueber, &mut sw, &mut ns, gesamt, sammelZiel, eskalation, feindWTWert, feindAbwehrArmee, empAbzug);
    }
}

/// Angriffsauftrag der Mission: noetige Kraft = Staerke x verlangte Ueberlegenheit x Abwehr am Ziel (mindestens drei Lynx);
/// die Sollzahlen bauen die Truppe (modus::noetig), losgeschlagen wird, sobald sie steht -- spaetestens nach 12 000 Ticks.
#[allow(clippy::too_many_arguments)]
fn auftragAngriff(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32, soll: &Soll, b: &mut Bestand, feindWTBasis: i32, a: super::super::modus::Auftrag) {
    // Ohne bekanntes Feindgebaeude gibt es kein Ziel -- ausser beim Befehl "halten" (Original cm03: der Befehl verschwand still)
    if z.feindGeb == 0 && !super::super::befehl::istHalten(z) {
        super::super::befehl::keinZiel(welt, z);
        super::super::modus::auftragErledigt(z); return;
    }
    let (mut sw, ueber, gesamt) = truppeBemessen(welt, z);
    let feindAbwehrArmee = if PAR(z.spieler).abwehrRueckkehr >= 0.5 { z.feindArmeeZurueck } else { z.feindArmeeHeim };
    // Ziel: Feindbasis oder das Feindgebaeude naechst dem Ort
    let mut wunsch: Id = 0; let mut abwehr = feindWTBasis + feindAbwehrArmee;
    if let Some((ox, oy)) = a.ort {
        let mut best = i32::MAX;
        for p in 0..welt.NoPlayers() {
            if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { continue; }
            for &ty in GEBAEUDE.iter() {
                for u in welt.PlayerBuildingEnum(p, ty) {
                    if !welt.IsLive(u) { continue; }
                    let lo = welt.Location(u); let (ux, uy) = (edX(welt, z, lo), edY(welt, z, lo)); let d = abstand2(welt, ux, uy, ox, oy);
                    if d < best { best = d; wunsch = u; }
                }
            }
        }
        abwehr = feindStaerkeBei(welt, z, ox, oy, 12);
    }
    let ueberleg = PAR(z.spieler).ueberlegenheit.max(PAR(z.spieler).ueberlegMin);
    let noetig = ((a.staerke * ueberleg * abwehr as f64) as i32).max(3 * WERT_LYNX);
    super::super::modus::setzeNoetig(z, noetig);
    let mut ns = b.ns;
    let faellig = t - a.seit > 12000;
    // Befehl der Mission (befehl.rs): Truppe nach Liste/Anzahl/"vorhanden", Frist; nur die Sammelgruppe oder alle
    let bereitBefehl = super::super::befehl::bereit(welt, z, t, a.seit);
    let ueber = if super::super::befehl::nurSammel(z) { 0 } else if super::super::befehl::alleMit(z) { let mut d = 0; gruppenWert(welt, z.wache, &mut d) } else { ueber };
    let los = match bereitBefehl { Some(b) => b && ns + if ueber > 0 { 1 } else { 0 } >= 1, None => (gesamt >= noetig || faellig) && ns + if ueber > 0 { 2 } else { 0 } >= 2 };
    // Befehl "halten": die Truppe faehrt zum Punkt und bewacht ihn (kein Ziel im Feindlager)
    if los && super::super::befehl::istHalten(z) && a.ort.is_some() {
        let (ax, aa2, aw, sm) = (z.angriff, z.angriff2, z.wache, z.sammel);
        if sicherAlle(welt, z, ax, sm) > 0 { sicherAlle(welt, z, aa2, sm); }
        if super::super::befehl::alleMit(z) { if sicherAlle(welt, z, ax, aw) > 0 { sicherAlle(welt, z, aa2, aw); } }
        let (hx, hy) = a.ort.unwrap_or((z.bx, z.by));
        let mut na = 0; let wert = angriffWert(welt, z, &mut na);
        z.angriffStart = wert; z.kampfModus = 2; z.kampfModusSeit = t; z.letzterAngriffStart = t; z.angriffWunsch = 0; z.angriffArt = 2;
        z.angriffZerstoertStart = z.zerstoertWert; z.angriffVerlorenStart = z.verlorenWert;
        let warte = Rect::new(engine(welt, z, hx - 2, hy - 2), engine(welt, z, hx + 2, hy + 2));
        let schutz = Rect::new(engine(welt, z, hx - 6, hy - 6), engine(welt, z, hx + 6, hy + 6));
        for g in [z.angriff, z.angriff2] { if g != 0 { wachen(welt, g, warte, &[schutz], 1); } }
        super::super::befehl::losgeschlagen(z, a.ort);
        super::super::modus::auftragErledigt(z);
        melde(welt, z, "kampf", mapLynx, hx, hy, &format!("Befehl: {} Fahrzeuge (Wert {}) halten den Bereich um {},{}", na, wert, hx, hy));
        return;
    }
    if los {
        let text = text160(format!("Mission: befohlener Angriff {} mit Kraft {} (noetig {}{})",
            if wunsch != 0 { "auf das Ziel am Ort" } else { "auf die Feindbasis" }, gesamt, noetig, if faellig && gesamt < noetig { ", Frist abgelaufen" } else { "" }));
        losschlagen(welt, z, l, t, soll.fz, ueber, &mut sw, &mut ns, if wunsch != 0 { 2 } else { 0 }, wunsch, &text);
        super::super::befehl::losgeschlagen(z, a.ort);
        super::super::modus::auftragErledigt(z);
    }
}

/// Wert der Angriffstruppe: Sammelgruppe (sw), Ueberschuss der Wache (ueber), was davon wirklich mitfaehrt (gesamt)
fn truppeBemessen(welt: &mut dyn Welt, z: &mut Zustand) -> (i32, i32, i32) {
    // Sammelgruppe voll und kein Feind zu uns unterwegs: alle mit einem Befehl in die Angriffsgruppe
    let mut sw;
    { let mut dummy = 0; sw = gruppenWert(welt, z.sammel, &mut dummy); }
    // Umverteilung Wache -> Angriff aus der Lage: daheim bleibt nur, was der Feind neu aufstellen koennte, minus Tuerme.
    let mut ueber;
    {
        let mut dummy = 0;
        let ww = gruppenWert(welt, z.wache, &mut dummy);
        let mut halten = ((z.feindPotenzial - z.feindArmee) as f64 * PAR(z.spieler).heimHalten) as i32 - z.eigeneWTWert;
        if halten < 0 { halten = 0; }
        ueber = ww - halten;
        // riskanter, mehr aus der Verteidigung greift mit an
        if (ueber as f64) < ww as f64 * PAR(z.spieler).risikoAnteil { ueber = (ww as f64 * PAR(z.spieler).risikoAnteil) as i32; }
        if ueber < 0 || z.feindUnterwegs > 0 { ueber = 0; }
    }
    let mut gesamt = sw + ueber;
    // Nur zaehlen, was wirklich mitfaehrt: Angriffsgruppe fasst 32.
    {
        let mut nA = 0; let mut nS = 0;
        let wA = angriffWert(welt, z, &mut nA); let wS = gruppenWert(welt, z.sammel, &mut nS);
        let plaetze = 2 * GRUPPE_MAX;
        let mut platz = plaetze - nA - nS;
        if platz <= 0 { gesamt = if nA + nS > 0 { (wA + wS) * plaetze / (nA + nS) } else { 0 }; }
        else {
            gesamt = wA + wS;
            let mut bew = 0;
            let mut seen = [0i32; 64]; let mut ns2 = 0usize;
            for v in welt.GroupEnumerator(z.wache) {
                if !(platz > 0 && bew < ueber) { break; }
                if !welt.IsLive(v) || !kaempfer(welt, v) { continue; }
                let mut schon = false; for i in 0..ns2 { if seen[i] == v { schon = true; } }
                if schon { continue; }
                if ns2 < 64 { seen[ns2] = v; ns2 += 1; }
                let kw = kampfWert(welt, v);
                if bew > 0 && bew + kw > ueber + kw / 2 { continue; }
                bew += kw; platz -= 1;
            }
            gesamt += bew;
        }
    }
    (sw, ueber, gesamt)
}

/// a) Angriff auf die ganze Feindbasis, wenn die Truppe Tuerme plus Armee mit der verlangten Ueberlegenheit schlaegt
/// (Rust kampfModell: die verlangte Ueberlegenheit sinkt, wenn Warten das Verhaeltnis verschlechtert). true = losgeschlagen.
#[allow(clippy::too_many_arguments)]
fn basisangriff(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32, fz: i32, ueber: i32, sw: &mut i32, ns: &mut i32, gesamt: i32, bereit: bool,
                feindWTBasis: i32, feindAbwehrArmee: i32, empAbzug: i32) -> bool {
    // PAR ueberlegMin (Missionsmodus, Rolle aggressiv): Untergrenze der verlangten Ueberlegenheit (Standard 1)
    let umin = PAR(z.spieler).ueberlegMin;
    let mut ueberleg = if PAR(z.spieler).ueberlegenheit < umin { umin } else { PAR(z.spieler).ueberlegenheit };
    if kampfModellNeu() {
        // Rust (kampfModell): Warten kostet. Verhaeltnis jetzt und nach der Reaktionszeit (eigener Zuwachs: Erzzufluss und
        // Fabrikleistung, der kleinere; Feind: gemessenes Wachstum). Wird das Verhaeltnis schlechter, faellt die verlangte
        // Ueberlegenheit stetig gegen 1: max(1, Ueberlegenheit x spaeter/jetzt).
        let d = (feindWTBasis + feindAbwehrArmee - empAbzug).max(1) as f64;
        let zw = if z.zielWaffe > 0 { z.zielWaffe } else { mapLaser };
        let wert = sdkWertT(welt, fz, zw).max(1) as f64;
        let s = z.spieler;
        let kosten = (welt.GetOreCost(fz, s) + welt.GetOreCost(zw, s)).max(1) as f64;
        let erzRate = z.zuflussErz.max(0.0) / kosten;                                   // Fahrzeuge je 1000 Ticks
        let jeFz = (welt.GetBuildTime(fz, s) as f64 * welt.GetProductionCapacity(mapVehicleFactory, s) as f64 / 128.0).max(1.0);
        let fabRate = l.vfAnzahl as f64 * 1000.0 / jeFz;
        let eigen = erzRate.min(fabRate) * wert;
        let h = PAR(s).reaktion / 1000.0;
        let jetzt = gesamt as f64 / d;
        let spaeter = (gesamt as f64 + eigen * h) / (d + z.feindWachstum.max(0.0) * h);
        if spaeter < jetzt { ueberleg = (ueberleg * spaeter / jetzt).max(umin); }
    }
    if bereit && gesamt as f64 >= (feindWTBasis + feindAbwehrArmee - empAbzug) as f64 * ueberleg {
        let text = text160(format!("Angriff auf die Feindbasis bei {},{} gegen {} Wachtuerme und Armee {}", z.feindX, z.feindY, z.feindWT, z.feindArmee));
        losschlagen(welt, z, l, t, fz, ueber, sw, ns, 0, 0, &text);
        return true;
    }
    false
}

/// b) Zermuerben von Hand: schwaechstes Ziel, Angriff nur, wenn er sich laut Kampfmodell (Lanchester) lohnt
#[allow(clippy::too_many_arguments)]
fn zermuerbenHand(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32, fz: i32, ueber: i32, sw: &mut i32, ns: &mut i32, gesamt: i32, sammelZiel: i32,
                  eskalation: i32, feindWTWert: i32, feindAbwehrArmee: i32, empAbzug: i32) {
    let (schwaechstes, schwachAbwehr, schwachWert, schwach) = schwaechstesZiel(welt, z, feindAbwehrArmee);
    // Zermuerben nur, wenn sich der Angriff laut Kampfmodell lohnt (Lanchester, gewertet wie die Punkte).
    let mut zermuerbGewinn = 1.0f64;
    if schwaechstes != 0 {
        let e = gesamt as f64;
        // alt: die ganze Abwehr (der Weg fuehrt durch die Tuerme); schwachstelle: die oertliche am Ziel
        let dd = if schwach { schwachAbwehr - empAbzug } else { feindWTWert + feindAbwehrArmee - empAbzug };
        let d = if dd > 0 { dd as f64 } else { 0.0 };
        let zer = if e >= d { d } else { d * (1.0 - (1.0 - (e / if d > 0.0 { d } else { 1.0 }) * (e / if d > 0.0 { d } else { 1.0 })).sqrt()) };
        let ver = if e >= d { e * (1.0 - (1.0 - (d / if e > 0.0 { e } else { 1.0 }) * (d / if e > 0.0 { e } else { 1.0 })).sqrt()) } else { e };
        zermuerbGewinn = zer / 10.0 - ver / 20.0;
        // schwachstelle: faellt das Ziel, zaehlt sein Wert (Erzpunkte wie Zerstoerung /10, in Kampfwert-Einheiten /150)
        if schwach && e > d { zermuerbGewinn += schwachWert as f64 / 150.0; }
    }
    let gewinnMin = PAR(z.spieler).angriffMarge * gesamt as f64;   // knapper Gewinn: nur die Streuung
    if schwaechstes != 0 && gesamt > 0 && zermuerbGewinn > gewinnMin && (*ns >= sammelZiel || ueber > 0) && gesamt >= eskalation && t - z.letzterAngriffStart > z.zermuerbAbstand {
        let st = welt.GetType(schwaechstes); let nm = name(welt, st);
        let text = text160(format!("Zermuerben: Angriff auf {} (oertliche Abwehr {}, Wert {}, gelernter Abstand {} Ticks)", nm, schwachAbwehr, schwachWert, z.zermuerbAbstand));
        losschlagen(welt, z, l, t, fz, ueber, sw, ns, 2, schwaechstes, &text);
        return;
    }
}

/// Das Feindgebaeude mit der schwaechsten oertlichen Abwehr (Tuerme in 7 Kacheln plus Armee); mit PAR schwachstelle
/// nach Wert/Abwehr/Weg. Liefert (Ziel, Abwehr, Wert, schwachstelle an)
fn schwaechstesZiel(welt: &mut dyn Welt, z: &mut Zustand, feindAbwehrArmee: i32) -> (Id, i32, i32, bool) {
    // b) Zermuerben: das Feindgebaeude mit der schwaechsten oertlichen Abwehr (Tuerme in 7 Kacheln plus die ganze Feindarmee).
    let mut schwaechstes: Id = 0; let mut schwachAbwehr = 1 << 30;
    // Rust (PAR schwachstelle, Nutzer 2026-11-17: er gewann ueber die schwach bewachte Mine im Sueden): Wert/Abwehr je
    // Ziel; die Feindarmee zaehlt nur, soweit sie das Ziel rechtzeitig erreicht.
    let schwach = kampfModellNeu() && PAR(z.spieler).schwachstelle >= 0.5;
    let mut schwachWert = 0;
    let mut bestWertung = -1.0f64;
    let mut feindFz: Vec<(i32, i32, i32)> = Vec::new();
    if schwach {
        let mut q = 0;
        while q < welt.NoPlayers() {
            if q == z.spieler || welt.IsAlliedTo(z.spieler, q) { q += 1; continue; }
            for u in welt.PlayerVehicleEnum(q) {
                if !welt.IsLive(u) { continue; }
                let w = kampfWert(welt, u); if w <= 0 { continue; }
                let lo = welt.Location(u); feindFz.push((edX(welt, z, lo), edY(welt, z, lo), w));
            }
            q += 1;
        }
    }
    let mut p = 0;
    while p < welt.NoPlayers() {
        if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { p += 1; continue; }
        for &ty in GEBAEUDE.iter() {
            if ty == mapGuardPost { continue; }
            for b in welt.PlayerBuildingEnum(p, ty) {
                if !welt.IsLive(b) { continue; }
                let lo = welt.Location(b);
                let bx = edX(welt, z, lo); let by = edY(welt, z, lo);
                let mut abwehr = 0;
                for g in welt.PlayerBuildingEnum(p, mapGuardPost) {
                    if welt.IsLive(g) { let gl = welt.Location(g); let (gx, gy) = (edX(welt, z, gl), edY(welt, z, gl)); if abstand2(welt, gx, gy, bx, by) <= 7 * 7 { abwehr += kampfWert(welt, g); } }
                }
                if schwach {
                    // Armee nach Entfernung: bis 10 Kacheln voll, ab 40 Kacheln nichts (kommt zu spaet)
                    for &(fx, fy, w) in feindFz.iter() {
                        let d = (abstand2(welt, fx, fy, bx, by) as f64).sqrt();
                        abwehr += (w as f64 * (1.0 - (d - 10.0) / 30.0).clamp(0.0, 1.0)) as i32;
                    }
                    let wert = schwachZielWert(welt, ty, p);
                    let weg = (abstand2(welt, bx, by, z.bx, z.by) as f64).sqrt();
                    let wertung = wert as f64 / (1.0 + abwehr as f64) / (1.0 + weg / 60.0);
                    if wertung > bestWertung { bestWertung = wertung; schwachAbwehr = abwehr; schwaechstes = b; schwachWert = wert; }
                    continue;
                }
                // Die ganze Feindarmee zaehlt: sie steht in der Basis und ist in Sekunden da.
                abwehr += feindAbwehrArmee;
                if abwehr < schwachAbwehr { schwachAbwehr = abwehr; schwaechstes = b; }
            }
        }
        p += 1;
    }
    (schwaechstes, schwachAbwehr, schwachWert, schwach)
}

/// Rust (PAR truppPlan): Zermuerben mit dem Truppnetz. Trupp = Sammelgruppe (+ Wach-Ueberschuss anteilig), Gegner = alle
/// feindlichen Kampffahrzeuge und Tuerme; Ziele = Feindgebaeude der gelernten Klassen und der naechste Lastwagen.
#[allow(clippy::too_many_arguments)]
pub(super) fn truppPlanZermuerben(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32, fz: i32, ueber: i32, sw: &mut i32, ns: &mut i32,
                       gesamt: i32, sammelZiel: i32, eskalation: i32) {
    use super::truppplan as tp;
    if gesamt <= 0 || !((*ns >= sammelZiel || ueber > 0) && gesamt >= eskalation && t - z.letzterAngriffStart > z.zermuerbAbstand) { return; }
    let sp = z.spieler;
    let mut us: Vec<Id> = Vec::new();
    for v in welt.GroupEnumerator(z.sammel) { if welt.IsLive(v) && welt.IsVehicle(v) && kampfWert(welt, v) > 0 { us.push(v); } }
    let mut fzs = Vec::new();
    for &v in us.iter() { let lo = welt.Location(v); let w = tp::fzWert(welt, v); fzs.push((edX(welt, z, lo), edY(welt, z, lo), w)); }
    // Wach-Ueberschuss: Kampfwert -> Erzwert im Verhaeltnis der Sammelgruppe
    if *sw > 0 && ueber > 0 { let f = (gesamt as f64 / *sw as f64) as f32; for e in fzs.iter_mut() { e.2 = (e.2 as f32 * f) as i32; } }
    let anteil = tp::gebAnteil(welt, &us);
    let mut ang = tp::seite(fzs, anteil, (z.bx, z.by));
    if ang.wert <= 0 { ang.wert = gesamt * 10; }
    let mut feind = Vec::new(); let mut tuerme = Vec::new();
    let mut ziele: Vec<(Id, (i32, i32), usize, i32)> = Vec::new();
    let mut lkw: Option<(Id, (i32, i32), i32)> = None;
    let mut p = 0;
    while p < welt.NoPlayers() {
        if p == sp || welt.IsAlliedTo(sp, p) { p += 1; continue; }
        feind.extend(tp::kampfFz(welt, z, p)); tuerme.extend(tp::tuermeVon(welt, z, p));
        for &ty in GEBAEUDE.iter() {
            let Some(k) = tp::zielKlasse(ty) else { continue };
            for b in welt.PlayerBuildingEnum(p, ty) {
                if !welt.IsLive(b) { continue; }
                let lo = welt.Location(b); let w = tp::zielWert(welt, ty, p);
                ziele.push((b, (edX(welt, z, lo), edY(welt, z, lo)), k, w));
            }
        }
        for u in welt.PlayerVehicleEnum(p) {
            if !welt.IsLive(u) || welt.GetType(u) != mapCargoTruck { continue; }
            let lo = welt.Location(u); let q = (edX(welt, z, lo), edY(welt, z, lo));
            let d = (q.0 - ang.mx).pow(2) + (q.1 - ang.my).pow(2);
            if lkw.map_or(true, |e| d < e.2) { lkw = Some((u, q, d)); }
        }
        p += 1;
    }
    if let Some((u, q, _)) = lkw { let o = welt.OwnerID(u); let w = tp::zielWert(welt, mapCargoTruck, o); ziele.push((u, q, 6, w)); }
    let ver = tp::seite(feind, 0.0, (z.feindX, z.feindY));
    let basis = (z.feindX, z.feindY);
    let risiko = PAR(sp).truppRisiko as f32;
    // Vorauswahl nach Luftlinie (Routen kosten Rechenzeit), dann die besten 10 Ziele mit ihren Routen
    let mut vor: Vec<(f32, usize)> = ziele.iter().enumerate()
        .filter_map(|(i, &(_, q, k, w))| tp::besterPlan(&ang, &ver, &tuerme, (q, k, w), &[], basis, risiko).map(|b| (b.ertrag, i))).collect();
    vor.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut best: Option<(tp::Wahl, Id, Option<super::routen::Route>)> = None;
    for &(_, i) in vor.iter().take(10) {
        let (u, q, k, w) = ziele[i];
        let (rs, info) = tp::routenMit(welt, z, &ang, &ver, &tuerme, q);
        if let Some(wa) = tp::besterPlan(&ang, &ver, &tuerme, (q, k, w), &info, basis, risiko) {
            if best.as_ref().map_or(true, |b| wa.ertrag > b.0.ertrag) { best = Some((wa, u, wa.route.map(|r| rs[r].clone()))); }
        }
    }
    let Some((wa, ziel, route)) = best else { return };
    let (s, v, r, wt) = (wa.schaden, wa.verlust, wa.rueck, wa.weiter);
    let kein = tp::vorhersage(&tp::merkmale(&ang, &ver, &tuerme, None, None, basis, 0.0, false)).map_or(0.0, |y| y[0] - risiko * y[1]);
    if wa.ertrag <= (PAR(sp).truppMin as f32).max(kein) { return; }
    let zt = welt.GetType(ziel); let nm = name(welt, zt);
    let text = text160(format!("Truppplan: Zermuerben auf {} {} (Schaden {:.2}, Verlust {:.2} je Startwert; Rueckzug {}, danach {})",
        nm, match wa.route { Some(r) => format!("ueber Route {} von {}", r + 1, vor.len().min(3)), None => "Luftlinie".into() }, s, v, if r > 0.0 { format!("unter {:.0} %", r * 100.0) } else { "nie".into() }, if wt { "weiter" } else { "heim" }));
    z.planAn = 1; z.planZiel = ziel; z.planRueck = (r * 1000.0) as i32; z.planWeiter = wt as i32; z.planStart = 0;
    z.planAufX = 0; z.planAufY = 0; z.planWpN = 0; z.planWpI = 0;
    if let Some(ro) = route.as_ref() {
        // Aufmarsch auf der Route: 25..40 Wegkacheln vor dem Ziel, ausser Reichweite der Tuerme (9 Kacheln); die Wegpunkte danach
        let n = ro.weg.len();
        let mut auf: Option<usize> = None;
        for a in 25..=40usize {
            if a + 2 >= n { break; }
            let (x, y) = ro.weg[n - 1 - a];
            if tuerme.iter().all(|t| (t.0 - x).pow(2) + (t.1 - y).pow(2) > 81) { auf = Some(n - 1 - a); break; }
        }
        let ab = auf.unwrap_or(0);
        if let Some(i) = auf { z.planAufX = ro.weg[i].0; z.planAufY = ro.weg[i].1; }
        let mut k = 0usize;
        let mut i = ab + 8;
        while i + 8 < n && k < 16 { z.planWpX[k] = ro.weg[i].0; z.planWpY[k] = ro.weg[i].1; k += 1; i += 8; }
        z.planWpN = k as i32;
    }
    losschlagen(welt, z, l, t, fz, ueber, sw, ns, 2, ziel, &text);
}
