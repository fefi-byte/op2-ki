//! Missionsmodus (Nutzer 2026-11-22, DOKU/missionsmodus.md): neben dem Mehrspieler-Modus (alles wie bisher) gibt die Mission
//! Schwierigkeit, Rolle und eigene Angriffe vor. Die Mission setzt sie ueber Befehle (spaeter Bruecke/Codegen-Editor), Tests
//! ueber die Parameter missionsModus, stufe, rolle, eigeneAngriffe. `anwenden` rechnet Stufe und Rolle auf den geladenen
//! Parametersatz (die Basis bleibt erhalten, ein Wechsel rechnet neu). Schwierigkeit ohne Boni: nur Verhalten.
use super::*;
use std::cell::RefCell;

pub const LEICHT: i32 = 0;
pub const MITTEL: i32 = 1;
pub const SCHWER: i32 = 2;

pub const ROLLE_KEINE: i32 = 0;
pub const ROLLE_AGGRESSIV: i32 = 1;
pub const ROLLE_DEFENSIV: i32 = 2;
pub const ROLLE_KOLONIE: i32 = 3;

/// Angriffsauftrag der Mission (Trigger "starte Angriff"): Ziel Feindbasis (ort None) oder ein Ort, Staerke als Faktor
#[derive(Clone, Copy, Default)]
pub struct Auftrag { pub ort: Option<(i32, i32)>, pub staerke: f64, pub seit: i32 }

/// Aussenposten-Auftrag der Mission: Ort, Groesse (1 Mine, 2 gesichert, 3 Basis), Baken (leer = die im Umkreis von 8 Kacheln)
#[derive(Clone, Debug, Default)]
pub struct Posten { pub x: i32, pub y: i32, pub groesse: i32, pub baken: Vec<(i32, i32)> }

/// Befehle der Mission (None = nicht gesetzt, dann gilt der Parameter)
#[derive(Clone, Copy, Default)]
pub struct Befehle { pub mission: Option<bool>, pub stufe: Option<i32>, pub rolle: Option<i32>, pub angriffe: Option<bool> }

thread_local! {
    static BEFEHL: RefCell<[Befehle; 8]> = RefCell::new(Default::default());
    static AUFTRAG: RefCell<[Option<Auftrag>; 8]> = RefCell::new(Default::default());
    static TEST_GESETZT: RefCell<[bool; 8]> = RefCell::new(Default::default());
    /// Kampfwert, den der offene Auftrag braucht (die Sollzahlen der Sammelgruppe richten sich danach)
    static NOETIG: RefCell<[i32; 8]> = RefCell::new(Default::default());
    /// noch nicht begonnene Aussenposten-Auftraege
    static POSTEN: RefCell<[Vec<Posten>; 8]> = RefCell::new(Default::default());
    static TEST_POSTEN: RefCell<[bool; 8]> = RefCell::new(Default::default());
    /// letzter Tick, an dem Feinde die Kolonie bedrohten (Gegenstoss der Rolle defensiv)
    static DROHUNG: RefCell<[(i32, i32); 8]> = RefCell::new(Default::default());   // (Tick, Hoechstwert) der letzten Bedrohung
    /// Welle (Tick), fuer die der Gegenstoss schon lief
    static GENUTZT: RefCell<[i32; 8]> = RefCell::new(Default::default());
    /// geladener Parametersatz vor Stufe und Rolle
    static BASIS: RefCell<[Option<crate::cpp::zustand::Parameter>; 8]> = RefCell::new(Default::default());
}

fn sp(z: &Zustand) -> usize { (z.spieler & 7) as usize }

/// Neues Spiel: Befehle vergessen
pub fn zuruecksetzen(z: &Zustand) {
    KIGA.with(|k| k.borrow_mut()[sp(z)] = None);
    BEFEHL.with(|b| b.borrow_mut()[sp(z)] = Befehle::default()); BASIS.with(|b| b.borrow_mut()[sp(z)] = None);
    AUFTRAG.with(|a| a.borrow_mut()[sp(z)] = None); TEST_GESETZT.with(|g| g.borrow_mut()[sp(z)] = false);
    POSTEN.with(|p| p.borrow_mut()[sp(z)].clear()); TEST_POSTEN.with(|g| g.borrow_mut()[sp(z)] = false);
}

/// Trigger der Mission: Angriff mit dieser Staerke (Faktor auf die verlangte Ueberlegenheit der Stufe) auf die Feindbasis
/// (ort None) oder das Feindgebaeude naechst dem Ort. Die KI baut die Truppe, sammelt sie und schickt sie los.
pub fn setzeAngriff(welt: &mut dyn Welt, z: &mut Zustand, ort: Option<(i32, i32)>, staerke: f64) {
    let t = welt.Tick();
    AUFTRAG.with(|a| a.borrow_mut()[sp(z)] = Some(Auftrag { ort, staerke: staerke.max(0.1), seit: t }));
    let wo = match ort { Some((x, y)) => format!("Ziel bei {},{}", x, y), None => "Ziel Feindbasis".to_string() };
    melde(welt, z, "modus", mapNone, z.bx, z.by, &format!("Mission: Angriff befohlen ({}, Staerke {:.1}) -- Truppe wird gebaut", wo, staerke));
}

/// Offener Angriffsauftrag (PAR testAngriff/testStaerke loest im Simulator einmal einen aus)
pub fn auftrag(welt: &mut dyn Welt, z: &mut Zustand) -> Option<Auftrag> {
    let i = sp(z);
    let ta = PAR(z.spieler).testAngriff;
    if ta > 0.0 && welt.Tick() as f64 >= ta && !TEST_GESETZT.with(|g| g.borrow()[i]) {
        TEST_GESETZT.with(|g| g.borrow_mut()[i] = true);
        let st = PAR(z.spieler).testStaerke;
        setzeAngriff(welt, z, None, if st > 0.0 { st } else { 1.0 });
    }
    AUFTRAG.with(|a| a.borrow()[i])
}

/// Trigger der Mission: Aussenposten (zweite Basis) an diesem Ort, mit dieser Groesse und optional diesen Baken
pub fn setzeAussenposten(welt: &mut dyn Welt, z: &mut Zustand, x: i32, y: i32, groesse: i32, baken: &[(i32, i32)]) {
    POSTEN.with(|p| p.borrow_mut()[sp(z)].push(Posten { x, y, groesse: groesse.clamp(1, 4), baken: baken.to_vec() }));
    melde(welt, z, "modus", mapCommandCenter, x, y, &format!("Mission: Aussenposten bei {},{} befohlen (Groesse {}, {} Baken vorgegeben)", x, y, groesse.clamp(1, 4), baken.len()));
}

/// Naechster offener Aussenposten-Auftrag (wird damit entnommen); PAR testPosten loest im Simulator einmal einen aus
pub fn postenNehmen(welt: &mut dyn Welt, z: &mut Zustand) -> Option<Posten> {
    let i = sp(z); let pm = PAR(z.spieler);
    if pm.testPosten > 0.0 && welt.Tick() as f64 >= pm.testPosten && !TEST_POSTEN.with(|g| g.borrow()[i]) {
        TEST_POSTEN.with(|g| g.borrow_mut()[i] = true);
        let baken: Vec<(i32, i32)> = if pm.testBakeX > 0.0 { vec![(pm.testBakeX as i32, pm.testBakeY as i32)] } else { Vec::new() };
        setzeAussenposten(welt, z, pm.testPostenX as i32, pm.testPostenY as i32, pm.testPostenGroesse as i32, &baken);
    }
    POSTEN.with(|p| { let mut p = p.borrow_mut(); if p[i].is_empty() { None } else { Some(p[i].remove(0)) } })
}

pub fn auftragErledigt(z: &Zustand) { AUFTRAG.with(|a| a.borrow_mut()[sp(z)] = None); NOETIG.with(|n| n.borrow_mut()[sp(z)] = 0); }
pub fn setzeNoetig(z: &Zustand, kw: i32) { NOETIG.with(|n| n.borrow_mut()[sp(z)] = kw); }
/// Kampfwert fuer den offenen Auftrag (0 = keiner)
pub fn noetig(z: &Zustand) -> i32 { NOETIG.with(|n| n.borrow()[sp(z)]) }

pub fn mission(z: &Zustand) -> bool {
    BEFEHL.with(|b| b.borrow()[sp(z)].mission).unwrap_or_else(|| basis(z).missionsModus >= 0.5)
}

fn basis(z: &Zustand) -> crate::cpp::zustand::Parameter { BASIS.with(|b| b.borrow()[sp(z)]).unwrap_or_else(|| PAR(z.spieler)) }

/// Wirksame Stufe: Befehl, sonst Parameter stufe (0-2); im Missionsmodus ohne beides die Schwierigkeit des Spiels;
/// im Mehrspieler-Modus ohne beides schwer (= die bisherige KI)
pub fn stufe(welt: &mut dyn Welt, z: &Zustand) -> i32 {
    if let Some(s) = BEFEHL.with(|b| b.borrow()[sp(z)].stufe) { return s.clamp(0, 2); }
    let p = basis(z).stufe;
    if p >= 0.0 { return (p as i32).clamp(0, 2); }
    if mission(z) { let s = welt.Schwierigkeit(z.spieler); if s >= 0 { return s.clamp(0, 2); } }
    SCHWER
}

pub fn rolle(z: &Zustand) -> i32 { BEFEHL.with(|b| b.borrow()[sp(z)].rolle).unwrap_or(basis(z).rolle as i32) }

/// Je Kampftakt: Bedrohung der Kolonie merken (fuer den Gegenstoss)
pub fn drohungMerken(z: &Zustand, t: i32) {
    DROHUNG.with(|d| {
        let mut d = d.borrow_mut(); let e = &mut d[sp(z)];
        // neue Welle (alte laenger als 6000 Ticks her) oder hoeherer Wert: Hoechstwert und Zeit merken
        if z.feindDrohung > 0 && (t - e.0 > 6000 || z.feindDrohung >= e.1) { *e = (t, z.feindDrohung); }
    });
}

/// Gegenstoss (Rolle defensiv): ein Angriff auf die Kolonie ist abgewehrt (keine Bedrohung mehr) und liegt hoechstens
/// 6000 Ticks zurueck -- dann darf die KI selbst angreifen (mit den normalen Schwellen)
pub fn gegenstoss(z: &Zustand, t: i32) -> bool {
    // abgewehrt: die Bedrohung ist auf hoechstens ein Viertel ihres Hoechstwerts gefallen (ganz 0 wird sie selten)
    let (d, spitze) = DROHUNG.with(|d| d.borrow()[sp(z)]);
    d > 0 && spitze > 0 && z.feindDrohung * 4 <= spitze && t - d <= 6000
}

/// Rolle defensiv: Gegenstoss faellig (abgewehrte Welle, fuer die noch keiner lief)?
pub fn gegenstossFaellig(z: &Zustand, t: i32) -> bool {
    rolle(z) == ROLLE_DEFENSIV && gegenstoss(z, t) && GENUTZT.with(|g| g.borrow()[sp(z)]) != DROHUNG.with(|d| d.borrow()[sp(z)].0)
}
pub fn gegenstossGenutzt(z: &Zustand) { let w = DROHUNG.with(|d| d.borrow()[sp(z)].0); GENUTZT.with(|g| g.borrow_mut()[sp(z)] = w); }

/// Gilt die Kampffuehrung in diesem Durchgang? Stufe mittel jeder zweite, leicht jeder dritte (langsamere Reaktion).
pub fn kampfTakt(welt: &mut dyn Welt, z: &Zustand, t: i32, scan: i32) -> bool {
    match stufe(welt, z) { LEICHT => (t / scan) % 3 == 0, MITTEL => (t / scan) % 2 == 0, _ => true }
}

/// Darf die KI jetzt selbst einen Angriff beginnen? (Befehl/Parameter eigeneAngriffe, Rolle, Schonfrist der Stufe)
pub fn eigeneAngriffe(z: &Zustand, t: i32) -> bool {
    let an = BEFEHL.with(|b| b.borrow()[sp(z)].angriffe).unwrap_or_else(|| {
        let r = rolle(z);
        basis(z).eigeneAngriffe >= 0.5 && r != ROLLE_KOLONIE && (r != ROLLE_DEFENSIV || gegenstoss(z, t))
    });
    an && (t as f64) >= PAR(z.spieler).schonfrist
}

pub fn setzeMission(welt: &mut dyn Welt, z: &mut Zustand, an: bool) { BEFEHL.with(|b| b.borrow_mut()[sp(z)].mission = Some(an)); anwenden(welt, z); }
pub fn setzeStufe(welt: &mut dyn Welt, z: &mut Zustand, s: i32) { BEFEHL.with(|b| b.borrow_mut()[sp(z)].stufe = Some(s)); anwenden(welt, z); }
pub fn setzeRolle(welt: &mut dyn Welt, z: &mut Zustand, r: i32) { BEFEHL.with(|b| b.borrow_mut()[sp(z)].rolle = Some(r)); anwenden(welt, z); }
pub fn setzeAngriffe(welt: &mut dyn Welt, z: &mut Zustand, an: bool) {
    BEFEHL.with(|b| b.borrow_mut()[sp(z)].angriffe = Some(an));
    melde(welt, z, "modus", mapNone, z.bx, z.by, &format!("Mission: eigene Angriffe {}", if an { "an" } else { "aus" }));
}

/// Befehl der Mission (befehl.rs): eine Stellschraube waehrend des Spiels aendern -- im geladenen Satz (vor Stufe und
/// Rolle), dann neu anwenden. false = Name unbekannt.
pub fn parameterAendern(welt: &mut dyn Welt, z: &mut Zustand, name: &str, wert: f64) -> bool {
    let i = sp(z);
    let mut q = basis(z);
    let ok = match crate::cpp::ablauf::feld(&mut q, name) { Some(f) => { *f = wert; true } None => false };
    if ok { BASIS.with(|b| b.borrow_mut()[i] = Some(q)); anwenden(welt, z); }
    ok
}

thread_local! {
    /// Befehl der Mission: Kindergarten-Schwellen (aus ueber, wieder an unter -- freie Arbeiter)
    static KIGA: RefCell<[Option<(i32, i32)>; 8]> = RefCell::new(Default::default());
}

/// Mission (Codegen-Editor, Nutzer 2026-11-22: "Die Schwelle soll anpassbar sein im Game Editor"): Kindergarten aus, sobald
/// mehr als `aus` Arbeiter frei sind, wieder an unter `an`
pub fn setzeKigaSchwellen(welt: &mut dyn Welt, z: &mut Zustand, aus: i32, an: i32) {
    KIGA.with(|k| k.borrow_mut()[sp(z)] = Some((aus, an.min(aus))));
    melde(welt, z, "modus", mapNursery, z.bx, z.by, &format!("Mission: Kindergarten aus ueber {} freien Arbeitern, wieder an unter {}", aus, an.min(aus)));
}

/// Wirksame Kindergarten-Schwellen: Befehl der Mission, sonst die Parameter kigaAus / kigaAn
pub fn kigaSchwellen(z: &Zustand) -> (i32, i32) {
    KIGA.with(|k| k.borrow()[sp(z)]).unwrap_or_else(|| { let p = PAR(z.spieler); (p.kigaAus as i32, p.kigaAn as i32) })
}

/// Stufe und Rolle auf den geladenen Parametersatz rechnen (nach parameterLaden und bei jedem Wechsel)
pub fn anwenden(welt: &mut dyn Welt, z: &mut Zustand) {
    let i = sp(z);
    if BASIS.with(|b| b.borrow()[i].is_none()) { let p = PAR(z.spieler); BASIS.with(|b| b.borrow_mut()[i] = Some(p)); }
    let mut q = basis(z);
    let s = stufe(welt, z);
    let r = rolle(z);
    // Schwierigkeit: schwer = die bisherige KI. Leichter heisst: mehr Ueberlegenheit verlangt, kleinere Wellen, spaeter der
    // erste Angriff, einfachere Gefechtsfuehrung, weniger Ausbau der Fabriken -- nie Boni.
    match s {
        LEICHT => {
            q.ueberlegenheit *= 1.6; q.welle = if q.welle >= 1.0 { q.welle.min(8.0) } else { 8.0 };
            q.schonfrist = q.schonfrist.max(40000.0); q.gefecht = 0.0; q.fabrikErz = 0.0; q.zermuerbAbstand *= 2.0;
        }
        MITTEL => {
            q.ueberlegenheit *= 1.3; q.welle = if q.welle >= 1.0 { q.welle.min(16.0) } else { 16.0 };
            q.schonfrist = q.schonfrist.max(20000.0); if q.gefecht >= 1.5 { q.gefecht = 1.0; } q.zermuerbAbstand *= 1.5;
        }
        _ => {}
    }
    // Rollen (ausserhalb des Kampfes)
    match r {
        ROLLE_AGGRESSIV => {
            q.ueberlegenheit *= 0.85; q.ueberlegMin = q.ueberlegMin.min(0.85); q.zermuerbAbstand *= 0.5; q.turmAnteil *= 0.6;
            q.angriffMarge = q.angriffMarge.min(-0.05);   // Zermuerben auch bei leicht negativer Bilanz
        }
        // defensiv: Tuerme an den Paessen und Wache am Pass; der Turmanteil bleibt, damit fuer den Gegenstoss Fahrzeuge da sind
        ROLLE_DEFENSIV => { if q.gelaende <= 0.0 { q.gelaende = 1.0; } q.engTuerme = q.engTuerme.max(4.0); }
        ROLLE_KOLONIE => {
            // Ausbau: Zweitbasen frueher und bis zu zwei (halbe Armee genuegt, kein Erzueberschuss noetig, daheim duerfen noch
            // zwei Plaetze offen sein); ein zweiter Vermesser fuer die Baken (erz/fahrzeuge.rs)
            q.zweitbasis = q.zweitbasis.max(1.0); q.zbArmee = q.zbArmee.min(0.5); q.zbAnzahl = q.zbAnzahl.max(2.0);
            q.zbUeberschuss = 0.0; q.zbOffen = q.zbOffen.max(2.0); q.zbErz *= 0.5;
        }
        _ => {}
    }
    // Missionsmodus (Nutzer 2026-11-22): Minen nur nahe einer Basis (35 Kacheln), eine je Sorte; Mauern an den Zugaengen
    // aus dem Gelaende (Fels zu Fels, mit Tor) statt Stuecken an jedem Turm
    if mission(z) {
        if q.minenUmkreis == 0.0 { q.minenUmkreis = 35.0; }
        if q.minenJeSorte == 0.0 { q.minenJeSorte = 1.0; }
        if q.zugangMauer == 0.0 { q.zugangMauer = 1.0; }
        if q.wacheVorBasis == 0.0 { q.wacheVorBasis = 1.0; }
        if q.wachePatrouille == 0.0 { q.wachePatrouille = 1.0; }
        // Bezirke statt des dichten Bebauungsplans
        if q.bezirke == 0.0 { q.bezirke = 1.0; }
        if q.bezirke >= 0.5 { q.vorplan = 0.0; }
        if q.gebietAuto == 0.0 { q.gebietAuto = 1.0; }
        // kernLuft (luftiger Kern) bleibt aus: im Original blieben einzelne seiner Roehrenkacheln dauerhaft unbebaut, dahinter
        // stand jeder Ausbau still (cm03 ab ~195 000; im Simulator nicht nachzustellen). Mit kernLuft=1 einschaltbar.
        if q.roehrenVorrang == 0.0 { q.roehrenVorrang = 1.0; }
        if q.roehreAnNetz == 0.0 { q.roehreAnNetz = 1.0; }
        if q.kigaBremse == 0.0 { q.kigaBremse = 1.0; }
        if q.raumhafen == 0.0 { q.raumhafen = 1.0; }
        if q.spinnen == 0.0 { q.spinnen = 1.0; }
        if q.aufstieg == 0.0 { q.aufstieg = 1.0; }
        if q.mhd == 0.0 { q.mhd = 1.0; }
        if q.solar == 0.0 { q.solar = 1.0; }
    }
    crate::cpp::globale::par.with(|p| p.borrow_mut()[i] = q);
    // Mehrspieler ohne Vorgaben: keine Meldung (die Spur bleibt gleich wie vor dem Missionsmodus)
    if !mission(z) && s == SCHWER && r == ROLLE_KEINE && BEFEHL.with(|b| b.borrow()[i].angriffe).is_none() && q.eigeneAngriffe >= 0.5 && q.schonfrist <= 0.0 { return; }
    let sn = ["leicht", "mittel", "schwer"][s as usize];
    let rn = ["keine", "aggressiv", "defensiv", "Kolonie"][r.clamp(0, 3) as usize];
    melde(welt, z, "modus", mapNone, z.bx, z.by, &format!("{}: Stufe {}, Rolle {}, eigene Angriffe {}",
          if mission(z) { "Missionsmodus" } else { "Mehrspieler" }, sn, rn, if eigeneAngriffe(z, i32::MAX) { "an" } else { "aus" }));
}
