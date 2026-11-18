//! Wissen und Parameter: Parametertext lesen (parameterLaden), Ergebnis/Punkte, Verluste zaehlen, Langzeitwissen laden/speichern.
use super::*;
use super::{WERTUNG_TICK, GEBAEUDE};

/// C++: void wissenPfad(char* out, int n, const char* datei)  (op2_ki.hpp:6762)
// Abweichung: der Ordner (op2::log::kiPath) kommt aus welt.dateiLesen/dateiSchreiben -- hier nur der Dateiname.
pub fn wissenPfad(_welt: &mut dyn Welt, out: &mut String, _n: i32, datei: &str) {
    *out = datei.to_string();
}

/// C++: void parameterLaden(Zustand& z)  (op2_ki.hpp:6772)
// Parameter aus logs/KI-Parameter-P<n>.txt (name=wert je Zeile, # = Kommentar). Unbekannte Namen werden gemeldet.
/// Schalter frueherer Fassungen, die die Missionen noch mitschicken: jetzt fest eingebaut (an), der Wert wird nicht
/// gelesen. lernen: Langzeit-Wissen gibt es nicht mehr (immer 0).
const FEST: [&str; 13] = ["lernen", "gefecht", "vorplan", "wehrlos", "turmNetz", "wissArbeit", "forschungVorrang", "fabrikWirtschaft", "seltenNutzen",
                          "fabrikErz", "erzAusbau", "gefechtStellung", "gefechtAbstand"];

pub fn parameterLaden(welt: &mut dyn Welt, z: &mut Zustand) {
    let sp = (z.spieler & 7) as usize;
    let mut q = Parameter::default();
    let speichern = |q: &Parameter| par.with(|p| p.borrow_mut()[sp] = *q);
    speichern(&q);
    let mut datei = format!("KI-Parameter-P{}.txt", z.spieler);
    let mut pfad = String::new();
    wissenPfad(welt, &mut pfad, 600, &datei);
    let f = welt.dateiLesen(&pfad);
    let text = parameterText.with(|t| t.borrow()[sp].clone());
    if f.is_none() && text.is_none() { return; }
    fn feld<'a>(q: &'a mut Parameter, name: &str) -> Option<&'a mut f64> {
        Some(match name {
            "reaktion" => &mut q.reaktion, "horizont" => &mut q.horizont, "ueberlegenheit" => &mut q.ueberlegenheit,
            "fabrikGewinn" => &mut q.fabrikGewinn, "reserveStart" => &mut q.reserveStart, "reserveLernrate" => &mut q.reserveLernrate,
            "reserveRueckgang" => &mut q.reserveRueckgang, "reserveMax" => &mut q.reserveMax, "turmAnteil" => &mut q.turmAnteil,
            "zermuerbAbstand" => &mut q.zermuerbAbstand, "schmelzeNutzen" => &mut q.schmelzeNutzen,
            "wohnraumGrenze" => &mut q.wohnraumGrenze, "freischaltWert" => &mut q.freischaltWert, "nahRadius" => &mut q.nahRadius,
            "heimHalten" => &mut q.heimHalten, "steigerung" => &mut q.steigerung, "vorsorge" => &mut q.vorsorge,
            "seltenGewinn" => &mut q.seltenGewinn, "schmelzenJeMine" => &mut q.schmelzenJeMine, "empSchwelle" => &mut q.empSchwelle,
            "risikoAnteil" => &mut q.risikoAnteil, "angriffMarge" => &mut q.angriffMarge, "rueckzugAb" => &mut q.rueckzugAb,
            "platzFern" => &mut q.platzFern, "roehreKachel" => &mut q.roehreKachel, "paarung" => &mut q.paarung,
            "turmOhneNetz" => &mut q.turmOhneNetz, "fabrikAuslastung" => &mut q.fabrikAuslastung, "moralWert" => &mut q.moralWert,
            "turmVorne" => &mut q.turmVorne, "turmAbstand" => &mut q.turmAbstand, "turmFront" => &mut q.turmFront,
            "turmUeberMauer" => &mut q.turmUeberMauer, "gefechtRadius" => &mut q.gefechtRadius,
            "gefechtSperre" => &mut q.gefechtSperre,
           
           
           
           
           
            "vorplanWachstum" => &mut q.vorplanWachstum, "vorplanRadius" => &mut q.vorplanRadius, "vorplanDicht" => &mut q.vorplanDicht,
            _ => return None,
        })
    }
    // naechste Zeile aus der Datei (fgets, 200) oder aus dem eingebauten Text
    // (Rust: erst der eingebaute Text, dann die Datei -- die Datei ueberschreibt einzelne Werte, statt den Satz zu ersetzen)
    let mut zeilen: Vec<String> = {
        let b = text.as_deref().unwrap_or("").as_bytes();
        let mut v = Vec::new();
        let mut pos = 0;
        while pos < b.len() {
            let a = pos;
            while pos < b.len() && b[pos] != b'\n' && pos - a < 200 - 2 { pos += 1; }
            let mut s = String::from_utf8_lossy(&b[a..pos]).into_owned();
            if pos < b.len() && b[pos] == b'\n' { pos += 1; }
            s.push('\n');
            v.push(s);
        }
        v
    };
    if let Some(ft) = &f { zeilen.extend(fgets_zeilen(ft, 200)); }
    let mut n = 0;
    for zeile in zeilen.iter() {
        if zeile.starts_with('#') || zeile.starts_with('\n') { continue; }
        // sscanf(zeile, "%63[^=]=%lf", name, &w) == 2
        let gl = match zeile.find('=') { Some(i) => i, None => continue };
        if gl == 0 || gl > 63 { continue; }
        let name = &zeile[..gl];
        let w = match scan_f64(&zeile[gl..], "=") { Some(w) => w, None => continue };
        let mut bekannt = false;
        if let Some(fe) = feld(&mut q, name) { *fe = w; bekannt = true; n += 1; }
        else if FEST.contains(&name) { bekannt = true; n += 1; }   // fest eingebaut, der Wert zaehlt nicht
        speichern(&q);
        if !bekannt { melde(welt, z, "info", mapNone, z.bx, z.by, &format!("Parameter unbekannt: {}", name)); }
    }
    if f.is_none() { datei = "eingebaut (Mission)".to_string(); } else if text.is_some() { datei = format!("eingebaut (Mission) + {}", datei); }
    // Grenzen, damit kein Satz die KI lahmlegt / sane bounds
    fn grenze(v: &mut f64, lo: f64, hi: f64) { if *v < lo { *v = lo; } if *v > hi { *v = hi; } }
    grenze(&mut q.reaktion, 1000.0, 30000.0); grenze(&mut q.horizont, 2000.0, 80000.0); grenze(&mut q.ueberlegenheit, 0.5, 4.0);
    grenze(&mut q.fabrikGewinn, 0.2, 1.0); grenze(&mut q.reserveStart, 0.5, 4.0); grenze(&mut q.reserveLernrate, 0.0, 2.0);
    grenze(&mut q.reserveRueckgang, 0.5, 1.0); grenze(&mut q.reserveMax, 1.0, 6.0); grenze(&mut q.turmAnteil, 0.0, 1.0);
    grenze(&mut q.zermuerbAbstand, 1000.0, 60000.0); grenze(&mut q.schmelzeNutzen, 0.1, 10.0); grenze(&mut q.wohnraumGrenze, 50.0, 99.0);
    grenze(&mut q.freischaltWert, 0.0, 2.0);
    grenze(&mut q.nahRadius, 8.0, 60.0); grenze(&mut q.heimHalten, 0.0, 4.0); grenze(&mut q.steigerung, 1.0, 2.0); grenze(&mut q.vorsorge, 0.0, 100.0);
    grenze(&mut q.seltenGewinn, 0.0, 5.0); grenze(&mut q.schmelzenJeMine, 0.5, 2.0); grenze(&mut q.empSchwelle, 0.1, 5.0);
    grenze(&mut q.gefechtSperre, 4.0, 400.0);
    grenze(&mut q.turmVorne, 1.0, 20.0); grenze(&mut q.turmAbstand, 1.0, 5.0); grenze(&mut q.turmFront, 1.0, 10.0); grenze(&mut q.turmUeberMauer, 0.5, 3.0);
    speichern(&q);
    melde(welt, z, "lernen", mapNone, z.bx, z.by, &format!("{} Parameter aus {} geladen (reaktion {:.0}, ueberlegenheit {:.2}, reserveStart {:.2}, lernen 0)",
        n, datei, q.reaktion, q.ueberlegenheit, q.reserveStart));
}

/// C++: void ergebnisSchreiben(Zustand& z)  (op2_ki.hpp:6825)
// Ergebnis eines Optimierer-Laufs: logs/KI-Ergebnis-P<n>.txt (eine Zeile, wird ueberschrieben).
pub fn ergebnisSchreiben(welt: &mut dyn Welt, z: &mut Zustand) {
    let datei = format!("KI-Ergebnis-P{}.txt", z.spieler);
    let mut pfad = String::new();
    wissenPfad(welt, &mut pfad, 600, &datei);
    let lp = laufPunkte(welt, z);
    let kl = if l_kolonieLebt(welt, z) { 1 } else { 0 };
    let text = format!("tick={} punkte={} zielArt={} ziel={} zerstoert={} verloren={} maxBew={} kolonie={} sieg={} niederlage={} verlustGeb={} verlustEinh={} schmelze2={}\n",
        welt.Tick(), lp, z.ziel, z.zielErreicht, z.zerstoertWert, z.verlorenWert, z.maxBew,
        kl, z.siegTick, z.niederlageTick, z.verlustGebaeude, z.verlustEinheiten, z.zweiteSchmelzeTick);
    welt.dateiSchreiben(&pfad, &text);
}

/// C++: int laufPunkte(const Zustand& z)  (op2_ki.hpp:6849)
// Wertung eines Laufs bis WERTUNG_TICK. Ziel 3: Sieg/Niederlage je frueher desto mehr, dazu gedeckelt +-1000
// Zerstoerung gegen Verluste. Ziel 2: Anteil am Bevoelkerungsziel. Ziel 1: Satellit. Immer: Verluste, Kolonie +200.
pub fn laufPunkte(welt: &mut dyn Welt, z: &Zustand) -> i32 {
    let mut p = 0;
    if z.ziel == 3 {
        let mut k = z.zerstoertWert / 10 - z.verlustEinheiten / 40 - z.verlustGebaeude / 5;
        if k > 1000 { k = 1000; } if k < -1000 { k = -1000; }
        let zeitWert = |t: i32| { let wt = WERTUNG_TICK; let rest = if wt - t > 0 { wt - t } else { 0 }; 2000 + (8000.0 * rest as f64 / WERTUNG_TICK as f64) as i32 };
        if z.siegTick != 0 { return zeitWert(z.siegTick) + k; }
        if z.niederlageTick != 0 { return -zeitWert(z.niederlageTick) + k; }
        return k;
    }
    if z.ziel == 2 {
        // Bevoelkerung: Anteil am Ziel, dazu die selbst angerichtete Zerstoerung und Verluste nur halb so schwer.
        let anteil = if z.zielWert > 0 { if z.maxBew * 1000 / z.zielWert > 1000 { 1000 } else { z.maxBew * 1000 / z.zielWert } } else { 0 };
        p += anteil + z.zerstoertWert / 10;
        if z.zielErreicht != 0 { p += (WERTUNG_TICK - z.zielErreicht) / 200; }
        p += z.verlorenWert / 40;                       // gleicht den allgemeinen Abzug (/20) auf /40 aus
    } else {
        if z.zielErreicht != 0 { p += 1000 - z.zielErreicht / 200; }
        p += z.maxBew;
    }
    p -= z.verlorenWert / 20;
    if l_kolonieLebt(welt, z) { p += 200; }
    p
}

/// C++: int einheitWert(UnitEx u)  (op2_ki.hpp:6879)
// Erzwert einer Einheit (mit Waffe).
pub fn einheitWert(welt: &mut dyn Welt, u: Id) -> i32 {
    let ty = welt.GetType(u);
    let mut w = welt.GetOreCost(ty, 0);
    let waffe = if welt.IsVehicle(u) || ty == mapGuardPost { welt.GetWeapon(u) } else { mapNone };
    if waffe > 0 && waffe != mapNone && waffe < 128 { w += welt.GetOreCost(waffe, 0); }
    if w > 0 { w } else { 100 }
}

/// C++: void verlusteZaehlen(int* ids, int* werte, int* pos, int& anz, int& summe, int& heim, bool feind, const Zustand& z, int* summeGeb = nullptr, int* summ  (op2_ki.hpp:6886)
// Zerstoerung zaehlen: was beim naechsten Scan fehlt, wurde zerstoert; der Erzwert wird summiert.
pub fn verlusteZaehlen(welt: &mut dyn Welt, ids: &mut [i32], werte: &mut [i32], pos: &mut [i32], anz: &mut i32, summe: &mut i32, heim: &mut i32, feind: bool, z: &Zustand, summeGeb: Option<&mut i32>, summeEinh: Option<&mut i32>) {
    let mut summeGeb = summeGeb;
    let mut summeEinh = summeEinh;
    let mut neuId = [0i32; 192];
    let mut neuWert = [0i32; 192];
    let mut neuPos = [0i32; 192];
    let mut n = 0usize;
    for p in 0..welt.NoPlayers() {
        if if feind { p == z.spieler || welt.IsAlliedTo(z.spieler, p) } else { p != z.spieler } { continue; }
        for u in welt.PlayerUnitEnum(p) {
            if n >= 192 { break; }
            if !welt.IsLive(u) { continue; }
            let t = welt.GetType(u);
            // Robo-Miner/GeoCon verwandeln sich in Minen -- kein Verlust
            if t == mapNone || t >= 128 || t == mapTube || t == mapWall || t == mapRoboMiner || t == mapGeoCon { continue; }
            let lo = welt.Location(u);
            let (ex, ey) = (edX(welt, z, lo), edY(welt, z, lo));
            neuPos[n] = (ex & 0xffff) | ey.wrapping_shl(16);
            neuId[n] = u | (t << 16); neuWert[n] = einheitWert(welt, u); n += 1;   // gleiche ID mit anderem Typ = neue Einheit
        }
        // Gebaeude ausdruecklich: in der Wertung 2026-11-07 blieb der Gebaeudeverlust trotz Niederlagen immer 0 (PlayerUnitEnum).
        for &gt in GEBAEUDE {
            for g in welt.PlayerBuildingEnum(p, gt) {
                if n >= 192 { break; }
                if !welt.IsLive(g) { continue; }
                let kennung = g | (gt << 16);
                let mut schon = false; for k in 0..n { if neuId[k] == kennung { schon = true; } }
                if schon { continue; }
                let lo = welt.Location(g);
                let (ex, ey) = (edX(welt, z, lo), edY(welt, z, lo));
                neuPos[n] = (ex & 0xffff) | ey.wrapping_shl(16);
                neuId[n] = kennung; neuWert[n] = einheitWert(welt, g); n += 1;
            }
        }
    }
    for i in 0..*anz as usize {
        let mut da = false;
        for k in 0..n { if neuId[k] == ids[i] { da = true; break; } }
        if !da {
            *summe += werte[i];
            if let Some(sg) = summeGeb.as_mut() {
                let typ = (ids[i] >> 16) & 0xffff;
                let mut geb = false;
                if typ != mapGuardPost { for &t in GEBAEUDE { if t == typ { geb = true; } } }
                if geb { **sg += werte[i]; } else if let Some(se) = summeEinh.as_mut() { **se += werte[i]; }
            }
            let x = (pos[i] & 0xffff) as i16 as i32;
            let y = pos[i] >> 16;
            if abstand2(welt, x, y, z.bx, z.by) <= 25 * 25 { *heim += werte[i]; }      // an der eigenen Basis gefallen
        }
    }
    ids[..n].copy_from_slice(&neuId[..n]); werte[..n].copy_from_slice(&neuWert[..n]); pos[..n].copy_from_slice(&neuPos[..n]); *anz = n as i32;
}
