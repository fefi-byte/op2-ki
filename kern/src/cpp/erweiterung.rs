//! Erweiterungen der Rust-Fassung (nicht in der C++-KI), zuschaltbar ueber PAR: Gelaende (Tuerme und Wache an der Engstelle).
//! Extensions of the Rust version, switched by parameters: terrain (towers and guard at the chokepoint).
use std::cell::RefCell;

use super::gelaende::{Engstelle, Gelaende};
use super::*;

/// Gelaende eines Spielers: einmal gerechnet, alle 10000 Ticks neu (Mauern veraendern die Karte).
#[derive(Default)]
pub struct ErwGelaende { pub tick: i32, pub feind: (i32, i32), pub g: Option<Gelaende>, pub eng: Option<Engstelle> }

thread_local! {
    pub static GELAENDE: RefCell<[Option<ErwGelaende>; 8]> = RefCell::new(Default::default());
    static WELTBILD: RefCell<[bool; 8]> = const { RefCell::new([false; 8]) };
}
pub fn zuruecksetzen_erweiterung() {
    GELAENDE.with(|g| *g.borrow_mut() = Default::default());
    WELTBILD.with(|w| *w.borrow_mut() = [false; 8]);
    super::zweitbasis::zuruecksetzen_zweitbasis();
    super::vorplan::zuruecksetzen_vorplan();
    super::gefecht::zuruecksetzen_gefecht();
    super::paarung::zuruecksetzen_paarung();
    super::hilfen::zuruecksetzen_turmnetz();
    super::mauer::zuruecksetzen_mauer();
    super::bezirke::zuruecksetzen_bezirke();
}

/// Naechste feindliche Kommandozentrale (Editor-Kacheln), sonst irgendein feindliches Gebaeude.
pub fn feindBasis(welt: &mut dyn Welt, z: &Zustand) -> Option<(i32, i32)> {
    let mut best: Option<(i32, (i32, i32))> = None;
    for p in 0..welt.NoPlayers() {
        if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { continue; }
        let mut ids = welt.PlayerBuildingEnum(p, mapCommandCenter);
        if ids.is_empty() { ids = welt.PlayerBuildingEnum(p, mapNone); }
        for u in ids {
            let l = welt.Location(u);
            let (x, y) = (edX(welt, z, l), edY(welt, z, l));
            let d = abstand2(welt, x, y, z.bx, z.by);
            if best.is_none_or(|b| d < b.0) { best = Some((d, (x, y))); }
        }
    }
    best.map(|b| b.1)
}

/// Gelaende rechnen (oder aus dem Speicher). Liefert die gewaehlte Engstelle (die naechste im Suchbereich).
pub fn engstelle(welt: &mut dyn Welt, z: &Zustand) -> Option<Engstelle> {
    let p = PAR(z.spieler);
    if p.gelaende <= 0.0 { return None; }
    let sp = (z.spieler & 7) as usize;
    let t = welt.Tick();
    let zug = p.zugang >= 0.5;
    let neu = GELAENDE.with(|g| g.borrow()[sp].as_ref().is_none_or(|e| t - e.tick >= if zug { 3000 } else { 10000 }));
    if neu {
        let feind = feindBasis(welt, z)?;
        let (b, h) = (z.breite, z.hoehe);
        let mut zellen = vec![0i32; (b * h) as usize];
        for y in 0..h { for x in 0..b { let l = engine(welt, z, x, y); zellen[(y * b + x) as usize] = welt.GetCellType(l); } }
        let (g, eng) = if zug {
            // Rust (PAR zugang, Nutzer 2026-11-17): Zugaenge zum ganzen Kolonie-Raum (Gebaeude, Minen, Lastwagenwege) statt
            // zur Basismitte; die schmalste Stelle im Suchbereich, egal wie breit.
            let (eigen, feindK) = kolonieRaum(welt, z);
            if eigen.is_empty() || feindK.is_empty() { return None; }
            let g = Gelaende::neu_quellen(&zellen, b, h, &eigen, &feindK, &[], 0.4, p.engMin as i32, (p.engMax as i32).min(16), 999);
            // Wertung: Breite + halber Abstand vor der Kolonie (nah und schmal); die bisherige bleibt, solange die neue nicht
            // mindestens 15 % besser ist (sonst springt die Wache bei jeder Neuberechnung)
            let wert = |e: &Engstelle| e.breite as f64 + 0.5 * e.abstand as f64;
            let mut eng = g.engstellen.iter().min_by(|a, b| wert(a).partial_cmp(&wert(b)).unwrap()).cloned();
            let alt = GELAENDE.with(|gg| gg.borrow()[sp].as_ref().and_then(|e| e.eng.clone()));
            if let (Some(n), Some(a)) = (eng.as_ref(), alt.as_ref()) {
                if let Some(a2) = g.engstellen.iter().find(|e| (e.abstand - a.abstand).abs() <= 1) {
                    if wert(n) > wert(a2) * 0.85 { eng = Some(a2.clone()); }
                }
            }
            (g, eng)
        } else {
            let g = Gelaende::neu(&zellen, b, h, (z.bx, z.by), feind, 0.4, p.engMin as i32, p.engMax as i32, p.engBreite as i32);
            let eng = g.engstellen.first().cloned();
            (g, eng)
        };
        if let Some(e) = &eng {
            melde(welt, z, "gelaende", mapNone, e.paesse[0].mitte.0, e.paesse[0].mitte.1, &format!(
                "Engstelle {} Wegkacheln vor der Basis, {} Kacheln breit, {} Pass/Paesse (Weg zum Feind {})", e.abstand, e.breite, e.paesse.len(), g.weg));
        } else {
            melde(welt, z, "gelaende", mapNone, z.bx, z.by, &format!("keine Engstelle bis {} Wegkacheln (Weg zum Feind {}) -- offenes Feld", p.engMax as i32, g.weg));
        }
        GELAENDE.with(|gg| gg.borrow_mut()[sp] = Some(ErwGelaende { tick: t, feind, g: Some(g), eng }));
    }
    GELAENDE.with(|g| g.borrow()[sp].as_ref().and_then(|e| e.eng.clone()))
}

/// Wachturm an einem Pass: je Pass bis engTuerme Tuerme, engHinter Kacheln hinter dem Querschnitt, 2 Kacheln
/// auseinander, freies Schussfeld zum Feind. Liefert den Platz-Index wie koloniePlatz, -1 = kein Platz (dann wie bisher).
pub fn passTurm(welt: &mut dyn Welt, z: &mut Zustand) -> i32 {
    let Some(e) = engstelle(welt, z) else { return -1 };
    let p = PAR(z.spieler);
    let sp = (z.spieler & 7) as usize;
    let mut hin = [Rechteck::default(); 160];
    let nh = hindernisse(welt, z, &mut hin, 160);
    let tuerme: Vec<(i32, i32)> = welt.PlayerBuildingEnum(z.spieler, mapGuardPost).into_iter()
        .map(|u| { let l = welt.Location(u); (edX(welt, z, l), edY(welt, z, l)) }).collect();
    let geplant: Vec<(i32, i32)> = (0..z.anzahlPlaetze as usize).filter(|&i| z.plaetze[i].typ as i32 == mapGuardPost && (z.plaetze[i].zustand == P_FAHRT || z.plaetze[i].zustand == P_BAUT))
        .map(|i| (z.plaetze[i].x0 as i32, z.plaetze[i].y0 as i32)).collect();
    let d_eigen = GELAENDE.with(|g| g.borrow()[sp].as_ref().and_then(|x| x.g.as_ref().map(|g| (g.d_eigen.clone(), g.b))));
    let Some((d_eigen, b)) = d_eigen else { return -1 };
    let ziel = e.abstand - p.engHinter as i32;
    for pass in &e.paesse {
        let nah = |t: &(i32, i32)| pass.kacheln.iter().any(|k| (k.0 - t.0).abs() <= 4 && (k.1 - t.1).abs() <= 4);
        let schon = tuerme.iter().chain(geplant.iter()).filter(|t| nah(t)).count();
        if schon as f64 >= p.engTuerme { continue; }
        // Kandidaten: Kacheln im Abstand ziel..ziel+1 von der Basis, nahe am Pass, nach Naehe zur Passmitte
        let mut kand: Vec<(i32, i32, i32)> = Vec::new();
        for &(kx, ky) in &pass.kacheln {
            for dy in -4..=4 { for dx in -4..=4 {
                let (x, y) = (kx + dx, ky + dy);
                if x < 2 || y < 2 || x >= z.breite - 2 || y >= z.hoehe - 2 { continue; }
                let d = d_eigen[(y * b + x) as usize];
                if d < ziel - 1 || d > ziel + 1 { continue; }
                let m = (x - pass.mitte.0).pow(2) + (y - pass.mitte.1).pow(2);
                if !kand.iter().any(|k| k.0 == x && k.1 == y) { kand.push((x, y, m)); }
            } }
        }
        kand.sort_by_key(|k| k.2);
        for (x, y, _) in kand {
            if tuerme.iter().chain(geplant.iter()).any(|t| (t.0 - x).abs() < 2 && (t.1 - y).abs() < 2) { continue; }
            let mut g: &'static str = "";
            if !platzFrei(welt, z, x, y, 1, 1, &hin, nh, &mut g) || !schussfeldFrei(welt, z, &hin, nh, x, y, pass.zum_feind.0, pass.zum_feind.1) { continue; }
            let idx = platzEintrag(welt, z, mapGuardPost, x, y);
            if idx >= 0 {
                melde(welt, z, "kampf", mapGuardPost, x, y, &format!("Wachturm am Pass {},{} ({} Kacheln breit, {} von {} Tuermen)",
                    pass.mitte.0, pass.mitte.1, pass.kacheln.len(), schon + 1, p.engTuerme as i32));
            }
            return idx;
        }
    }
    -1
}

/// Warteplatz der Wache hinter dem (breitesten) Pass, statt 8 Kacheln Richtung Feind. None = wie bisher.
pub fn wachePunkt(welt: &mut dyn Welt, z: &Zustand) -> Option<(i32, i32)> {
    let e = engstelle(welt, z)?;
    // Rust (PAR zugang): der Zugang, den der Feind am schnellsten erreicht (kleinster Wegabstand zum Feind)
    let pass = if PAR(z.spieler).zugang >= 0.5 {
        let sp = (z.spieler & 7) as usize;
        let df = GELAENDE.with(|g| g.borrow()[sp].as_ref().and_then(|x| x.g.as_ref().map(|g| (g.d_feind.clone(), g.b))));
        match df {
            Some((d, b)) => e.paesse.iter().min_by_key(|p| { let v = d[(p.mitte.1 * b + p.mitte.0) as usize]; if v < 0 { i32::MAX } else { v } })?,
            None => e.paesse.first()?,
        }
    } else { e.paesse.first()? };
    let r = PAR(z.spieler).engHinter + 3.0;
    let x = (pass.mitte.0 as f64 - pass.zum_feind.0 * r + 0.5) as i32;
    let y = (pass.mitte.1 as f64 - pass.zum_feind.1 * r + 0.5) as i32;
    Some((x.clamp(2, z.breite - 3), y.clamp(2, z.hoehe - 3)))
}

/// Ist ein Pass der Engstelle noch nicht mit engTuerme Tuermen besetzt?
pub fn passOffen(welt: &mut dyn Welt, z: &Zustand) -> bool {
    let Some(e) = engstelle(welt, z) else { return false };
    let soll = PAR(z.spieler).engTuerme as usize;
    let tuerme: Vec<(i32, i32)> = welt.PlayerBuildingEnum(z.spieler, mapGuardPost).into_iter()
        .map(|u| { let l = welt.Location(u); (edX(welt, z, l), edY(welt, z, l)) }).collect();
    e.paesse.iter().any(|p| tuerme.iter().filter(|t| p.kacheln.iter().any(|k| (k.0 - t.0).abs() <= 4 && (k.1 - t.1).abs() <= 4)).count() < soll)
}
/// Zusaetzliche Tuerme ueber MAX_WT hinaus fuer die Paesse.
pub fn passTuermeSoll(welt: &mut dyn Welt, z: &Zustand) -> i32 {
    engstelle(welt, z).map(|e| e.paesse.len() as i32 * PAR(z.spieler).engTuerme as i32).unwrap_or(0)
}

/// Schmalen Pass bis auf eine Luecke zumauern, sobald dort ein Turm steht (der Feind muss einzeln durch). Einmal je Pass.
pub fn passMauern(welt: &mut dyn Welt, z: &mut Zustand) {
    let p = PAR(z.spieler);
    if p.gelaende <= 0.0 || p.engMauerBreite <= 0.0 || z.bauAn == 0 { return; }
    let Some(e) = engstelle(welt, z) else { return };
    thread_local! { static GEMAUERT: RefCell<Vec<(i32, i32, i32)>> = const { RefCell::new(Vec::new()) }; }
    let mut tuerme: Vec<(i32, i32)> = Vec::new();
    for u in welt.PlayerBuildingEnum(z.spieler, mapGuardPost) {
        if welt.GetCurAction(u) == moObjWait { continue; }
        let l = welt.Location(u); let (x, y) = (edX(welt, z, l), edY(welt, z, l)); tuerme.push((x, y));
    }
    for pass in &e.paesse {
        if pass.kacheln.len() as f64 > p.engMauerBreite || pass.kacheln.len() < 2 { continue; }
        let schon = GEMAUERT.with(|g| g.borrow().iter().any(|&(s, x, y)| s == z.spieler && x == pass.mitte.0 && y == pass.mitte.1));
        if schon { continue; }
        if !tuerme.iter().any(|t| pass.kacheln.iter().any(|k| (k.0 - t.0).abs() <= 4 && (k.1 - t.1).abs() <= 4)) { continue; }
        // die Kachel naechst der Mitte bleibt offen
        let offen = *pass.kacheln.iter().min_by_key(|k| (k.0 - pass.mitte.0).pow(2) + (k.1 - pass.mitte.1).pow(2)).unwrap();
        let mut n = 0;
        for &(x, y) in &pass.kacheln {
            if (x, y) == offen { continue; }
            super::mauer::vormerken(welt, z, x, y);
            n += 1;
        }
        GEMAUERT.with(|g| g.borrow_mut().push((z.spieler, pass.mitte.0, pass.mitte.1)));
        melde(welt, z, "kampf", mapWall, pass.mitte.0, pass.mitte.1, &format!("Pass {},{} bis auf eine Luecke zugemauert ({} Mauerkacheln)", pass.mitte.0, pass.mitte.1, n));
    }
}

/// Weltbild (nur Rust, Messung fuer den Abgleich Simulator/Original): einmal beim ersten Schritt alle Typwerte und
/// Koloniewerte, die die KI abfragt, als Spurzeile "art":"welt". Aendert nichts am Spiel.
pub fn weltbild(welt: &mut dyn Welt, z: &Zustand, l: &Lage) {
    let pl = z.spieler;
    if WELTBILD.with(|w| { let mut w = w.borrow_mut(); let alt = w[(pl & 7) as usize]; w[(pl & 7) as usize] = true; alt }) { return; }
    let mut p = Puffer::default();
    p.f(&format!("{{\"t\":{},\"art\":\"welt\",\"p\":{},\"typen\":{{", welt.Tick(), pl));
    let mut erst = true;
    // nur Typen mit Einheitendaten (tb::EINHEIT) -- UnitInfo fremder Nummern stuerzt im Original ab
    for e in super::tb::EINHEIT.iter() {
        let t = e.typ;
        let v = [welt.GetHitPoints(t, pl), welt.GetArmor(t, pl), welt.GetOreCost(t, pl), welt.GetRareOreCost(t, pl), welt.GetWorkersRequired(t, pl),
            welt.GetScientistsRequired(t, pl), welt.GetPowerRequired(t, pl), welt.GetWeaponRange(t, pl), welt.GetProductionCapacity(t, pl),
            welt.GetReloadTime(t, pl), welt.GetPenetrationDamage(t, pl), welt.GetConcussionDamage(t, pl), welt.GetBuildTime(t, pl)];
        if v.iter().all(|&x| x == 0) { continue; }
        if !erst { p.f(","); } erst = false;
        p.f(&format!("\"{}\":{:?}", t, v));
    }
    p.f(&format!("}},\"kolonie\":{:?}", [welt.Ore(pl), welt.RareOre(pl), welt.GetMaxOre(pl), welt.GetMaxRareOre(pl), welt.Workers(pl), welt.Scientists(pl), welt.Kids(pl),
        welt.GetNumAvailableWorkers(pl), welt.GetNumAvailableScientists(pl), welt.GetAmountPowerGenerated(pl), welt.GetAmountPowerConsumed(pl),
        welt.GetAmountPowerAvailable(pl), welt.GetNumIdleBuildings(pl), welt.GetNumActiveBuildings(pl), welt.GetNumUnpoweredStructures(pl),
        welt.GetNumWorkersRequired(pl), welt.GetNumScientistsRequired(pl), welt.GetNumScientistsAssignedToResearch(pl), welt.GetNetFoodProduction(pl),
        welt.GetTotalResidenceCapacity(pl), welt.GetTotalMedCenterCapacity(pl), welt.GetTotalRecreationFacilityCapacity(pl), welt.GetTotalForumCapacity(pl),
        welt.MoraleLevel(pl), welt.MoraleRoh(pl), welt.FoodStored(pl)]));
    p.f(",\"lage\":"); p.str(&format!("{:?}", l));
    p.f(&format!(",\"zustand\":[{},{},{},{}]", z.bedrohung, z.sollArmee, z.eigeneArmee, z.feindArmee));
    p.f(",\"einheiten\":[");
    let mut erst = true;
    for q in 0..welt.NoPlayers() {
        for u in welt.PlayerUnitEnum(q) {
            let lo = welt.Location(u);
            if !erst { p.f(","); } erst = false;
            p.f(&format!("[{},{},{},{},{},{},{}]", q, welt.GetType(u), edX(welt, z, lo), edY(welt, z, lo), welt.GetCurAction(u), welt.GetDamage(u), welt.GetCargo(u)));
        }
    }
    p.f("],\"techs\":[");
    for i in 0..welt.GetTechCount() {
        let ti = welt.GetTechInfo(i);
        if i > 0 { p.f(","); }
        let req: Vec<i32> = (0..ti.required.len()).map(|r| welt.GetTechInfo(ti.required[r]).id).collect();
        p.f(&format!("[{},{},{},{},{},{},{},{:?},{}]", ti.id, ti.valid as i32, ti.category, ti.eden_cost, ti.plymouth_cost, ti.lab, ti.max_scientists, req,
            welt.GetPlayerHasTech(i, pl) as i32));
    }
    p.f("],\"vorh\":[");
    for b in 0..=2i32 {
        let mut v = [Vorhersage::default(); 6];
        let n = super::forschung::bereichZiele(welt, z, l, b, &mut v, 6);
        if b > 0 { p.f(","); }
        p.f(&format!("{:?}", (0..n as usize).map(|i| (v[i].id, (v[i].nutzen * 10.0).round() / 10.0, v[i].kosten)).collect::<Vec<_>>()).replace('(', "[").replace(')', "]"));
    }
    let mut tr = [Trupp::default(); 16];
    let nt = super::forschung::truppen(welt, z, &mut tr, 16);
    p.f(&format!("],\"truppen\":{:?}", (0..nt as usize).map(|i| [tr[i].traeger, tr[i].waffe, tr[i].anzahl]).collect::<Vec<_>>()));
    p.f("}");
    p.schreiben(welt);
}

/// Rust (PAR zugang): Kolonie-Raum (eigene Gebaeudeflaechen, Lastwagenwege Mine -> Schmelze) und feindliche Gebaeudeflaechen
/// in Editor-Kacheln.
pub fn kolonieRaum(welt: &mut dyn Welt, z: &Zustand) -> (Vec<(i32, i32)>, Vec<(i32, i32)>) {
    let mut eigen: Vec<(i32, i32)> = Vec::new();
    let mut feind: Vec<(i32, i32)> = Vec::new();
    for p in 0..welt.NoPlayers() {
        let fremd = p != z.spieler && !welt.IsAlliedTo(z.spieler, p);
        if p != z.spieler && !fremd { continue; }
        for &typ in GEBAEUDE.iter() {
            for u in welt.PlayerBuildingEnum(p, typ) {
                if !welt.IsLive(u) { continue; }
                let lo = welt.Location(u);
                let (ex, ey) = (edX(welt, z, lo), edY(welt, z, lo));
                let r = rechteckUm(welt, typ, ex, ey);
                for y in r.y0..r.y0 + r.h { for x in r.x0..r.x0 + r.w { if fremd { feind.push((x, y)); } else { eigen.push((x, y)); } } }
            }
        }
    }
    // Lastwagenwege: gerade Linie Mine -> Schmelze (die Lastwagen fahren ungefaehr so)
    for i in 0..MAX_MINEN {
        let (m, s) = (z.minenId[i], z.schmelzeId[i]);
        if m == 0 || s == 0 || !welt.IsLive(m) || !welt.IsLive(s) { continue; }
        let (lm, ls) = (welt.Location(m), welt.Location(s));
        let (x0, y0, x1, y1) = (edX(welt, z, lm), edY(welt, z, lm), edX(welt, z, ls), edY(welt, z, ls));
        let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
        for k in 0..=n { eigen.push((x0 + (x1 - x0) * k / n, y0 + (y1 - y0) * k / n)); }
    }
    (eigen, feind)
}

/// Rust (PAR zugang, Nutzer 2026-11-17): die Wache auf die Zugaenge verteilen. Die Hauptwache steht hinter dem Zugang, den
/// der Feind am schnellsten erreicht (wachePunkt); die uebrigen Paesse (bis 3) bekommen eigene Gruppen, Anteil nach Breite und
/// Feindnaehe. Ist der Feind an einem Pass deutlich staerker, zieht die Gruppe fuer 1500 Ticks zur Basis zurueck.
pub fn zugangWachen(welt: &mut dyn Welt, z: &mut Zustand, basis: Rect) {
    let Some(e) = engstelle(welt, z) else { return };
    let sp = (z.spieler & 7) as usize;
    let Some((df, b)) = GELAENDE.with(|g| g.borrow()[sp].as_ref().and_then(|x| x.g.as_ref().map(|g| (g.d_feind.clone(), g.b)))) else { return };
    let dfeind = |x: i32, y: i32| { let v = df[(y * b + x) as usize]; if v < 0 { 999 } else { v } };
    let mut paesse: Vec<&super::gelaende::Pass> = e.paesse.iter().collect();
    paesse.sort_by_key(|p| dfeind(p.mitte.0, p.mitte.1));
    let gewicht = |p: &super::gelaende::Pass| p.kacheln.len() as f64 / (dfeind(p.mitte.0, p.mitte.1) as f64 + 10.0);
    let summe: f64 = paesse.iter().map(|p| gewicht(p)).sum();
    if summe <= 0.0 { return; }
    // Kaempfer in Hauptwache und Zugangsgruppen
    let mut gesamt = 0;
    for v in welt.GroupEnumerator(z.wache) { if welt.IsLive(v) && kaempfer(welt, v) { gesamt += 1; } }
    // die Eingreifreserve wartet ohnehin daheim: sie gehoert mit in den Topf fuer die Zugaenge
    if z.reserveAn != 0 { for v in welt.GroupEnumerator(z.eingreif) { if welt.IsLive(v) && kaempfer(welt, v) { gesamt += 1; } } }
    for j in 1..4 { if z.zugAn[j] != 0 { for v in welt.GroupEnumerator(z.zugGruppe[j]) { if welt.IsLive(v) && kaempfer(welt, v) { gesamt += 1; } } } }
    let t = welt.Tick();
    let hinter = PAR(z.spieler).engHinter + 3.0;
    if t % 5000 < TAKT {
        let anz: Vec<i32> = (1..4).map(|j| if z.zugAn[j] != 0 { welt.GroupEnumerator(z.zugGruppe[j]).into_iter().filter(|&v| welt.IsLive(v)).count() as i32 } else { 0 }).collect();
        let soll: Vec<i32> = (1..4).map(|j| paesse.get(j).map_or(0, |p| (gesamt as f64 * gewicht(p) / summe).floor() as i32)).collect();
        melde(welt, z, "kampf", mapLynx, z.bx, z.by, &format!("Zugaenge: {} Paesse, Wache+Reserve {} Fahrzeuge, Zugangsgruppen {:?} (Soll {:?})", paesse.len(), gesamt, anz, soll));
    }
    for j in 1..4usize {
        let pass = paesse.get(j);
        if z.zugAn[j] == 0 { if pass.is_none() { continue; } z.zugGruppe[j] = welt.CreateFightGroup(z.spieler); z.zugAn[j] = 1; z.zugX[j] = -1; }
        let g = z.zugGruppe[j];
        let soll = pass.map_or(0, |p| (gesamt as f64 * gewicht(p) / summe).floor() as i32);
        let mitglieder: Vec<Id> = welt.GroupEnumerator(g).into_iter().filter(|&v| welt.IsLive(v) && kaempfer(welt, v)).collect();
        let mut n = mitglieder.len() as i32;
        // zu viele: zurueck in die Hauptwache
        for &v in mitglieder.iter() { if n <= soll { break; } welt.RemoveUnit(g, v); welt.TakeUnit(z.wache, v); n -= 1; }
        // zu wenige: aus der Hauptwache
        if n < soll {
            for quelle in [z.eingreif, z.wache] {
                if quelle == z.eingreif && z.reserveAn == 0 { continue; }
                let frei: Vec<Id> = welt.GroupEnumerator(quelle).into_iter().filter(|&v| welt.IsLive(v) && kaempfer(welt, v) && !welt.IsBusy(v)).collect();
                for v in frei { if n >= soll { break; } welt.RemoveUnit(quelle, v); welt.TakeUnit(g, v); n += 1; }
            }
        }
        let Some(p) = pass else { continue };
        if n == 0 { continue; }
        // Rueckzug: Feind am Pass deutlich staerker als die Gruppe
        let mut eigen = 0; for v in welt.GroupEnumerator(g) { if welt.IsLive(v) { eigen += kampfWert(welt, v); } }
        let mut feind = 0;
        for q in 0..welt.NoPlayers() {
            if q == z.spieler || welt.IsAlliedTo(z.spieler, q) { continue; }
            for u in welt.PlayerVehicleEnum(q) {
                if !welt.IsLive(u) { continue; }
                let lo = welt.Location(u);
                let (ux, uy) = (edX(welt, z, lo), edY(welt, z, lo));
                if abstand2(welt, ux, uy, p.mitte.0, p.mitte.1) <= 10 * 10 { feind += kampfWert(welt, u); }
            }
        }
        if feind as f64 > eigen as f64 * 1.2 && t >= z.zugZurueck[j] {
            z.zugZurueck[j] = t + 1500; z.zugX[j] = -2;
            let warte = Rect::new(engine(welt, z, z.bx - 3, z.by - 3), engine(welt, z, z.bx + 3, z.by + 3));
            super::kampf::wachen(welt, g, warte, &[basis], 1);
            melde(welt, z, "kampf", mapLynx, p.mitte.0, p.mitte.1, &format!("Zugang {},{}: Feind zu stark ({} gegen {}) -- {} Fahrzeuge ziehen zur Basis", p.mitte.0, p.mitte.1, feind, eigen, n));
            continue;
        }
        if t < z.zugZurueck[j] { continue; }
        if z.zugX[j] == p.mitte.0 && z.zugY[j] == p.mitte.1 { continue; }
        z.zugX[j] = p.mitte.0; z.zugY[j] = p.mitte.1;
        let wx = (p.mitte.0 as f64 - p.zum_feind.0 * hinter + 0.5) as i32;
        let wy = (p.mitte.1 as f64 - p.zum_feind.1 * hinter + 0.5) as i32;
        let (wx, wy) = (wx.clamp(3, z.breite - 4), wy.clamp(3, z.hoehe - 4));
        let warte = Rect::new(engine(welt, z, wx - 2, wy - 2), engine(welt, z, wx + 2, wy + 2));
        let schutz = Rect::new(engine(welt, z, p.mitte.0 - 5, p.mitte.1 - 5), engine(welt, z, p.mitte.0 + 5, p.mitte.1 + 5));
        super::kampf::wachen(welt, g, warte, &[schutz], 1);
        melde(welt, z, "kampf", mapLynx, wx, wy, &format!("Zugang {},{} ({} Kacheln breit): {} von {} Wachfahrzeugen", p.mitte.0, p.mitte.1, p.kacheln.len(), n, gesamt));
    }
}
