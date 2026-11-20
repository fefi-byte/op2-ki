//! Kampfwerte und Waffen: kampfWert, Waffen-/Fahrgestellwahl, EMP-Rakete, Kraft je Erz, Seltenerz-Vorteil.
use super::*;

// Was als Naechstes gebaut wird: Notfaelle, Bau-Kuchen (Kolonie / Ziel / Wirtschaft), Lueckenfueller.
pub const B_KOLONIE: i32 = 0;
pub const B_ZIEL: i32 = 1;
pub const B_WIRTSCHAFT: i32 = 2;
pub const B_VERTEIDIGUNG: i32 = 3;
pub const B_ANZAHL: i32 = 4;
pub const BEREICH: [&str; B_ANZAHL as usize] = ["Kolonie", "Ziel", "Wirtschaft", "Verteidigung"];

// Kampf: Lagebild der Gegner (Kartenscan), Bedrohung, Soll-Staerke.
pub const SCAN_TAKT: i32 = 200;
pub const WERTUNG_TICK: i32 = 200000;   // Spieler 2026-11-05: lange Spiele, damit Raumhafen/EMP-Raketen zaehlen (vorher 80000)
pub const WERT_WT: i32 = 4;     // Kampfwert eines Laser-Wachturms nach SDK (fuer Geplantes)
pub const WERT_LYNX: i32 = 5;   // Laser-Lynx nach SDK
pub const MAX_WT: i32 = 8;
pub const AUSSEN_R: i32 = 12;   // Minen weiter weg von der Basismitte sind Aussenposten

// Kampfmodell aus der Arena: Gegenwert g = Waffenfaktor x Fahrgestellfaktor x Wurzel(HP x Schaden je Tick).
/// Faktoren des Arena-8-Modells (auswertung/kampf_fit.py, 14076 Kaempfe Stufe 9/12, Test 74,5 % statt 66,2 %). Waffen ohne
/// Messung (Railgun, Energiekanone): alter Faktor im Verhaeltnis Laser neu/alt.
pub fn waffenFaktorNeu(w: i32) -> f64 {
    match w {
        mapAcidCloud => 3.57, mapESG => 1.65, mapLaser => 0.49, mapMicrowave => 0.52, mapRPG => 0.66, mapThorsHammer => 0.85,
        mapRailGun => 0.62, mapEnergyCannon => 0.50, mapEMP => 1.20, _ => 0.49,
    }
}
/// Gestellfaktor des Arena-8-Modells (Skorpion und Wachturm ohne Messung: alter Faktor im Verhaeltnis Lynx neu/alt).
pub fn gestellFaktorNeu(f: i32) -> f64 {
    if f == mapLynx { 0.96 } else if f == mapPanther { 0.85 } else if f == mapTiger { 1.23 } else if f == mapScorpion { 0.62 } else { 0.90 }
}
/// Verstaerkung der Hauptgruppe durch einen EMP-Anteil q: Staerke x (1 + a x q) (Arena 8; ungemessene Waffen: Mittel 0,15).
pub fn empVerstaerkung(w: i32) -> f64 {
    match w {
        mapLaser => 0.37, mapMicrowave => 0.28, mapRPG => 0.10, mapAcidCloud => 0.09, mapThorsHammer => 0.06, mapESG => 0.0, _ => 0.15,
    }
}


/// C++: double gegenwert(int spieler, int traeger, int waffe, double hpAnteil = 1.0)  (op2_ki.hpp:1007)
pub fn gegenwert(welt: &mut dyn Welt, spieler: i32, traeger: i32, waffe: i32, hpAnteil: f64) -> f64 {
    let mut waffe = waffe;
    // EMP mit seinen eigenen SDK-Werten (betaeubt ueber die Erschuetterung) und eigenem Faktor; Schaum zaehlt wie Laser.
    if waffe == mapStickyfoam { waffe = mapLaser; }
    else if waffe == mapNone || waffe <= 0 || waffe >= 128 { waffe = if traeger == mapScorpion { mapEnergyCannon } else { mapLaser }; }
    let r = welt.GetReloadTime(waffe, spieler);
    let dps = (0.9 * welt.GetPenetrationDamage(waffe, spieler) as f64 + 0.33 * welt.GetConcussionDamage(waffe, spieler) as f64 * 1.1)
        / (if r > 0 { r } else { 1 }) as f64;
    let hp = welt.GetHitPoints(traeger, spieler) as f64 * (if hpAnteil > 0.05 { hpAnteil } else { 0.05 });
    // Massstab: ein Lynx mit Laser hat denselben Wert wie im alten Modell (0,426 x 0,67 x 1,07 = 0,649 x 0,49 x 0,96).
    0.649 * waffenFaktorNeu(waffe) * gestellFaktorNeu(traeger) * (hp * dps).sqrt()
}

/// Panzerwert der Simulation je Panzerklasse 0..5 (op2-rs rules.rs: Anteil des Erschuetterungsschadens / 256)
pub const PANZERWERT: [i32; 6] = [256, 180, 130, 90, 60, 40];
pub fn panzerWert(klasse: i32) -> i32 { PANZERWERT[klasse.clamp(0, 5) as usize] }
/// Panzer laut Welt als Panzerwert (liefert eine Welt die Klasse 0..5, wird sie umgerechnet)
pub fn panzerJetzt(welt: &mut dyn Welt, typ: i32, spieler: i32) -> i32 { let a = welt.GetArmor(typ, spieler); if a <= 5 { panzerWert(a) } else { a } }

/// Mittlerer Erschuetterungs- und Durchschlagsschaden je Treffer der Kampfeinheiten und Wachtuerme aller Feinde von `spieler`
/// (Schaden = Panzerwert x Erschuetterung / 256 + Durchschlag); ohne Einheiten die Grundwaffe der Feind-Fraktion. Je Tick gemerkt.
pub fn feindMix(welt: &mut dyn Welt, spieler: i32) -> (f64, f64) {
    thread_local! { static MIX: std::cell::RefCell<[(i32, f64, f64); 8]> = const { std::cell::RefCell::new([(-1, 0.0, 0.0); 8]) }; }
    let t = welt.Tick();
    let sp = (spieler.clamp(0, 7)) as usize;
    if let Some(m) = MIX.with(|c| { let a = c.borrow()[sp]; if a.0 == t { Some((a.1, a.2)) } else { None } }) { return m; }
    let (mut k, mut pe, mut n) = (0.0f64, 0.0f64, 0);
    for p in 0..welt.NoPlayers() {
        if p == spieler || welt.IsAlliedTo(spieler, p) { continue; }
        let mut dazu = |welt: &mut dyn Welt, wf: i32| {
            if wf <= 0 || wf >= 128 { return; }
            k += welt.GetConcussionDamage(wf, p) as f64; pe += welt.GetPenetrationDamage(wf, p) as f64; n += 1;
        };
        for u in welt.PlayerVehicleEnum(p) {
            if !welt.IsLive(u) { continue; }
            let ty = welt.GetType(u);
            if ty != mapLynx && ty != mapPanther && ty != mapTiger && ty != mapScorpion { continue; }
            let wf = welt.GetWeapon(u); dazu(welt, wf);
        }
        for u in welt.PlayerBuildingEnum(p, mapGuardPost) { if welt.IsLive(u) { let wf = welt.GetWeapon(u); dazu(welt, wf); } }
    }
    if n == 0 {
        for p in 0..welt.NoPlayers() {
            if p == spieler || welt.IsAlliedTo(spieler, p) { continue; }
            let wf = if welt.IsEden(p) { mapLaser } else { mapMicrowave };
            k += welt.GetConcussionDamage(wf, p) as f64; pe += welt.GetPenetrationDamage(wf, p) as f64; n += 1;
        }
    }
    let m = if n > 0 { (k / n as f64, pe / n as f64) } else { (0.0, 0.0) };
    MIX.with(|c| c.borrow_mut()[sp] = (t, m.0, m.1));
    m
}

/// Schaden je Treffer der Feinde von `spieler` gegen Panzerwert `pw`
pub fn feindTreffer(welt: &mut dyn Welt, spieler: i32, pw: i32) -> f64 {
    let (k, p) = feindMix(welt, spieler);
    pw as f64 * k / 256.0 + p
}

/// C++: int kampfWert(UnitEx u)  (op2_ki.hpp:1018)
pub fn kampfWert(welt: &mut dyn Welt, u: Id) -> i32 {
    let t = welt.GetType(u); let w = welt.GetWeapon(u);
    if t == mapSpider { return 1; }                                  // repariert nur
    if t != mapLynx && t != mapPanther && t != mapTiger && t != mapScorpion && t != mapGuardPost { return 0; }
    // Ein Wachturm ohne Roehrenverbindung zur Kommandozentrale hat keinen Strom und macht nur einen Teil des Schadens
    // (im Original gemessen 2026-11-17: ~230 statt ~425 Schaden an Lynx, kein Abschuss) -- er zaehlt mit PAR turmOhneNetz.
    let ohneNetz = if t == mapGuardPost && !turmAngeschlossen(welt, u) {
        PAR(g_kraftSpieler.with(|c| c.get())).turmOhneNetz } else { 1.0 };
    let hp = welt.GetHitPoints(t, welt.OwnerID(u));
    let anteil = if hp > 0 { 1.0 - welt.GetDamage(u) as f64 / hp as f64 } else { 1.0 };
    let o = welt.OwnerID(u);
    let g = (gegenwert(welt, o, t, w, anteil) * ohneNetz + 0.5) as i32;
    if g > 0 { g } else { 1 }
}

// Kampffahrzeug fuer die eigenen Gruppen (der Spider zaehlt im SDK, repariert aber nur)
/// C++: bool kaempfer(UnitEx u)  (op2_ki.hpp:1028)
pub fn kaempfer(welt: &mut dyn Welt, u: Id) -> bool { welt.IsVehicle(u) && welt.GetType(u) != mapSpider && kampfWert(welt, u) > 0 }

// EMP-Rakete (nur Plymouth): Einschlag 200-300 Ticks nach dem Start, Radius ~6-7 Kacheln, laehmt ~700-750 Ticks.
pub const EMP_TECH: i32 = 12201;
pub const EMP_RADIUS: i32 = 6;
pub const EMP_FLUG: i32 = 300;
pub const EMP_AUFMARSCH: i32 = 4;

/// C++: bool empMoeglich(const Zustand& z)  (op2_ki.hpp:1038)
pub fn empMoeglich(welt: &mut dyn Welt, z: &Zustand) -> bool { !welt.IsEden(z.spieler) && hatTech(welt, z, EMP_TECH) }

// Bestes Ziel: die Stelle, an der im EMP-Radius die meiste feindliche Abwehr steht. zx/zy in Editor-Kacheln.
/// C++: int empZiel(const Zustand& z, int& zx, int& zy)  (op2_ki.hpp:1041)
pub fn empZiel(welt: &mut dyn Welt, z: &Zustand, zx: &mut i32, zy: &mut i32) -> i32 {
    let mut px = [0i32; 128]; let mut py = [0i32; 128]; let mut w = [0i32; 128]; let mut n = 0usize;
    for p in 0..welt.NoPlayers() {
        if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { continue; }
        for u in welt.PlayerVehicleEnum(p) {
            if n >= 128 { break; }
            if welt.IsLive(u) && kaempfer(welt, u) { let l = welt.Location(u); px[n] = edX(welt, z, l); py[n] = edY(welt, z, l); w[n] = kampfWert(welt, u); n += 1; }
        }
        for u in welt.PlayerBuildingEnum(p, mapGuardPost) {
            if n >= 128 { break; }
            if welt.IsLive(u) { let l = welt.Location(u); px[n] = edX(welt, z, l); py[n] = edY(welt, z, l); w[n] = kampfWert(welt, u); n += 1; }
        }
    }
    let mut beste = 0; *zx = 0; *zy = 0;
    for i in 0..n {
        let mut s = 0;
        for j in 0..n { if abstand2(welt, px[i], py[i], px[j], py[j]) <= EMP_RADIUS * EMP_RADIUS { s += w[j]; } }
        if s > beste { beste = s; *zx = px[i]; *zy = py[i]; }
    }
    beste
}

// Kosten einer Rakete in Kampfwert: so viele beste Kampffahrzeuge haette man fuer dasselbe Erz (Seltenerz doppelt).
/// C++: int empKostenWert(const Zustand& z)  (op2_ki.hpp:1063)
pub fn empKostenWert(welt: &mut dyn Welt, z: &Zustand) -> i32 {
    let r = mapEMPMissile;
    let mut fz = fahrgestellWahl(welt, z, 1 << 20); if fz == 0 { fz = mapLynx; }
    let wf = waffeWahl(welt, z, fz, 1 << 20);
    let s = z.spieler;
    let fzKosten = welt.GetOreCost(fz, s) + welt.GetOreCost(wf, s) + 2 * (welt.GetRareOreCost(fz, s) + welt.GetRareOreCost(wf, s));
    let rKosten = (if welt.GetOreCost(r, s) > 0 { welt.GetOreCost(r, s) } else { 3000 })
        + 2 * (if welt.GetRareOreCost(r, s) > 0 { welt.GetRareOreCost(r, s) } else { 1500 });
    if fzKosten > 0 { rKosten * sdkWertT(welt, fz, wf) / fzKosten } else { 100 }
}

// Lohnt eine Rakete gegen die jetzige Abwehr?
/// C++: bool empLohnt(const Zustand& z, int* wert = nullptr)  (op2_ki.hpp:1073)
pub fn empLohnt(welt: &mut dyn Welt, z: &Zustand, wert: Option<&mut i32>) -> bool {
    let (mut zx, mut zy) = (0, 0); let v = empZiel(welt, z, &mut zx, &mut zy);
    if let Some(wp) = wert { *wp = v; }
    v > 0 && v as f64 >= empKostenWert(welt, z) as f64 * PAR(z.spieler).empSchwelle
}

/// C++: bool istKampfFz(int typ)  (op2_ki.hpp:1079)
pub fn istKampfFz(_welt: &mut dyn Welt, typ: i32) -> bool { typ == mapLynx || typ == mapPanther || typ == mapTiger }

/// C++: int reparaturTyp(const Zustand& z)  (op2_ki.hpp:1081)
pub fn reparaturTyp(welt: &mut dyn Welt, z: &Zustand) -> i32 { if welt.IsEden(z.spieler) { mapRepairVehicle } else { mapSpider } }

/// C++: double kraft(const Kraftwerte& v, int waffe)  (op2_ki.hpp:1084)
pub fn kraft(_welt: &mut dyn Welt, v: &Kraftwerte, waffe: i32) -> f64 {
    if v.reload <= 0 || v.hp <= 0 { return 0.0; }
    let mut dps = (v.pen as f64 + 0.5 * v.kon as f64) / v.reload as f64;
    if waffe == mapEMP { dps = 0.9; }                                  // betaeubt nur kurz: wie eine Laser-Klasse-Waffe
    (dps * v.hp as f64 * (1.0 + 0.15 * v.panzer as f64)).sqrt()
}

/// C++: int sdkWertT(int traeger, int waffe)  (op2_ki.hpp:1091)
pub fn sdkWertT(welt: &mut dyn Welt, traeger: i32, waffe: i32) -> i32 {
    if traeger != mapLynx && traeger != mapPanther && traeger != mapTiger && traeger != mapScorpion && traeger != mapGuardPost {
        return if traeger == mapSpider { 1 } else { 0 };
    }
    let ks = g_kraftSpieler.with(|c| c.get());
    let g = (gegenwert(welt, ks, traeger, waffe, 1.0) + 0.5) as i32;
    if g > 0 { g } else { 1 }
}

// Kampfkraft aus den aktuellen Werten (UnitInfo je Spieler, mit allem Erforschten)
/// C++: double kraftJetzt(int spieler, int traeger, int waffe)  (op2_ki.hpp:1098)
pub fn kraftJetzt(welt: &mut dyn Welt, spieler: i32, traeger: i32, waffe: i32) -> f64 {
    let v = Kraftwerte {
        hp: welt.GetHitPoints(traeger, spieler), panzer: welt.GetArmor(traeger, spieler),
        pen: welt.GetPenetrationDamage(waffe, spieler), kon: welt.GetConcussionDamage(waffe, spieler),
        reload: welt.GetReloadTime(waffe, spieler),
    };
    kraft(welt, &v, waffe)
}

/// C++: bool einheitErforscht(const Zustand& z, int typ)  (op2_ki.hpp:1104)
pub fn einheitErforscht(welt: &mut dyn Welt, z: &Zustand, typ: i32) -> bool {
    match tb::einheit(typ) {
        Some(e) => e.tech > 0 && hatTech(welt, z, e.tech),
        None => false,
    }
}

// Gesamtstaerke, die der Vorrat ergibt: Gegenwert x Anzahl, das knappere Erz begrenzt.
/// C++: double staerkeAusVorrat(const Zustand& z, int traeger, int waffe, int erzDa, int seltenDa)  (op2_ki.hpp:1115)
pub fn staerkeAusVorrat(welt: &mut dyn Welt, z: &Zustand, traeger: i32, waffe: i32, erzDa: i32, seltenDa: i32) -> f64 {
    let s = z.spieler;
    let oe = welt.GetOreCost(traeger, s) + welt.GetOreCost(waffe, s);
    let se = welt.GetRareOreCost(traeger, s) + welt.GetRareOreCost(waffe, s);
    let mut n = if oe > 0 { erzDa as f64 / oe as f64 } else { 1e6 };
    if se > 0 && (seltenDa as f64 / se as f64) < n { n = seltenDa as f64 / se as f64; }
    n * gegenwert(welt, s, traeger, waffe, 1.0)
}

// Beste erforschte Waffe fuer einen Traeger (nicht EMP, Starflare/Supernova, Stickyfoam). Seltenerz muss reichen.
/// C++: int waffeWahl(const Zustand& z, int traeger, int seltenDa)  (op2_ki.hpp:1123)
pub fn waffeWahl(welt: &mut dyn Welt, z: &Zustand, traeger: i32, seltenDa: i32) -> i32 {
    const KANDIDATEN: [i32; 8] = [mapLaser, mapMicrowave, mapRailGun, mapRPG, mapAcidCloud, mapESG, mapThorsHammer, mapEnergyCannon];
    let mut beste = if welt.IsEden(z.spieler) { mapLaser } else { mapMicrowave };
    let erzDa = erzFuerKampf(welt, z);
    let mut bestWert = staerkeAusVorrat(welt, z, traeger, beste, erzDa, seltenDa) * super::paarung::paarFaktor(welt, z, traeger, beste);
    for &w in KANDIDATEN.iter() {
        if w == beste || !einheitErforscht(welt, z, w) { continue; }
        if welt.GetRareOreCost(w, z.spieler) > seltenDa { continue; }
        let mut v = staerkeAusVorrat(welt, z, traeger, w, erzDa, seltenDa) * super::paarung::paarFaktor(welt, z, traeger, w);
        // Turm (Stellschraube turmUeberMauer): Waffen, die ueber Mauern schiessen, gewichten
        if traeger == mapGuardPost && matches!(w, mapAcidCloud | mapESG | mapRPG | mapThorsHammer) { v *= PAR(z.spieler).turmUeberMauer; }
        if v > bestWert { bestWert = v; beste = w; }
    }
    beste
}

// Bestes erforschtes Kampf-Fahrgestell (Tiger > Panther > Lynx), Seltenerz muss reichen. 0 = keins.
/// C++: int fahrgestellWahl(const Zustand& z, int seltenDa)  (op2_ki.hpp:1139)
pub fn fahrgestellWahl(welt: &mut dyn Welt, z: &Zustand, seltenDa: i32) -> i32 {
    const REIHE: [i32; 3] = [mapTiger, mapPanther, mapLynx];
    let erzDa = erzFuerKampf(welt, z);
    let mut beste = 0; let mut bestWert = -1.0;
    for &f in REIHE.iter() {
        if !einheitErforscht(welt, z, f) { continue; }
        let fs = welt.GetRareOreCost(f, z.spieler);
        if fs > seltenDa { continue; }
        let w = waffeWahl(welt, z, f, seltenDa - fs);
        let v = staerkeAusVorrat(welt, z, f, w, erzDa, seltenDa) * super::paarung::paarFaktor(welt, z, f, w);
        if v > bestWert { bestWert = v; beste = f; }
    }
    beste
}

// Erz fuer die Fahrzeugwahl: Vorrat plus Zufluss im Planungshorizont, mindestens 1
/// C++: int erzFuerKampf(const Zustand& z)  (op2_ki.hpp:1155)
pub fn erzFuerKampf(welt: &mut dyn Welt, z: &Zustand) -> i32 {
    let e = welt.Ore(z.spieler) as f64 + (if z.zuflussErz > 0.0 { z.zuflussErz } else { 0.0 }) * PAR(z.spieler).horizont / 1000.0;
    if e > 1.0 { e as i32 } else { 1 }
}

/// C++: int eigeneWaffe(const Zustand& z)  (op2_ki.hpp:1159)
pub fn eigeneWaffe(welt: &mut dyn Welt, z: &Zustand) -> i32 { let s = welt.RareOre(z.spieler); waffeWahl(welt, z, mapGuardPost, s) }

// Kampfkraft je Erz (Seltenerz zaehlt wie Erz) der besten erforschten Kombination -- mit oder ohne Seltenerz.
/// C++: double kraftJeErz(const Zustand& z, bool mitSelten)  (op2_ki.hpp:1163)
pub fn kraftJeErz(welt: &mut dyn Welt, z: &Zustand, mitSelten: bool) -> f64 {
    const TRAEGER: [i32; 3] = [mapLynx, mapPanther, mapTiger];
    const WAFFEN: [i32; 8] = [mapLaser, mapMicrowave, mapRailGun, mapRPG, mapAcidCloud, mapESG, mapThorsHammer, mapEnergyCannon];
    let grund = if welt.IsEden(z.spieler) { mapLaser } else { mapMicrowave };
    let mut beste = 0.0;
    for &f in TRAEGER.iter() {
        if !einheitErforscht(welt, z, f) { continue; }
        for &w in WAFFEN.iter() {
            if w != grund && !einheitErforscht(welt, z, w) { continue; }
            let sel = welt.GetRareOreCost(f, z.spieler) + welt.GetRareOreCost(w, z.spieler);
            if !mitSelten && sel > 0 { continue; }
            let kosten = welt.GetOreCost(f, z.spieler) + welt.GetOreCost(w, z.spieler) + sel;
            if kosten <= 0 { continue; }
            // Rust (PAR paarung): Kampfkraft gegen die gesehene Feindmischung, nicht nur absolut
            let k = gegenwert(welt, z.spieler, f, w, 1.0) * super::paarung::paarFaktor(welt, z, f, w) / kosten as f64;
            if k > beste { beste = k; }
        }
    }
    beste
}

// Vorteil der Seltenerz-Fahrzeuge (0.3 = 30 % mehr Kampfkraft je Erz); 0 ohne Kampffahrzeug.
/// C++: double seltenVorteil(const Zustand& z)  (op2_ki.hpp:1185)
pub fn seltenVorteil(welt: &mut dyn Welt, z: &Zustand) -> f64 {
    let ohne = kraftJeErz(welt, z, false);
    if ohne > 0.0 { kraftJeErz(welt, z, true) / ohne - 1.0 } else { 0.0 }
}

// Seltenerz lohnt sich (Parameter seltenGewinn).
/// C++: bool seltenGewollt(const Zustand& z)  (op2_ki.hpp:1190)
pub fn seltenGewollt(welt: &mut dyn Welt, z: &Zustand) -> bool {
    seltenVorteil(welt, z) >= PAR(z.spieler).seltenGewinn && seltenVorteil(welt, z) > 0.0
}

/// C++: bool l_kolonieLebt(const Zustand& z)  (op2_ki.hpp:1193)
pub fn l_kolonieLebt(welt: &mut dyn Welt, z: &Zustand) -> bool { !welt.PlayerBuildingEnum(z.spieler, mapCommandCenter).is_empty() }

// Prognose der naechsten Welle: Trend der letzten beiden Wellen (nie kleiner als die letzte). Vor der ersten
// Welle: zwei Lynx je Feind-Fahrzeugfabrik, nach der ersten die Haelfte mehr.
/// C++: int welleProg(const Zustand& z)  (op2_ki.hpp:1198)
pub fn welleProg(_welt: &mut dyn Welt, z: &Zustand) -> i32 {
    if z.anzWellen == 0 { return z.feindFabriken * 2 * WERT_LYNX; }
    let letzte = z.wellen[(z.anzWellen - 1) as usize];
    if z.anzWellen == 1 { return letzte + letzte / 2; }
    let trend = letzte - z.wellen[(z.anzWellen - 2) as usize];
    letzte + if trend > 0 { trend } else { 0 }
}
