//! Neuer Angriff: Truppe bemessen, Basisangriff, Zermuerben. Aufgeteilt aus
//! kampfFuehrungInnen (Rust 2026-11-18, Verhalten unveraendert).
use super::*;
use super::{GRUPPE_MAX, GEBAEUDE};
use super::fuehrung::{Bestand, Soll};

/// Kein Angriff laeuft: lohnt sich einer (ganze Feindbasis oder Zermuerben)?
#[allow(clippy::too_many_arguments)]
pub(super) fn neuerAngriff(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32, soll: &Soll, b: &mut Bestand, feindWTWert: i32, feindWTBasis: i32) {
    let Soll { fz, sammelZiel, .. } = *soll;
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
        // Zur Abwehr zaehlt nur die Feindarmee in ihrer Basis (15 Kacheln), nicht Wachen an fernen Aussenposten oder Paessen.
        let feindAbwehrArmee = z.feindArmeeHeim;
        if basisangriff(welt, z, l, t, fz, ueber, &mut sw, &mut ns, gesamt, bereit, feindWTBasis, feindAbwehrArmee, empAbzug) { return; }
        zermuerbenHand(welt, z, t, fz, ueber, &mut sw, &mut ns, gesamt, sammelZiel, eskalation, feindWTWert, feindAbwehrArmee, empAbzug);
    }
}

/// Wert der Angriffstruppe: Sammelgruppe (sw), Ueberschuss der Wache (ueber), was davon wirklich mitfaehrt (gesamt)
fn truppeBemessen(welt: &mut dyn Welt, z: &mut Zustand) -> (i32, i32, i32) {
    // Sammelgruppe voll und kein Feind zu uns unterwegs: alle mit einem Befehl in die Angriffsgruppe
    let sw;
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
    // Nur zaehlen, was wirklich mitfaehrt: Angriffsgruppe fasst 32.
    let mut gesamt;
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
/// (die verlangte Ueberlegenheit sinkt, wenn Warten das Verhaeltnis verschlechtert). true = losgeschlagen.
#[allow(clippy::too_many_arguments)]
fn basisangriff(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32, fz: i32, ueber: i32, sw: &mut i32, ns: &mut i32, gesamt: i32, bereit: bool,
                feindWTBasis: i32, feindAbwehrArmee: i32, empAbzug: i32) -> bool {
    let mut ueberleg = if PAR(z.spieler).ueberlegenheit < 1.0 { 1.0 } else { PAR(z.spieler).ueberlegenheit };
    // Warten kostet. Verhaeltnis jetzt und nach der Reaktionszeit (eigener Zuwachs: Erzzufluss und
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
    if spaeter < jetzt { ueberleg = (ueberleg * spaeter / jetzt).max(1.0); }
    if bereit && gesamt as f64 >= (feindWTBasis + feindAbwehrArmee - empAbzug) as f64 * ueberleg {
        let text = text160(format!("Angriff auf die Feindbasis bei {},{} gegen {} Wachtuerme und Armee {}", z.feindX, z.feindY, z.feindWT, z.feindArmee));
        losschlagen(welt, z, t, fz, ueber, sw, ns, 0, 0, &text);
        return true;
    }
    false
}

/// b) Zermuerben von Hand: schwaechstes Ziel, Angriff nur, wenn er sich laut Kampfmodell (Lanchester) lohnt
#[allow(clippy::too_many_arguments)]
fn zermuerbenHand(welt: &mut dyn Welt, z: &mut Zustand, t: i32, fz: i32, ueber: i32, sw: &mut i32, ns: &mut i32, gesamt: i32, sammelZiel: i32,
                  eskalation: i32, feindWTWert: i32, feindAbwehrArmee: i32, empAbzug: i32) {
    let (schwaechstes, schwachAbwehr) = schwaechstesZiel(welt, z, feindAbwehrArmee);
    // Zermuerben nur, wenn sich der Angriff laut Kampfmodell lohnt (Lanchester, gewertet wie die Punkte).
    let mut zermuerbGewinn = 1.0f64;
    if schwaechstes != 0 {
        let e = gesamt as f64;
        // die ganze Abwehr: der Weg fuehrt durch die Tuerme
        let dd = feindWTWert + feindAbwehrArmee - empAbzug;
        let d = if dd > 0 { dd as f64 } else { 0.0 };
        let zer = if e >= d { d } else { d * (1.0 - (1.0 - (e / if d > 0.0 { d } else { 1.0 }) * (e / if d > 0.0 { d } else { 1.0 })).sqrt()) };
        let ver = if e >= d { e * (1.0 - (1.0 - (d / if e > 0.0 { e } else { 1.0 }) * (d / if e > 0.0 { e } else { 1.0 })).sqrt()) } else { e };
        zermuerbGewinn = zer / 10.0 - ver / 20.0;
    }
    let gewinnMin = PAR(z.spieler).angriffMarge * gesamt as f64;   // knapper Gewinn: nur die Streuung
    if schwaechstes != 0 && gesamt > 0 && zermuerbGewinn > gewinnMin && (*ns >= sammelZiel || ueber > 0) && gesamt >= eskalation && t - z.letzterAngriffStart > z.zermuerbAbstand {
        let st = welt.GetType(schwaechstes); let nm = name(welt, st);
        let text = text160(format!("Zermuerben: Angriff auf {} (oertliche Abwehr {}, gelernter Abstand {} Ticks)", nm, schwachAbwehr, z.zermuerbAbstand));
        losschlagen(welt, z, t, fz, ueber, sw, ns, 2, schwaechstes, &text);
        return;
    }
}

/// Das Feindgebaeude mit der schwaechsten oertlichen Abwehr (Tuerme in 7 Kacheln plus die ganze Feindarmee, die in der
/// Basis steht und in Sekunden da ist). Liefert (Ziel, Abwehr)
fn schwaechstesZiel(welt: &mut dyn Welt, z: &mut Zustand, feindAbwehrArmee: i32) -> (Id, i32) {
    let mut schwaechstes: Id = 0; let mut schwachAbwehr = 1 << 30;
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
                abwehr += feindAbwehrArmee;
                if abwehr < schwachAbwehr { schwachAbwehr = abwehr; schwaechstes = b; }
            }
        }
        p += 1;
    }
    (schwaechstes, schwachAbwehr)
}
