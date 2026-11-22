//! Zustand eines KI-Spielers (alles, was zwischen den Schritten bleibt), Platz, Roehre, Anspruch.
use super::*;
use super::super::*;

/// Bauplatz (C++ struct Platz).
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Platz {
    pub x0: i16,   // Grundflaeche oben links (Editor-Kacheln)
    pub y0: i16,   // Grundflaeche oben links (Editor-Kacheln)
    pub typ: i16,   // map_id
    pub zustand: i16,   // PlatzZustand
    pub convec: i32,   // unitID des ConVecs, der hier baut (0 = keiner)
    pub seit: i32,   // Tick der letzten Aenderung
    pub versuche: i16,
    pub raster: i16,   // frei (frueher Rasterplatz), immer -1
}

/// Gerades Roehrenstueck (C++ struct Roehre).
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Roehre {
    pub x1: i16,
    pub y1: i16,
    pub x2: i16,
    pub y2: i16,
    pub zustand: i16,   // 0 offen, 1 beauftragt, 2 fertig
    pub versuche: i16,
    pub seit: i32,
}

/// Anspruch auf knappe Mittel (C++ struct Anspruch).
#[derive(Clone, Copy, Debug, Default)]
pub struct Anspruch { pub typ: i32, pub rang: i32, pub menge: [i32; 4], pub vorhaben: bool }

/// Zustand einer KI (C++ struct Zustand), wird mit Nullen angelegt wie g_save im Original.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Zustand {
    pub aktiv: i32,
    pub spieler: i32,
    pub ux: i32,   // Engine-Koordinate der Editor-Kachel (0,0)
    pub uy: i32,   // Engine-Koordinate der Editor-Kachel (0,0)
    pub breite: i32,   // Kartengroesse in Kacheln
    pub hoehe: i32,   // Kartengroesse in Kacheln
    pub bx: i32,   // Basismitte (Editor-Kacheln)
    pub by: i32,   // Basismitte (Editor-Kacheln)
    pub naechsterSchritt: i32,
    pub naechsteSpur: i32,
    pub naechsterAnschluss: i32,
    pub forschungModus: i32,   // 0: DoResearch mit techID, 1: mit Tabellenindex (kalibriert sich selbst)
    pub forschungLab: i32,   // unitID des Labors, dessen Start geprueft wird
    pub forschungWert: i32,   // angefragtes Thema
    pub forschungSeit: i32,
    pub kitTyp: i32,   // Bausatz, der gerade in der Fabrik entsteht (0 = keiner)
    pub kitSeit: i32,
    pub kitGesperrtTyp: [i32; 8],   // Bausaetze, die die Fabrik (noch) nicht herstellt
    pub kitGesperrtBis: [i32; 8],
    pub kitPauseBis: i32,   // Fabrik hat den letzten Auftrag nicht angenommen -> kurz warten
    pub fzTyp: i32,   // Fahrzeug, das gerade in der Fahrzeugfabrik entsteht
    pub fzSeit: i32,
    pub fzPauseBis: i32,
    pub meldungenSeit: [i32; 32],   // Entprellung wiederkehrender Meldungen
    pub plaetze: [Platz; 48],
    pub anzahlPlaetze: i32,
    pub roehren: [Roehre; 48],
    pub anzahlRoehren: i32,
    pub minenId: [i32; 4],   // unitID der Mine je MiningGroup (0 = frei)
    pub schmelzeId: [i32; 4],
    pub minenGruppe: [Gruppe; 4],
    pub bakeZiel: i32,   // unitID der Bake, die als Naechstes erschlossen wird
    pub bakeMiner: i32,   // unitID des Miners, der gerade eine Mine baut
    pub bakeSeit: i32,   // seit wann diese Bake das Ziel ist
    pub minerSeit: i32,   // wann der Miner den Auftrag bekam
    /// Robo-Miner, der schon zur Bake faehrt, waehrend sie vermessen wird; Bake, zu der Lastwagen vorgefahren sind
    pub minerVor: i32,
    pub lkwVorBake: i32,   // Bake, zu der schon Lastwagen vorgefahren sind
    pub lkwVor: [i32; 2],   // die zur Mine vorgefahrenen Lastwagen (bis zur Zuteilung)
    pub bakenX: [i32; 4],   // selbst angelegte Baken (Editor-Kacheln)
    pub bakenY: [i32; 4],   // selbst angelegte Baken (Editor-Kacheln)
    pub anzahlBaken: i32,
    pub kbX: [i16; 32],   // alle bekannten Baken (Editor-Kacheln) -- die Aufzaehler liefern keine, platzFrei braucht sie
    pub kbY: [i16; 32],   // alle bekannten Baken (Editor-Kacheln) -- die Aufzaehler liefern keine, platzFrei braucht sie
    pub kbN: i32,   // Anzahl, Tick der letzten Erfassung
    pub kbStand: i32,   // Anzahl, Tick der letzten Erfassung
    pub sperrBake: [i32; 8],   // Baken, an denen keine Mine zustande kam, gesperrt bis Tick
    pub sperrBis: [i32; 8],   // Baken, an denen keine Mine zustande kam, gesperrt bis Tick
    pub minerWeg: i32,   // wie der Miner zum Bauen gebracht wird: 1 DoBuild, 2 BuildingGroup, 0 HFL
    pub minenGesehen: i32,   // so viele Minen standen beim letzten Schritt
    pub minenBauAn: i32,
    pub minenBau: Gruppe,   // Weg 2: die Spiel-KI setzt den Miner ein
    pub ladeConvec: i32,   // ConVec, der gerade an der Strukturfabrik laedt
    pub ladeSeit: i32,   // ConVec, der gerade an der Strukturfabrik laedt
    pub ziel: i32,   // 0 = nur Kolonie, 1 = Satellit starten
    pub zielErreicht: i32,   // Tick, an dem der Satellit im All war (0 = noch nicht)
    pub raumSchritt: i32,   // Ablauf am Spaceport, siehe raumfahrt()
    pub raumSeit: i32,
    pub raumVersuche: i32,
    pub raumAuftrag: i32,   // was der Spaceport gerade herstellt (0 = nichts)
    pub seltenMineId: [i32; 2],   // Seltenerz: Mine je MiningGroup
    pub seltenSchmelzeId: [i32; 2],
    pub seltenGruppe: [Gruppe; 2],
    pub seltenBake: i32,   // Seltenerz-Bake, die erschlossen wird
    pub seltenBakeSeit: i32,
    pub seltenMiner: i32,
    pub seltenMinerSeit: i32,
    pub startErz: i32,
    pub startSelten: i32,
    pub bau: Gruppe,
    pub bauAn: i32,
    pub geparkt: [i32; 16],
    pub geparktGrund: [i32; 16],   // 0 = Arbeiter-Rangliste, 1 = Leerlauf (nur der zustaendige Bereich weckt es)
    pub anzahlGeparkt: i32,
    pub leerId: [i32; 16],   // seit wann ein Gebaeude untaetig ist (Leerlauf-Regel)
    pub leerSeit: [i32; 16],   // seit wann ein Gebaeude untaetig ist (Leerlauf-Regel)
    pub anzahlLeer: i32,
    pub naechsteArbeit: i32,
    pub moralFrei: i32,   // FreeMoraleLevel schon aufgerufen?
    pub techGemeldet: [u8; 512],   // abgeschlossene Forschung fuer die Spur
    pub naechsteTechPruefung: i32,   // abgeschlossene Forschung fuer die Spur
    pub fabrikStill: i32,   // unitID der Fahrzeugfabrik, die die KI mangels Bedarf stillgelegt hat
    pub fabrikStillW: [i32; 4],   // weitere Fahrzeugfabriken, die mangels Kampfbedarf stillgelegt sind
    pub wunschArbeiter: i32,
    pub wunschRang: i32,
    pub wunschSeit: i32,
    pub naechsteVerteilung: i32,   // Wissenschaftler umverteilen (wissenschaftlerVerteilen)
    pub naechsterScan: i32,
    pub feindArmee: i32,
    pub feindNah: i32,
    pub feindGeb: i32,
    pub feindX: i32,
    pub feindY: i32,
    pub feindFabriken: i32,
    pub feindWT: i32,
    pub feindBestWert: i32,   // staerkstes gesehenes Feindfahrzeug (Qualitaet)
    pub feindBestTyp: i32,   // staerkstes gesehenes Feindfahrzeug (Qualitaet)
    pub feindBestWaffe: i32,   // staerkstes gesehenes Feindfahrzeug (Qualitaet)
    pub forschGestartet: [i32; 4],   // gestartete Themen je Bereich (Kolonie, Ziel, Kampf, Wirtschaft) -- Kuchen anteilig
    pub empRakete: i32,
    pub empAuftrag: i32,
    pub empAuftragSeit: i32,
    pub empStart: i32,
    pub empZielX: i32,
    pub empZielY: i32,
    pub aufmarschX: i32,
    pub aufmarschY: i32,
    pub aufmarschSeit: i32,
    /// Wachgebiet der Basis (Editor-Kacheln x0, y0, x1, y1) und das zuletzt an die Wache gegebene
    pub wacheGebiet: [i32; 4],
    pub wacheGebietAlt: [i32; 4],
    /// Geleitschutz (PAR geleit): Einsatzort der Wache und seit wann dort kein Feind mehr ist (geleitSeit 0 = daheim)
    pub geleitX: i32,
    pub geleitY: i32,
    pub geleitSeit: i32,
    pub geleitRuhe: i32,
    /// Rust (PAR parallel): Aufmarschpunkt der zweiten Angriffsgruppe (0,0 = keiner)
    pub aufmarsch2X: i32,
    pub aufmarsch2Y: i32,
    pub parallelBreite: i32,
    pub eigeneArmee: i32,
    pub sollArmee: i32,
    pub bedrohung: i32,
    pub letzterAngriff: i32,
    pub angriffe: i32,
    pub feindGemeldet: i32,
    pub staerksterAngriff: i32,   // groesste Feindarmee bisher (auf der Karte oder vor der Basis) -- Gedaechtnis zwischen den Wellen
    pub eigeneGeb: i32,   // eigene Gebaeude beim letzten Scan (Verluste erkennen)
    pub wache: Gruppe,
    pub wacheAn: i32,
    pub wacheAnzahl: i32,
    pub feindPosN: i32,
    pub feindPosX: [i32; 16],
    pub feindPosY: [i32; 16],
    pub feindMinD2: i32,
    pub feindNX: i32,
    pub feindNY: i32,
    pub eigeneMobil: i32,
    pub eigeneWTWert: i32,
    pub kampfModus: i32,   // 0 Wache, 1 Abfangen, 2 Angriff auf die Feindbasis
    pub kampfModusSeit: i32,   // 0 Wache, 1 Abfangen, 2 Angriff auf die Feindbasis
    pub angriffStart: i32,   // 0 Wache, 1 Abfangen, 2 Angriff auf die Feindbasis
    pub aussenId: [i32; 6],
    pub aussenX: [i32; 6],
    pub aussenY: [i32; 6],
    pub aussenAngriff: [i32; 6],
    pub aussenAn: [i32; 6],
    pub angriffSektoren: i32,
    pub naechsteVorhersage: i32,   // wann die Techbaum-Vorhersage wieder in die Spur geschrieben wird
    pub messErz: i32,
    pub messSelten: i32,
    pub messTick: i32,
    pub zuflussErz: f64,
    pub zuflussSelten: f64,
    pub wellen: [i32; 8],
    pub anzWellen: i32,
    pub welleSpitze: i32,
    pub welleStart: i32,   // Beginn der letzten Welle, gemessener Abstand zwischen zwei Wellen (0 = unbekannt)
    pub wellenAbstand: i32,   // Beginn der letzten Welle, gemessener Abstand zwischen zwei Wellen (0 = unbekannt)
    pub feindUnterwegs: i32,   // Kampfwert feindlicher Fahrzeuge ausserhalb ihrer Basis
    pub feindWachstum: f64,   // gemessener Zuwachs der Feindarmee je 1000 Ticks (gleitend)
    pub feindArmeeVorher: i32,
    pub zielWert: i32,   // Ziel 2: Einwohnerzahl
    pub turmAnteil: f64,   // Anteil des Solls, den Wachtuerme decken duerfen (Rest: Fahrzeuge) -- Parameter
    pub reserve: f64,
    pub welleEigenStart: i32,
    pub welleEigenMin: i32,
    pub laufNr: i32,
    pub naechstesSpeichern: i32,
    pub erzJeRouteGelernt: f64,
    pub verlustSumme: i32,
    pub maxBew: i32,
    pub ueberstanden: i32,
    pub feindVernichtet: i32,
    pub moralSumme: f64,
    pub moralN: i32,
    pub feindPos: [i32; 192],   // Verluste nach Ort: nah an der eigenen Basis oder nicht
    pub eigenPos: [i32; 192],   // Verluste nach Ort: nah an der eigenen Basis oder nicht
    pub zerstoertHeim: i32,   // Verluste nach Ort: nah an der eigenen Basis oder nicht
    pub verlorenHeim: i32,   // Verluste nach Ort: nah an der eigenen Basis oder nicht
    pub bilZerHeim: i32,   // Verluste nach Ort: nah an der eigenen Basis oder nicht
    pub bilVerHeim: i32,   // Verluste nach Ort: nah an der eigenen Basis oder nicht
    pub garBucht: [i32; 6],   // Garage: wer steht in welcher Bucht, wer faehrt hin
    pub garFrei: [i32; 6],   // Garage: wer steht in welcher Bucht, wer faehrt hin
    pub garDran: i32,   // Garage: wer steht in welcher Bucht, wer faehrt hin
    pub garDranSeit: i32,   // Garage: wer steht in welcher Bucht, wer faehrt hin
    pub garRaus: i32,   // Garage: wer steht in welcher Bucht, wer faehrt hin
    pub garRausSeit: i32,   // Garage: wer steht in welcher Bucht, wer faehrt hin
    pub garZaehl: i32,   // Garage: wer steht in welcher Bucht, wer faehrt hin
    pub zielEmp: i32,   // EMP-Anteil der Sammelgruppe; Spinnen: wer uebernimmt wen
    pub zielEmpFz: i32,   // Rust (kampfModell): Fahrgestell der EMP-Beimischung
    pub feindArmeeHeim: i32,
    pub feindArmeeZurueck: i32,   // PAR abwehrRueckkehr: Feindarmee, die vor unserer Truppe wieder daheim sein kann   // Rust (kampfModell): Kampfwert feindlicher Fahrzeuge innerhalb von 15 Kacheln um ihre Basis
    pub unterwegsN: i32,   // Rust (kampfModell): Abstaende feindlicher Kampffahrzeuge zur eigenen Basis beim letzten Scan
    pub unterwegsId: [i32; 128],
    pub unterwegsD: [i32; 128],
    pub zielSchaum: i32,   // Rust: Stickyfoam-Lynx in der Sammelgruppe
    pub feindDrohung: i32,   // Rust (kampfModell): Kampfwert im Nahradius, der naeher kommt oder auf unserer Seite steht
    pub bilUeber: i32,   // EMP-Anteil der Sammelgruppe; Spinnen: wer uebernimmt wen
    pub spinneZiel: [i32; 8],   // EMP-Anteil der Sammelgruppe; Spinnen: wer uebernimmt wen
    pub spinneId: [i32; 8],   // EMP-Anteil der Sammelgruppe; Spinnen: wer uebernimmt wen
    pub spinneTakt: i32,   // EMP-Anteil der Sammelgruppe; Spinnen: wer uebernimmt wen
    pub bilRueck: i32,   // Rueckzug je Angriff; Verfolgung: gejagte Einheit
    pub bilRueckErz: i32,   // Rueckzug je Angriff; Verfolgung: gejagte Einheit
    pub jagdZiel: i32,   // Rueckzug je Angriff; Verfolgung: gejagte Einheit
    pub jagdSeit: i32,   // Rueckzug je Angriff; Verfolgung: gejagte Einheit
    pub jagdTakt: i32,   // Rueckzug je Angriff; Verfolgung: gejagte Einheit
    pub bilK: [i32; 6],   // erster Feindkontakt der Truppe: dt, Modus, nah, alle, E nah, D nah; 300 Ticks spaeter: nah, E, D
    pub bilK2: [i32; 3],   // erster Feindkontakt der Truppe: dt, Modus, nah, alle, E nah, D nah; 300 Ticks spaeter: nah, E, D
    pub bilKTick: i32,   // erster Feindkontakt der Truppe: dt, Modus, nah, alle, E nah, D nah; 300 Ticks spaeter: nah, E, D
    pub bilE: i32,   // Angriffsbilanz: erwartete Staerken, Ankunft der Truppe
    pub bilD: i32,   // Angriffsbilanz: erwartete Staerken, Ankunft der Truppe
    pub bilArmee: i32,   // Angriffsbilanz: erwartete Staerken, Ankunft der Truppe
    pub bilKontakt: i32,   // Angriffsbilanz: erwartete Staerken, Ankunft der Truppe
    pub bilErste: i32,   // Angriffsbilanz: erwartete Staerken, Ankunft der Truppe
    pub bilNach: i32,   // Angriffsbilanz: erwartete Staerken, Ankunft der Truppe
    pub bilGruppe: i32,   // Angriffsbilanz: erwartete Staerken, Ankunft der Truppe
    pub feindId: [i32; 192],   // feindId: unitID | (Typ << 16) -- IDs werden schnell neu vergeben
    pub feindWert: [i32; 192],   // feindId: unitID | (Typ << 16) -- IDs werden schnell neu vergeben
    pub anzFeindId: i32,   // feindId: unitID | (Typ << 16) -- IDs werden schnell neu vergeben
    pub eigenId: [i32; 192],
    pub eigenWert: [i32; 192],
    pub anzEigenId: i32,
    pub zweiteSchmelzeTick: i32,
    pub lynxFreiSeit: i32,
    pub siegTick: i32,   // neue Wertung
    pub niederlageTick: i32,   // neue Wertung
    pub verlustGebaeude: i32,   // neue Wertung
    pub verlustEinheiten: i32,   // neue Wertung
    pub feindHatteBasis: i32,   // neue Wertung
    pub zerstoertWert: i32,   // Erzwert zerstoerter Feind- bzw. verlorener eigener Einheiten
    pub verlorenWert: i32,   // Erzwert zerstoerter Feind- bzw. verlorener eigener Einheiten
    pub bewertet: i32,   // Wertung beim Lauf-Ende (WERTUNG_TICK) eingefroren
    pub laborWunsch: i32,
    pub laborWert: i32,
    pub laborFuer: i32,
    pub naechsterLaborCheck: i32,
    pub laborNutzen: [i32; 4],
    pub letztesKampfFz: i32,
    pub letzterBau: i32,
    pub kampfBedarfSeit: i32,
    pub bauDrangSeit: i32,
    pub naechstePruefung: i32,
    pub kampfLaeuft: i32,   // 1 waehrend kampfFuehrung -- steht es beim naechsten Aufruf noch, ist sie abgestuerzt
    pub sammel: Gruppe,
    pub angriff: Gruppe,
    pub eingreif: Gruppe,   // Eingreifreserve fuer die Bereiche
    pub reserveAn: i32,   // Eingreifreserve fuer die Bereiche
    pub reserveZiel: i32,   // Eingreifreserve fuer die Bereiche
    pub reserveOrt: i32,   // Eingreifreserve fuer die Bereiche
    pub reserveSeit: i32,   // Eingreifreserve fuer die Bereiche
    pub angriff2: Gruppe,   // zweite Angriffsgruppe: eine Gruppe fasst nur 32
    pub angriff2An: i32,   // zweite Angriffsgruppe: eine Gruppe fasst nur 32
    pub fabrik: Gruppe,
    pub sammelAn: i32,
    pub angriffAn: i32,
    pub fabrikAn: i32,
    pub fabrikVf: i32,
    pub fabrikDrin: [i32; 4],   // Fahrzeugfabriken in der Fabrikgruppe
    pub zielFz: i32,
    pub zielWaffe: i32,
    pub wacheZiel: i32,
    pub sammelZiel: i32,
    pub gruppenBedarf: i32,
    pub kampfFzAnzahl: i32,
    pub angriffArt: i32,
    pub angriffWunsch: i32,
    pub angriffZerstoertStart: i32,
    pub angriffVerlorenStart: i32,
    pub letzterAngriffStart: i32,
    pub bilStart: i32,
    pub bilEmp: i32,
    pub bilEinsatzErz: i32,
    pub bilEinsatzSelten: i32,
    pub bilFeindWT: i32,
    pub bilN: i32,
    pub bilFN: i32,
    pub bilTyp: [i16; 12],
    pub bilWaffe: [i16; 12],
    pub bilAnz: [i16; 12],
    pub bilFTyp: [i16; 12],
    pub bilFWaffe: [i16; 12],
    pub bilFAnz: [i16; 12],
    pub angriffQuote: [f64; 3],   // gelernt: zerstoert / verloren je Angriffsart (gleitend)
    pub zermuerbAbstand: i32,   // gelernt: Ticks zwischen zwei Zermuerbungsangriffen                 // je Labortyp (1 Basic, 2 Standard, 3 Advanced): wie dringend die Forschung ihn braucht
    pub angriffZiel: i32,   // unitID des Feindgebaeudes, das die Angriffstruppe gerade angreift
    pub eroeffWarteSeit: i32,   // seit wann ein Labor auf ein Thema der Eroeffnung wartet (0 = keines)
    pub fwAngriff: i32,   // Fruehe Wehr: geschaetzter fruehester Angriff (Tick, 0 = noch nicht berechnet)
    pub feindPotenzial: i32,   // was der Feind in der Reaktionszeit aufstellen koennte (Armee + Fabriken)
    pub erzUeberschuss: i32,   // Bilanz ohne Angriffstruppe: Erz reicht im Horizont (1) oder nicht (0)          // Bitmaske der 8 Himmelsrichtungen, aus denen Feinde an die Kolonie kamen
    pub aussenGruppe: [Gruppe; 6],
    /// Rust (PAR zugang): Wachgruppen an den weiteren Zugaengen (Index 0 = Hauptwache, hier 1..3), Passmitte, Rueckzug bis Tick
    pub zugGruppe: [Gruppe; 4],
    pub zugAn: [i32; 4],
    pub zugX: [i32; 4],
    pub zugY: [i32; 4],
    pub zugZurueck: [i32; 4],
    /// Rust (Truppnetz): laufender Plan -- an, Ziel, Rueckzug unter planRueck x Startwert (Promille), weiter nach dem Ziel, Startwert
    pub planAn: i32,
    pub planZiel: Id,
    pub planRueck: i32,
    pub planWeiter: i32,
    pub planStart: i32,
    /// Route: Aufmarschpunkt auf der Route (planAufX = 0: wie bisher) und die Wegpunkte danach bis vors Ziel
    pub planAufX: i32,
    pub planAufY: i32,
    pub planWpN: i32,
    pub planWpI: i32,
    pub planWpX: [i32; 16],
    pub planWpY: [i32; 16],
    /// Missionen (PAR missionen): je Platz Art (0 frei), Ort (Kacheln), Ziel, Gruppe, Phase, Beginn, Erzwert der Truppe beim
    /// Start, verlangte Kampfkraft, Wertung (x1000), letzter Befehl; gesperrte Orte (erfolglos) bis Tick; naechste Wahl
    pub missArt: [i32; 8],
    pub missX: [i32; 8],
    pub missY: [i32; 8],
    pub missZiel: [Id; 8],
    pub missGruppe: [Gruppe; 8],
    pub missGruppeAn: [i32; 8],
    pub missPhase: [i32; 8],
    pub missSeit: [i32; 8],
    pub missStart: [i32; 8],
    pub missSoll: [i32; 8],
    pub missWert: [i32; 8],
    pub missBefehl: [i32; 8],
    pub missSperrX: [i32; 16],
    pub missSperrY: [i32; 16],
    pub missSperrBis: [i32; 16],
    pub missNaechste: i32,
    pub missPool: Gruppe,
    pub missPoolAn: i32,
    /// Lernen (PAR missionLernen): je Platz geschaetzte Abwehr beim Start und geschickte Kampfkraft (waechst mit Verstaerkung);
    /// je Missionsart Summe gemessener (aus dem Verlust zurueckgerechneter) und geschaetzter Abwehr
    pub missDd: [f64; 8],
    /// Forschung (PAR forschungVorrang): je Labor der Bereich des gestarteten Themas (0 Kolonie, 1 Ziel, 2 Kampf,
    /// 3 Wirtschaft, -1 Fueller/sonst)
    pub forschLab: [Id; 8],
    /// Fabrik (PAR fabrikWirtschaft): seit wann die Kampf-Sollzahlen fuer ein dringenderes Wirtschaftsfahrzeug ruhen (0 nein)
    pub wirtVorrang: i32,
    /// Fabrik des laufenden Wirtschaftsfahrzeug-Auftrags (PAR fabrikLaeuft)
    pub fzVf: Id,
    pub forschLabBereich: [i32; 8],
    pub missKwStart: [i32; 8],
    pub missLernIst: [f64; 8],
    pub missLernErw: [f64; 8],
}
impl Zustand {
    /// Alles 0 (wie die statische Speicherung in der Mission).
    pub fn null() -> Box<Zustand> {
        // Sicher: alle Felder sind Zahlen bzw. Felder/Strukturen aus Zahlen.
        unsafe { Box::new(std::mem::zeroed()) }
    }
}
