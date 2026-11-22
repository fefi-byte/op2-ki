//! Datenstrukturen der C++-KI (op2_ki.hpp), Feld fuer Feld uebertragen. Erzeugt von struktur.py, danach von Hand ergaenzt.
#![allow(non_snake_case, non_camel_case_types, dead_code)]
use super::welt::{Gruppe, Id, Loc};

mod parameter;
mod spieler;
pub use parameter::*;
pub use spieler::*;

pub const TAKT: i32 = 20;
pub const SPUR_TAKT: i32 = 200;
pub const MAX_PLAETZE: usize = 48;
pub const MAX_ROEHREN: usize = 48;
pub const MAX_MINEN: usize = 4;
pub const MAX_VF: usize = 4;
pub const GEDULD_BAU: i32 = 1500;
pub const GEDULD_KIT: i32 = 2500;
pub const MAX_ANSPR: usize = 40;
pub const MAX_VORHABEN: usize = 4;
pub const P_FREI: i16 = 0;
pub const P_FAHRT: i16 = 1;
pub const P_BAUT: i16 = 2;
pub const P_STEHT: i16 = 3;
pub const P_GESPERRT: i16 = 9;
pub const M_ARB: usize = 0;
pub const M_WISS: usize = 1;
pub const M_ERZ: usize = 2;
pub const M_SELTEN: usize = 3;

/// Lagebild eines Schritts (C++ struct Lage).
#[derive(Clone, Debug)]
pub struct Lage {
    pub geb: [i32; 128],   // Anzahl fertiger Gebaeude je map_id (map_id < 128)
    pub fz: [i32; 128],   // Anzahl Fahrzeuge je map_id
    pub kitsImSpiel: [i32; 128],   // Bausaetze in Fabrikbuchten und auf ConVecs
    pub imBau: [i32; 128],   // Gebaeude im Bau (Aktion moObjWait) -- wirken erst, wenn sie fertig sind
    pub cc: Id,   // erste Kommandozentrale / Strukturfabrik / Fahrzeugfabrik
    pub sf: Id,   // erste Kommandozentrale / Strukturfabrik / Fahrzeugfabrik
    pub vf: Id,   // erste Kommandozentrale / Strukturfabrik / Fahrzeugfabrik
    pub vfs: [Id; 4],   // alle Fahrzeugfabriken (vfs[0] == vf)
    pub vfAnzahl: i32,
    pub vfLaeuft: i32,
    pub erz: i32,
    pub erzMax: i32,
    pub nahrung: i32,
    pub nahrungNetto: i32,
    pub stromFrei: i32,
    pub menschen: i32,
    pub wohnraum: i32,
    pub wiss: i32,
    pub selten: i32,   // Seltenerz / rare ore
    pub seltenMax: i32,   // Seltenerz / rare ore
    pub arbFrei: i32,   // freie Arbeiter / Wissenschaftler
    pub wissFrei: i32,   // freie Arbeiter / Wissenschaftler
    pub arbStilllegbar: [i32; 11],   // Arbeiter in laufenden Gebaeuden mit Rang < 10*i (koennten stillgelegt werden)
    pub vorErz: i32,   // vom Ziel vorgemerkt (vormerkungen())
    pub vorSelten: i32,   // vom Ziel vorgemerkt (vormerkungen())
    pub vorFuer: &'static str,
    pub arbFehlt: i32,   // Arbeiter / Wissenschaftler, die fertigen (nicht stillgelegten) Gebaeuden fehlen
    pub wissFehlt: i32,   // Arbeiter / Wissenschaftler, die fertigen (nicht stillgelegten) Gebaeuden fehlen
    pub unbesetzt: [i32; 128],   // fertige Gebaeude je Typ ohne Arbeiter -- davon nicht noch eins bauen
    pub medKap: i32,   // Gesamtkapazitaeten (nur laufende Gebaeude, HFL)
    pub freizeitKap: i32,   // Gesamtkapazitaeten (nur laufende Gebaeude, HFL)
    pub forumKap: i32,   // Gesamtkapazitaeten (nur laufende Gebaeude, HFL)
    pub dirtAbdeckung: i32,   // % der angeschlossenen Gebaeude, die laufende DIRTs abdecken (10 je DIRT); GORF in Betrieb
    pub gorfLaeuft: i32,   // % der angeschlossenen Gebaeude, die laufende DIRTs abdecken (10 je DIRT); GORF in Betrieb
    pub angeschlossen: i32,   // % der angeschlossenen Gebaeude, die laufende DIRTs abdecken (10 je DIRT); GORF in Betrieb
    pub kitsFabrik: i32,   // fertige Bausaetze in der Strukturfabrik, die auf einen ConVec warten
    pub seltenSpart: bool,   // die erwartete Seltenerz-Schmelze sammelt Erz: neue Kampffahrzeuge warten (ohne akute Bedrohung)
    pub erzSpart: bool,   // Rust (kampfModell): eine lohnende Erzschmelze sammelt Erz -- neue Kampffahrzeuge warten (ohne akute Bedrohung)
    pub anspr: [Anspruch; 40],   // Ansprueche (arbeitErheben)
    pub nAnspr: i32,   // Ansprueche (arbeitErheben)
}
impl Default for Lage {
    fn default() -> Self {
        Lage {
            geb: [0; 128],
            fz: [0; 128],
            kitsImSpiel: [0; 128],
            imBau: [0; 128],
            cc: 0,
            sf: 0,
            vf: 0,
            vfs: [0; 4],
            vfAnzahl: 0,
            vfLaeuft: 0,
            erz: 0,
            erzMax: 0,
            nahrung: 0,
            nahrungNetto: 0,
            stromFrei: 0,
            menschen: 0,
            wohnraum: 0,
            wiss: 0,
            selten: 0,
            seltenMax: 0,
            arbFrei: 0,
            wissFrei: 0,
            arbStilllegbar: [0; 11],
            vorErz: 0,
            vorSelten: 0,
            vorFuer: "-",
            arbFehlt: 0,
            wissFehlt: 0,
            unbesetzt: [0; 128],
            medKap: 0,
            freizeitKap: 0,
            forumKap: 0,
            dirtAbdeckung: 0,
            gorfLaeuft: 0,
            angeschlossen: 0,
            kitsFabrik: 0,
            seltenSpart: false,
            erzSpart: false,
            anspr: [Anspruch { typ: 0, rang: 0, menge: [0; 4], vorhaben: false }; 40],
            nAnspr: 0,
        }
    }
}

// ---------------------------------------------------------------- weitere Strukturen der C++-KI
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Groesse { pub w: i32, pub h: i32 }

/// Rechteck (C++ struct Rechteck in op2_ki.hpp): Grundflaeche oben links, Groesse, Roehrenstutzen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rechteck { pub x0: i32, pub y0: i32, pub w: i32, pub h: i32, pub stutzen: bool }

#[derive(Clone, Copy, Debug, Default)]
pub struct Kraftwerte { pub hp: i32, pub panzer: i32, pub pen: i32, pub kon: i32, pub reload: i32 }

#[derive(Clone, Copy, Debug)]
pub struct Bilanz {
    pub bedarfErz: i32, pub bedarfSelten: i32,
    pub erzHaengtAnSelten: i32,
    pub engpassErz: i32, pub engpassSelten: i32,
    pub routeErz: i32, pub routeSelten: i32,
    pub treiber: &'static str,
}
impl Default for Bilanz {
    fn default() -> Self { Bilanz { bedarfErz: 0, bedarfSelten: 0, erzHaengtAnSelten: 0, engpassErz: 0, engpassSelten: 0, routeErz: 0, routeSelten: 0, treiber: "-" } }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ErzWahl { pub typ: i32, pub nutzen: f64 }

/// C++ struct Wahl { int typ; int bereich; char warum[200]; } -- warum als String.
#[derive(Clone, Debug, Default)]
pub struct Wahl { pub typ: i32, pub bereich: i32, pub warum: String }

#[derive(Clone, Copy, Debug)]
pub struct Dringlichkeit {
    pub kolonie: i32, pub ziel: i32, pub wirtschaft: i32, pub verteidigung: i32, pub kampfForschung: i32,
    pub kolonieGrund: &'static str, pub wirtschaftGrund: &'static str,
}
impl Default for Dringlichkeit {
    fn default() -> Self { Dringlichkeit { kolonie: 0, ziel: 0, wirtschaft: 0, verteidigung: 0, kampfForschung: 0, kolonieGrund: "-", wirtschaftGrund: "-" } }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BakenFund { pub beste: Id, pub gesehen: i32, pub gewoehnlich: i32, pub frei: i32, pub einheiten: i32, pub balken: i32, pub wert: f64, pub unvermessen: Id }

#[derive(Clone, Copy, Debug, Default)]
pub struct Trupp { pub traeger: i32, pub waffe: i32, pub anzahl: i32 }

#[derive(Clone, Copy, Debug, Default)]
pub struct Vorhersage { pub id: i32, pub nutzen: f64, pub kosten: i32 }

/// Spur-Puffer (C++ struct Puffer): JSON-Zeile zusammensetzen, `schreiben` gibt sie an die Spur.
#[derive(Clone, Debug, Default)]
pub struct Puffer { pub b: String }
impl Puffer {
    /// p.roh(s)
    pub fn roh(&mut self, s: &str) { self.b.push_str(s); }
    /// p.f(fmt, ...) -- Aufrufer formatiert mit format!
    pub fn f(&mut self, s: &str) { self.b.push_str(s); }
    /// Text als JSON-String (Anfuehrungszeichen und Backslash maskiert)
    pub fn str(&mut self, s: &str) {
        self.b.push('"');
        for c in s.chars() { if c == '"' || c == '\\' { self.b.push('\\'); } self.b.push(c); }
        self.b.push('"');
    }
    pub fn schreiben(&mut self, welt: &mut dyn super::welt::Welt) { welt.kiRaw(&self.b); self.b.clear(); }
}

// netz:: (kleines Netz aus Datei), wie in C++
pub const MERKMAL_VERSION: i32 = 1;
pub const MAX_EIN: usize = 96;
pub const MAX_BREIT: usize = 64;
pub const MAX_SCHICHT: usize = 4;
pub const MAX_AUS: usize = 4;
#[derive(Clone)]
pub struct Netz {
    pub geladen: i32, pub versucht: i32, pub ein: i32, pub aus: i32, pub schichten: i32,
    pub zeilen: [i32; MAX_SCHICHT], pub spalten: [i32; MAX_SCHICHT], pub tanh_: [i32; MAX_SCHICHT],
    pub mx: [f32; MAX_EIN], pub sx: [f32; MAX_EIN], pub my: [f32; MAX_AUS], pub sy: [f32; MAX_AUS],
    pub W: Vec<[f32; MAX_EIN * MAX_BREIT]>,
    pub b: [[f32; MAX_BREIT]; MAX_SCHICHT],
}
impl Default for Netz {
    fn default() -> Self {
        Netz { geladen: 0, versucht: 0, ein: 0, aus: 0, schichten: 0, zeilen: [0; MAX_SCHICHT], spalten: [0; MAX_SCHICHT], tanh_: [0; MAX_SCHICHT],
            mx: [0.0; MAX_EIN], sx: [0.0; MAX_EIN], my: [0.0; MAX_AUS], sy: [0.0; MAX_AUS], W: vec![[0.0; MAX_EIN * MAX_BREIT]; MAX_SCHICHT], b: [[0.0; MAX_BREIT]; MAX_SCHICHT] }
    }
}
