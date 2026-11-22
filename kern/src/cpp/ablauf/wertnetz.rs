//! Wertnetz (netz::): kleines gelerntes Netz, das Lage und Angriffe bewertet (nur Spur/Meldungen, falls geladen).
use super::*;
use super::super::*;
use super::{WERTUNG_TICK, SCAN_TAKT, B_ANZAHL, GEBAEUDE, OreTypeCommon, Bar3, Bar2, VariantRandom, KI_MESSUNG};

// ---------------------------------------------------------------- netz:: (kleines Netz aus Datei)

/// C++: bool laden(Netz& n, const char* name)  (op2_ki.hpp:6343)
pub fn laden(welt: &mut dyn Welt, n: &mut Netz, name: &str) -> bool {
    if n.versucht != 0 { return n.geladen != 0; }
    n.versucht = 1;
    let datei = format!("KI-Netz-{}.txt", name);
    let mut pfad = String::new();
    wissenPfad(welt, &mut pfad, 600, &datei);
    let text = match welt.dateiLesen(&pfad) { Some(t) => t, None => return false };
    let mut f = Leser::neu(&text);
    fn kopf(f: &mut Leser, n: &mut Netz) -> Option<(String, i32)> {
        let kopf = f.wort(15)?;
        let _nm = f.wort(31)?;
        let ver = f.ganz()?;
        n.ein = f.ganz()?;
        n.aus = f.ganz()?;
        n.schichten = f.ganz()?;
        Some((kopf, ver))
    }
    let mut ok = match kopf(&mut f, n) {
        Some((kopf, ver)) => kopf == "KI-NETZ" && ver == MERKMAL_VERSION && n.ein > 0 && n.ein <= MAX_EIN as i32 &&
            n.aus > 0 && n.aus <= MAX_AUS as i32 && n.schichten > 0 && n.schichten <= MAX_SCHICHT as i32,
        None => false,
    };
    let mut i = 0;
    while ok && i < n.ein { match f.zahl() { Some(v) => n.mx[i as usize] = v, None => ok = false } i += 1; }
    i = 0;
    while ok && i < n.ein { match f.zahl() { Some(v) => n.sx[i as usize] = v, None => ok = false } i += 1; }
    i = 0;
    while ok && i < n.aus { match f.zahl() { Some(v) => n.my[i as usize] = v, None => ok = false } i += 1; }
    i = 0;
    while ok && i < n.aus { match f.zahl() { Some(v) => n.sy[i as usize] = v, None => ok = false } i += 1; }
    let mut k = 0usize;
    while ok && (k as i32) < n.schichten {
        let mut akt = String::new();
        ok = (|| -> Option<()> {
            n.zeilen[k] = f.ganz()?;
            n.spalten[k] = f.ganz()?;
            akt = f.wort(15)?;
            Some(())
        })().is_some() && n.zeilen[k] <= MAX_EIN as i32 &&
             n.spalten[k] <= MAX_BREIT as i32 && n.zeilen[k] * n.spalten[k] <= (MAX_EIN * MAX_BREIT) as i32;
        if !ok { break; }
        n.tanh_[k] = if akt == "tanh" { 1 } else { 0 };
        let mut i = 0;
        while ok && i < n.zeilen[k] * n.spalten[k] { match f.zahl() { Some(v) => n.W[k][i as usize] = v, None => ok = false } i += 1; }
        let mut i = 0;
        while ok && i < n.spalten[k] { match f.zahl() { Some(v) => n.b[k][i as usize] = v, None => ok = false } i += 1; }
        k += 1;
    }
    n.geladen = if ok { 1 } else { 0 };
    ok
}

/// C++: bool rechnen(const Netz& n, const float* x, int anzahl, float* out)  (op2_ki.hpp:6374)
// Vorwaertsrechnung; out bekommt die Ausgaenge in Originalskala. Forward pass, outputs de-normalised.
pub fn rechnen(welt: &mut dyn Welt, n: &Netz, x: &[f32], anzahl: i32, out: &mut [f32]) -> bool {
    if n.geladen == 0 || anzahl != n.ein { return false; }
    let mut a = [0f32; if MAX_BREIT > MAX_EIN { MAX_BREIT } else { MAX_EIN }];
    let mut neu = [0f32; MAX_BREIT];
    // begrenzt: Lagen weit ausserhalb der Trainingsdaten sollen das Netz nicht in die Saettigung treiben
    for i in 0..n.ein as usize {
        let v = (x[i] - n.mx[i]) / n.sx[i];
        a[i] = if v > 6.0 { 6.0 } else if v < -6.0 { -6.0 } else { v };
    }
    let mut breite = n.ein as usize;
    for k in 0..n.schichten as usize {
        let sp = n.spalten[k] as usize;
        for j in 0..sp {
            let mut s = n.b[k][j];
            for i in 0..breite { s += a[i] * n.W[k][i * sp + j]; }
            neu[j] = if n.tanh_[k] != 0 { s.tanh() } else { s };
        }
        breite = sp;
        for j in 0..breite { a[j] = neu[j]; }
    }
    for j in 0..n.aus as usize { out[j] = a[j] * n.sy[j] + n.my[j]; }
    true
}

/// C++: int netzMerkmale(const Zustand& z, const Lage& l, float* v)  (op2_ki.hpp:6396)
// Merkmale wie ki-test/merkmale.py (Reihenfolge und Skalen gleich). Features as in merkmale.py.
pub fn netzMerkmale(welt: &mut dyn Welt, z: &Zustand, l: &Lage, v: &mut [f32]) -> i32 {
    let pl = z.spieler;
    let mut n = 0usize;
    let kinder = welt.Kids(pl);
    let arbeiter = welt.Workers(pl);
    let wiss = welt.Scientists(pl);
    macro_rules! dazu { ($e:expr) => {{ let w: f32 = $e; v[n] = w; n += 1; }}; }
    dazu!(welt.Tick() as f32 / 100000.0); dazu!((kinder + arbeiter + wiss) as f32 / 100.0);
    dazu!(kinder as f32 / 100.0); dazu!(arbeiter as f32 / 100.0); dazu!(wiss as f32 / 100.0);
    dazu!(l.erz as f32 / 10000.0); dazu!(welt.RareOre(pl) as f32 / 10000.0); dazu!(l.nahrung as f32 / 10000.0); dazu!(l.nahrungNetto as f32 / 100.0);
    let mz = moralZahl(welt, z);
    dazu!(mz as f32 / 100.0);
    dazu!((welt.GetAmountPowerGenerated(pl) - welt.GetAmountPowerConsumed(pl)) as f32 / 1000.0);
    const GEB: [i32; 14] = [mapCommonOreSmelter, mapCommonOreMine, mapRareOreSmelter, mapRareOreMine, mapVehicleFactory,
                            mapStandardLab, mapAdvancedLab, mapResidence, mapGuardPost, mapAgridome, mapTokamak,
                            mapNursery, mapUniversity, mapMedicalCenter];
    for t in GEB { dazu!(l.geb[t as usize] as f32 / 10.0); }
    const FZ: [i32; 6] = [mapLynx, mapPanther, mapTiger, mapCargoTruck, mapConVec, mapRoboMiner];
    for t in FZ { dazu!(l.fz[t as usize] as f32 / 20.0); }
    dazu!(z.feindArmee as f32 / 100.0); dazu!(z.feindNah as f32 / 100.0); dazu!(z.feindGeb as f32 / 20.0); dazu!(z.eigeneArmee as f32 / 100.0);
    dazu!(z.sollArmee as f32 / 100.0); dazu!(z.bedrohung as f32 / 100.0); dazu!(z.eigeneMobil as f32 / 100.0); dazu!(z.kampfModus as f32);
    dazu!(if z.ziel == 2 { 1.0 } else { 0.0 }); dazu!(if z.ziel == 3 { 1.0 } else { 0.0 });
    n as i32
}

/// C++: int netzParameter(const Zustand& z, float* v)  (op2_ki.hpp:6416)
// Stellschrauben auf 0..1 wie ki-test/optimierer.py (PARAMETER).
pub fn netzParameter(welt: &mut dyn Welt, z: &Zustand, v: &mut [f32]) -> i32 {
    let q = PAR(z.spieler);
    let g: [(f64, f64, f64); 21] = [(q.reaktion, 2000.0, 15000.0), (q.horizont, 5000.0, 50000.0), (q.ueberlegenheit, 0.8, 3.0),
                  (q.fabrikGewinn, 0.4, 1.0), (q.reserveStart, 0.8, 3.0), (q.reserveLernrate, 0.0, 1.0),
                  (q.reserveRueckgang, 0.8, 1.0), (q.reserveMax, 1.0, 4.0), (q.turmAnteil, 0.0, 0.8),
                  (q.zermuerbAbstand, 2000.0, 30000.0), (q.schmelzeNutzen, 0.3, 3.0), (q.wohnraumGrenze, 60.0, 98.0),
                  (q.freischaltWert, 0.0, 1.0), (10.0, 2.0, 60.0), (40.0, 10.0, 90.0),   // entfallene Reparatur-Schwellen: alte Standardwerte, Netz-Format bleibt
                  (q.nahRadius, 12.0, 40.0), (q.heimHalten, 0.0, 2.0), (q.steigerung, 1.0, 1.5), (q.vorsorge, 0.0, 100.0),
                  (q.schmelzenJeMine, 0.5, 2.0), (q.empSchwelle, 0.1, 5.0)];
    let mut n = 0usize;
    for e in g.iter() { v[n] = ((e.0 - e.1) / (e.2 - e.1)) as f32; n += 1; }
    n as i32
}

/// C++: float netzWert(const Zustand& z, const Lage& l)  (op2_ki.hpp:6431)
// Erwartete Punkte bei Tick 80000 laut Wertnetz (oder -1e9 ohne Netz). Value-net prediction.
pub fn netzWert(welt: &mut dyn Welt, z: &Zustand, l: &Lage) -> f32 {
    if !NETZ_WERT.with(|nw| laden(welt, &mut nw.borrow_mut(), "wert")) { return -1e9; }
    let mut v = [0f32; MAX_EIN];
    let mut n = netzMerkmale(welt, z, l, &mut v);
    n += netzParameter(welt, z, &mut v[n as usize..]);
    let mut out = [0f32; MAX_AUS];
    if NETZ_WERT.with(|nw| rechnen(welt, &nw.borrow(), &v, n, &mut out)) { out[0] } else { -1e9 }
}

/// C++: bool netzAngriff(const Zustand& z, const Lage& l, int art, int fahrzeuge, int wertTruppe, float& zer, float& ver)  (op2_ki.hpp:6438)
// Erwartete Zerstoerung/Verluste eines Angriffs laut Angriffsnetz. Attack-net prediction.
pub fn netzAngriff(welt: &mut dyn Welt, z: &Zustand, l: &Lage, art: i32, fahrzeuge: i32, wertTruppe: i32, zer: &mut f32, ver: &mut f32) -> bool {
    if !NETZ_ANGRIFF.with(|na| laden(welt, &mut na.borrow_mut(), "angriff")) { return false; }
    let mut v = [0f32; MAX_EIN];
    let mut n = netzMerkmale(welt, z, l, &mut v) as usize;
    for k in 0..3 { v[n] = if art == k { 1.0 } else { 0.0 }; n += 1; }
    v[n] = fahrzeuge as f32 / 50.0; n += 1;
    v[n] = wertTruppe as f32 / 300.0; n += 1;
    n += netzParameter(welt, z, &mut v[n..]) as usize;
    let mut out = [0f32; MAX_AUS];
    if !NETZ_ANGRIFF.with(|na| rechnen(welt, &na.borrow(), &v, n as i32, &mut out)) { return false; }
    *zer = out[0]; *ver = out[1];
    true
}
