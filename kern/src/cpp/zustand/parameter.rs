//! Parameter (PAR): alle Stellschrauben mit Doku und Standardwert.
use super::*;
use super::super::*;

/// Stellschrauben je Spieler (C++ struct Parameter, Standardwerte wie dort).
#[derive(Clone, Copy, Debug)]
pub struct Parameter {
    /// Ticks, die die KI zum Nachruesten braucht (Feindwachstum, Fabrikentscheidung)
    pub reaktion: f64,
    /// Planungshorizont der Rohstoffbilanz
    pub horizont: f64,
    /// so viel staerker muss die Truppe fuer Basis- und Zielangriff sein
    pub ueberlegenheit: f64,
    /// weitere Fahrzeugfabrik, wenn die Aufruestung danach hoechstens so lange dauert
    pub fabrikGewinn: f64,
    /// Reserve auf die Feindarmee zu Beginn
    pub reserveStart: f64,
    /// Anstieg der Reserve je Anteil verlorenen Kampfwerts in einem Gefecht
    pub reserveLernrate: f64,
    /// Faktor nach einem Gefecht ohne Verluste
    pub reserveRueckgang: f64,
    /// Deckel der Reserve
    pub reserveMax: f64,
    /// Anteil des Solls, den Wachtuerme decken duerfen
    pub turmAnteil: f64,
    /// Ticks zwischen Zermuerbungsangriffen (Start)
    pub zermuerbAbstand: f64,
    /// Mindestnutzen (beseitigter Engpass / Kosten) fuer Schmelze bzw. Seltenerz-Schmelze
    pub schmelzeNutzen: f64,
    /// Auslastung in %, ab der Wohnraum dringlich ist und kein Haus ueberzaehlig
    pub wohnraumGrenze: f64,
    /// Anteil am Nutzen der Techniken, die eine Technik freischaltet
    pub freischaltWert: f64,
    /// Feind-Kampfeinheiten so nah an der Basismitte = Angriff auf die Kolonie
    pub nahRadius: f64,
    /// Faktor auf das, was beim Angriff daheim bleiben muss
    pub heimHalten: f64,
    /// jeder Angriff soll so viel staerker sein als der letzte (Eskalation)
    pub steigerung: f64,
    /// EMP-Rakete, wenn die gelaehmte Abwehr >= Schwelle x Kosten der Rakete (in Kampfwert)
    pub empSchwelle: f64,
    /// neue Erzmine, sobald so viele Schmelzen je Mine stehen (und Erz fehlt); 2 = Mine voll
    pub schmelzenJeMine: f64,
    /// Militaerforschung mindestens, solange ein Feind bekannt und kein Kampffahrzeug erforscht ist
    pub vorsorge: f64,
    /// Seltenerz-Schmelze bauen und betreiben, sobald Fahrzeuge mit Seltenerz so viel mehr Kampfkraft je Erz bringen (0.25 = +25 %)
    pub seltenGewinn: f64,
    /// Anteil der Wache, der beim Angriff mindestens mitfaehrt
    pub risikoAnteil: f64,
    /// Zermuerben: erwarteter Gewinn mindestens dieser Anteil der eigenen Staerke
    pub angriffMarge: f64,
    /// Rueckzug, wenn eigene Staerke vor Ort < Faktor x feindliche Staerke vor Ort
    pub rueckzugAb: f64,
    /// 0 = kein Langzeit-Wissen
    pub lernen: f64,
    /// Erweiterung Gelaende: Tuerme und Wache an der Engstelle (0 aus, 1 an)
    pub gelaende: f64,
    /// Engstelle fruehestens so viele Wegkacheln von der Basis
    pub engMin: f64,
    /// Engstelle hoechstens so weit
    pub engMax: f64,
    /// Querschnitt hoechstens so breit (Kacheln), sonst offenes Feld
    pub engBreite: f64,
    /// Wachtuerme je Pass
    pub engTuerme: f64,
    /// Tuerme und Wache so viele Kacheln hinter dem Pass (eigene Seite)
    pub engHinter: f64,
    /// Erweiterung Zweitbasis (0 aus, 1 an)
    pub zweitbasis: f64,
    /// Bakenfeld mindestens so viele Wegkacheln von der Basis
    pub zbMinWeg: f64,
    /// Mehrertrag im Horizont / Kosten, ab dem gebaut wird
    pub zbNutzen: f64,
    /// Wachtuerme der Zweitbasis
    pub zbTuerme: f64,
    /// Fahrzeugfabrik an der Zweitbasis (0/1)
    pub zbFabrik: f64,
    /// Strukturfabrik an der Zweitbasis (0/1)
    pub zbStruktur: f64,
    /// Zweitbasis nur, wenn der Feind mindestens so viel weiter weg ist als wir (Faktor)
    pub zbFeindAbstand: f64,
    /// Verteidigung mindestens so dringlich, solange der Pass unbesetzt ist (Gelaende)
    pub engDringlichkeit: f64,
    /// ein Turm am Pass zaehlt so viel weniger gegen den Turm-Anteil (er ist dort mehr wert)
    pub engTurmFaktor: f64,
    /// Pass bis zu dieser Breite bis auf eine Luecke zumauern (0 = nie)
    pub engMauerBreite: f64,
    /// Zweitbasis erst, wenn die eigene Armee mindestens diesen Anteil des Solls hat
    pub zbArmee: f64,
    /// Zweitbasis: hoechstens so viele (Nutzer 2026-11-19: feste Zahl, spaeter je Schwierigkeit), 0..4
    pub zbAnzahl: f64,
    /// Zweitbasis bereit (lernbar, Nutzer 2026-11-19): hoechstens so viele offene Bauplaetze daheim, Bedrohung unter zbBedrohung,
    /// Feindarmee hoechstens zbFeind x eigene, Erz mindestens zbErz x (Kommandozentrale + Schmelze), freie Arbeiter
    /// mindestens zbArbeiter x Bedarf der Zweitbasis
    pub zbOffen: f64,
    pub zbBedrohung: f64,
    pub zbFeind: f64,
    pub zbErz: f64,
    pub zbArbeiter: f64,
    /// Zweitbasis: Erzueberschuss (Bilanz) verlangen (1) oder nicht (0) -- lernbar
    pub zbUeberschuss: f64,
    /// Kein Platz am Roehrenstutzen: freien Platz bis zu diesem Abstand nehmen und per Roehre anschliessen (0 = nie)
    pub platzFern: f64,
    /// Platz ohne Anschluss (neues Modell): Wertung je noetiger Roehrenkachel, in Abstand^2-Kacheln zur Basismitte
    pub roehreKachel: f64,
    /// Bebauungsplan (Rust, nur mit kampfModell): Plaetze fuer den Bedarf im Horizont vorab reservieren (0 = aus)
    pub vorplan: f64,
    /// Moral (Rust, nur mit kampfModell): abgeschaltete Gebaeude stilllegen, sobald frei wieder einschalten (0 = aus)
    pub moralStill: f64,
    /// Rust: wehrloser Feind -> ganze Angriffstruppe (1 an, 0 alt)
    pub wehrlos: f64,
    /// Rust: groesste Angriffswelle in Fahrzeugen, Rest folgt als Nachschub (0 = alles auf einmal)
    pub welle: f64,
    /// Rust: Waffenwahl nach Paarungen (Gewicht des gemessenen Paarungswerts; 0 aus)
    pub paarung: f64,
    /// Rust: Wachturm ohne Roehrenverbindung zur Zentrale zaehlt 0 (1 an)
    pub turmNetz: f64,
    pub turmOhneNetz: f64,
    /// Rust: Wissenschaftler aus der Forschung als Arbeiter statt Stilllegen; Ausbildung nur ohne Arbeitermangel (1 an)
    pub wissArbeit: f64,
    /// Rust (Nutzer 2026-11-18, Forschung zu langsam): Wissenschaftler ausbilden, sobald Labore Platz haben, und die
    /// wissArbeit-Sperre nur, soweit unbesetzte Gebaeude die freien Arbeiter wirklich brauchen (1 an)
    pub wissAusbau: f64,
    /// Rust (Nutzer 2026-11-18): Konzentration der Wissenschaftler nach dem Kuchen-Anteil des Bereichs statt nach der
    /// festen Themengruppe (Kampfthemen wie Panther/EMP standen dort auf 0 und warteten immer) (1 an)
    pub forschungVorrang: f64,
    /// Rust (Nutzer 2026-11-18): ist ein Wirtschaftsfahrzeug (Robo-Miner, Lastwagen ...) dringender als das naechste
    /// Kampffahrzeug, ruhen die Sollzahlen der Kampfgruppen, bis die Fabrik es baut (hoechstens 4000 Ticks) (1 an)
    pub fabrikWirtschaft: f64,
    /// Rust (Nutzer 2026-11-18): Rang des Robo-Miners fuer die Seltenerz-Mine aus dem Vorteil der Seltenerz-Fahrzeuge
    /// (seltenVorteil): 50 + halber Vorteil in %, hoechstens 95 -- statt fest 50 (1 an)
    pub seltenNutzen: f64,
    /// Rust (Nutzerspiel 2026-11-19): Wirtschaftsfahrzeuge in einer Fahrzeugfabrik bauen, die laeuft (Strom und
    /// Arbeiter), statt immer in der ersten -- die zweite stand ohne Roehrenanschluss da, alle ConVec-Auftraege gingen
    /// ins Leere (1 an)
    pub fabrikLaeuft: f64,
    /// Rust (Nutzerspiel 2026-11-19): zur Abwehr der Feindbasis zaehlt jede feindliche Kampfeinheit, deren Heimweg nicht
    /// laenger ist als unser Anmarsch (weiter weg abnehmend ueber 30 Kacheln) -- statt nur 15 Kacheln um die Basis; die
    /// KI griff an, waehrend die Feindarmee unterwegs war, und musste jedes Mal umkehren (1 an)
    pub abwehrRueckkehr: f64,
    /// Rust (Nutzer 2026-11-19): Lueckenfueller-Tokamak nur, wenn der freie Strom fuer die naechsten geplanten und schon
    /// baubaren Gebaeude (Bebauungsplan) nicht reicht -- sonst bleibt der ConVec frei fuer das, was gleich gebraucht wird (1 an)
    pub lueckeKlug: f64,
    /// Wachtuerme (Rust): jeden Turm ohne Roehre zur Kommandozentrale per Roehre anschliessen (0 = aus)
    pub turmRoehre: f64,
    /// Mauerlinie (Rust, Nutzer 2026-11-19): Tuerme einer Front in einer waagerechten/senkrechten Reihe, dazwischen eine
    /// durchgehende Mauer statt verstreuter Stuecke, mit Tor neben einem Turm (0 = aus); Laenge je Seite, Torbreite und
    /// Abstand zwischen zwei Toren in Kacheln
    pub mauerLinie: f64,
    pub mauerLaenge: f64,
    pub torBreite: f64,
    pub torAbstand: f64,
    /// Bau-Reihenfolge (Rust, Nutzer 2026-11-19): erst Gebaeude mit Personal anschliessen, dann Tuerme, Mauern erst bei Luft
    /// in Schueben von mauerSchub Kacheln (0 = aus)
    pub bauReihe: f64,
    pub mauerSchub: f64,
    /// Zugangsmauern (Rust, Nutzer 2026-11-19): Ring im Wegabstand zmAbstand um die Kolonie, Zugaenge bis zmKlein Kacheln
    /// offen, bis zmMax Mauer mit Tor (torBreite) und zwei Tuermen zmHinter Kacheln dahinter (0 = aus)
    pub zugangMauer: f64,
    pub zmAbstand: f64,
    pub zmKlein: f64,
    pub zmMax: f64,
    pub zmHinter: f64,
    /// Laenge der Schildmauer vor breiten Zugaengen (mehr als zmMax Kacheln)
    pub zmSchild: f64,
    /// Fruehe Wehr (fest): Mindest-Aufruestzeit eines Gegners in Ticks, Fahrzeit je Wegkachel, Mindest-Armee-Soll und das
    /// Zeitfenster um den geschaetzten fruehesten Angriff (Vorlauf davor, Nachlauf danach) -- lernbar
    pub fwAufruest: f64,
    pub fwFahrt: f64,
    pub fwWert: f64,
    pub fwVorlauf: f64,
    pub fwNach: f64,
    /// Fruehe Wehr: erwartete Bedrohung zum geschaetzten fruehesten Angriff (0..100)
    pub fwBedrohung: f64,
    /// Fruehe Wehr: Daempfung der anderen Bereiche, solange die Mindestabwehr fehlt (0..1)
    pub fwVorrang: f64,
    /// Eroeffnung der Forschung (fest, forschung/labore.rs EROEFFNUNG): Dringlichkeit des Labors, das ihr naechstes Thema
    /// braucht (lernbar)
    pub eroeffLabor: f64,
    /// Messschalter (nicht fuer das Spiel): Eroeffnung aus, um das alte Verhalten im Duell zu vergleichen (0 = Eroeffnung an)
    pub eroeffAus: f64,
    /// Eroeffnung nur, wenn der frueheste Angriff (fruehe Wehr) bis zu diesem Tick kommt -- auf grossen Karten mit spaetem
    /// Angriff forscht die KI wie sonst (lernbar)
    pub eroeffBis: f64,
    /// Eroeffnung: so viele Ticks darf ein Labor auf ihr naechstes Thema warten, dann forscht es wie sonst (lernbar)
    pub eroeffWarten: f64,
    /// So viele Bauten duerfen gleichzeitig offen sein (aufgezeichnet, noch nicht gesetzt), hoechstens einer je ConVec (lernbar)
    pub bauGleichzeitig: f64,
    /// Wachgebiet: Rand um die Gebaeude der Basis und hoechster Abstand von der Mitte, bis zu dem Gebaeude zaehlen (lernbar)
    pub wacheRand: f64,
    pub wacheMax: f64,
    /// Forschungs-Kuchen: Faktor auf den Kampf-Anteil (1 = wie die Dringlichkeit sagt; lernbar)
    pub forschKampf: f64,
    /// Forschungsziel, das nur mit Seltenerz baubar ist, solange keins da ist: Anteil seines Nutzens (lernbar)
    pub seltenOhne: f64,
    /// Wert schnellerer Kampffahrzeuge in der Forschung (Anteil des Einheitwerts je +100 % Tempo, 0 = aus)
    pub tempoWert: f64,
    /// Geleitschutz: Wache faehrt zu Feinden an Lastwagen, Bergbau und Minen ausserhalb des Wachgebiets
    pub geleit: f64,
    /// Panzerung frueh mehr wert (Nutzer 2026-11-20): Zuschlag auf den Panzerungsnutzen, faellt bis Tick PAR panzerBis auf 0
    pub panzerFrueh: f64,
    /// Panzerungs-Forschung richtig bewerten (Nutzer 2026-11-20): Klasse -> Panzerwert der Simulation, Schaden der Feindwaffen
    pub panzerKlasse: f64,
    /// Kampfwert (gegenwert) mit Panzer-Ausbau gegen den Waffenmix der Feinde (Nutzer 2026-11-20, Punkt 3)
    pub panzerModell: f64,
    /// Forschungslinie (Nutzer 2026-11-20): das zuletzt beste Ziel eines Bereichs zaehlt x zielTreue (1 = aus)
    pub zielTreue: f64,
    /// Voraussetzung fuer Baubares (Nutzer 2026-11-20, Panther -> Railgun): Folgetechniken, die eine Einheit freischalten,
    /// zaehlen mit vorausBau statt freischaltWert, und seltenOhne trifft nur den eigenen Nutzen (0 = aus)
    pub vorausBau: f64,
    /// Missionsmodus (Nutzer 2026-11-22, modus.rs): 1 = Mission statt Mehrspieler
    pub missionsModus: f64,
    /// Schwierigkeit 0 leicht, 1 mittel, 2 schwer; -1 = aus dem Spiel (Missionsmodus) bzw. schwer (Mehrspieler)
    pub stufe: f64,
    /// Rolle: 0 keine, 1 aggressiv, 2 defensiv, 3 Kolonie
    pub rolle: f64,
    /// Eigene Angriffe der KI (1 an, 0 aus; Angriffe per Trigger der Mission bleiben moeglich)
    pub eigeneAngriffe: f64,
    /// Test (Simulator): ab diesem Tick einmal einen Aussenposten-Auftrag (0 = nie) an testPostenX/Y mit testPostenGroesse,
    /// Bake testBakeX/Y (0 = die im Umkreis)
    pub testPosten: f64,
    pub testPostenX: f64,
    pub testPostenY: f64,
    pub testPostenGroesse: f64,
    pub testBakeX: f64,
    pub testBakeY: f64,
    /// Minen nur im Umkreis von so vielen Kacheln um eine Basis (0 = aus; Missionsmodus 35, Nutzer 2026-11-22)
    pub minenUmkreis: f64,
    /// hoechstens so viele Minen je Erzsorte und Basis (0 = unbegrenzt; Missionsmodus 1)
    pub minenJeSorte: f64,
    /// Warte- und Sammelplatz der Armee vor dem bebauten Umkreis statt in der Basis (Missionsmodus 1)
    pub wacheVorBasis: f64,
    /// Wache patrouilliert ohne Angriff um die Basis (Missionsmodus 1), Ticks je Patrouillenpunkt
    pub wachePatrouille: f64,
    pub wachePatrouilleTakt: f64,
    /// Hauptbasis in Bezirken (Industrie, Wohnen, Forschung, Energie) statt dicht um die Mitte (Missionsmodus 1)
    pub bezirke: f64,
    /// Gebiet der Basis selbst finden (schmalster Ring) und alle Zugaenge befestigen, auch an der zweiten Basis (Missionsmodus 1)
    pub gebietAuto: f64,
    /// luftiger Kern mit kurzen Roehren (bezirke::kernPlatz; Missionsmodus 1) und Vorrang offener Roehren (Erz vorgemerkt,
    /// Armee wartet; Missionsmodus 1)
    pub kernLuft: f64,
    pub roehrenVorrang: f64,
    /// Roehren enden an der naechsten Netzroehre, kein zweiter Strang neben einem schon verlegten Weg (Missionsmodus 1)
    pub roehreAnNetz: f64,
    /// Kindergaerten bei Ueberbevoelkerung oder Nahrungsverlust stilllegen (Missionsmodus 1)
    pub kigaBremse: f64,
    /// Kindergarten aus ueber so vielen freien Arbeitern, wieder an darunter (kigaBremse)
    pub kigaAus: f64,
    pub kigaAn: f64,
    /// Missionsmodus: Ziel der KI (0 = wie die Mission startet; 1 Satellit/Raumhafen, 2 Bevoelkerung, 3 Feind vernichten)
    pub missionZiel: f64,
    /// Missionsmodus: einen Raumhafen in die Mitte der Basis bauen, mit Festung (1)
    pub raumhafen: f64,
    /// Missionsmodus, Plymouth: Spinnenfabrik und Spinnen fuer die Reparatur, je Basis (1)
    pub spinnen: f64,
    /// Missionsmodus: Fahrzeuge steigen mit der Forschung auf -- bestes erforschtes Fahrgestell (Tiger > Panther > Lynx),
    /// Waffe nach der Paarungs-Arena, Forschung dorthin (1)
    pub aufstieg: f64,
    /// Missionsmodus: MHD-Generator, sobald Seltenerz da ist (1)
    pub mhd: f64,
    /// Missionsmodus: Solarsatellit starten und ein Solarkraftwerk bauen (1)
    pub solar: f64,
    /// Missionsmodus, zweite Basis: Kampffahrzeuge ihrer eigenen Wache (4) und unter Bedrohung (6); 0 = keine Wache
    pub zbWache: f64,
    pub zbWacheDruck: f64,
    /// Missionsmodus, zweite Basis: so lange (Ticks) bleibt das Erz fuer ihren naechsten Bau vorgemerkt (Vorrang vor der
    /// Hauptbasis), erneuert solange sie wartet (3000); 0 = kein Vorrang
    pub zbVorrang: f64,
    /// Test (Simulator): ab diesem Tick einmal einen Angriffsauftrag wie per Trigger (0 = nie), mit testStaerke
    pub testAngriff: f64,
    pub testStaerke: f64,
    /// Untergrenze der verlangten Ueberlegenheit beim Basisangriff (Standard 1; Rolle aggressiv 0,85)
    pub ueberlegMin: f64,
    /// Kein eigener Angriff vor diesem Tick (die Stufe setzt ihn)
    pub schonfrist: f64,
    /// Nur die feste Moral der Mission behalten (75), Kolonie wie sonst -- Vergleichsbasis fuer moralFest (0 = aus)
    pub moralHalten: f64,
    /// Stabile Moral (Mehrspieler 75): Moral nicht freigeben, keine Gebaeude und Dringlichkeiten fuer Wohnraum, Medizin, Freizeit (0 = aus)
    pub moralFest: f64,
    /// Eroeffnung Universitaet, AdvancedLab, dann Kindergarten (Forschung und Bau; forschung/labore.rs, planung/bauwahl.rs; 0 = aus)
    pub eroeffFolge: f64,
    /// Kindergarten-Forschung (3304) erst nach der mobilen Waffenplattform (3851) (forschung/labore.rs; 0 = aus)
    pub kigaNachLynx: f64,
    /// Mine mit der Bake links, wenn rechts der Rand nicht befahrbar ist (erz/routen.rs minenEcke; 0 = aus)
    pub mineSeite: f64,
    /// Schmelze der Startbasis mit diesem Fahrweg (Kacheln) zur ersten Bake, auf beiden Seiten gleich (ablauf/aufbau.rs; 0 = aus)
    pub startSchmelze: f64,
    /// Baken vor der Startbasis erfassen, damit sie keine Minenplaetze zubaut (ablauf/schritt.rs; 0 = aus)
    pub startBaken: f64,
    /// Sparen fuer eine lohnende Schmelze hoechstens so viele Ticks am Stueck, dann ebenso lange nicht (bau/arbeit.rs; 0 = aus)
    pub sparFrist: f64,
    /// Unter dem Armee-Soll hoechstens alle so viele Ticks ein weiterer Lastwagen (erz/fahrzeuge.rs; 0 = aus)
    pub lkwWechsel: f64,
    /// Weitere Schmelze wartet, solange eigene Armee + Tuerme < schmelzeRisiko x Feindarmee, ausser sie zahlt sich vor dem
    /// ersten erwarteten Angriff aus (planung/lage.rs erzWahl; 0 = aus)
    pub schmelzeRisiko: f64,
    /// Bauplaetze nur, wo ein ConVec von der Basis aus ohne grossen Umweg hinkommt (hilfen/platz.rs erreichbar; 0 = aus)
    pub platzErreichbar: f64,
    /// Stau zwischen eigenen Bau- und Erdbaufahrzeugen loesen (bau/stau.rs; 0 = aus)
    pub bauStau: f64,
    /// Erforschte Moral-Technik (Medizin, Freizeit, DIRT, GORF): ihr erstes Gebaeude sofort bauen (0 = aus)
    pub moralBau: f64,
    /// Feldherr (Nutzer 2026-11-21): auf dem Marsch zum Aufmarschpunkt in Etappen sammeln (kampf/marsch.rs; 0 = aus)
    pub marschSammeln: f64,
    /// Etappenlaenge in Wegkacheln
    pub marschEtappe: f64,
    /// Radius (Kacheln) um den Etappenpunkt, in dem die Gruppe als beisammen zaehlt
    pub marschRadius: f64,
    /// Anteil der Gruppe, der am Etappenpunkt stehen muss, bevor es weitergeht
    pub marschAnteil: f64,
    /// Feldherr: der Aufmarschpunkt liegt auch so viele Kacheln von feindlichen Kampffahrzeugen weg, nicht nur 9 von den
    /// Tuermen -- gemessen 2026-11-21: die Truppe sammelte sich mitten in der Feindarmee und starb vor dem Angriff (0 = aus)
    pub aufmarschFeind: f64,
    /// Feldherr (Nutzer 2026-11-19): beim Losschlagen bleiben Fahrzeuge mit mehr als diesem Schadensanteil daheim (Wache,
    /// Garage, Reparatur) -- beschaedigte fahren langsamer (op2-sim movement: Zeit + Schaden/HP) und reissen die Truppe auseinander (0 = aus)
    pub marschHeil: f64,
    /// Keine zweite Erzschmelze, solange Kindergarten, Universitaet oder das Labor der Eroeffnung fehlen (0 = aus).
    /// Gemessen 2026-11-20: auf ml4_08/ml6_21 schob die Schmelze bei Tick 643 die Eroeffnung um ~4500 Ticks, das Erz lag ungenutzt
    pub schmelzeEroeff: f64,
    /// Dritte und weitere Erzschmelze spaeter (Nutzer 2026-11-20): ihr Nutzen / schmelzeSpaet (1 = aus)
    pub schmelzeSpaet: f64,
    /// Bis zu welcher Bedrohung die Fabrik fuer eine lohnende Schmelze spart (frueher fest 50)
    pub sparBedrohung: f64,
    /// Erstes Medical Center mit Rang 84 statt 70 (0 = aus)
    pub medVorrang: f64,
    /// Bauauftrag nach so vielen Ticks ohne Fortschritt abbrechen (0 = nie)
    pub bauAbbruch: f64,
    /// Ueberbelegt: Wohnhaus vor allem anderen (1 = immer, 2 = erst nach den Gebaeuden der Eroeffnung, 3 = nur waehrend der Eroeffnung: Wohnhaus, Kindergarten, Uni; 0 = aus)
    pub wohnVorrang: f64,
    /// Kampfwert: neu gefittete Gestell- und Waffenfaktoren (Kampf-Arena mit Gefechtsfuehrung; 0 = alte Faktoren)
    pub faktorNeu: f64,
    /// Eden vor der Railgun (5506): so viele Tuerme ueber MAX_WT hinaus, eigener Turmanteil am Armee-Soll (0 = wie sonst),
    /// keine eigenen Angriffe (0 = aus)
    pub laserTuerme: f64,
    pub laserTurmAnteil: f64,
    pub laserKeinAngriff: f64,
    pub panzerBis: f64,
    pub geleitRadius: f64,
    pub geleitMax: f64,
    pub geleitUeber: f64,
    /// Rust: weitere Fahrzeugfabrik, wenn der Erzzufluss mehr ist als die Fabriken verbauen (1 an), Auslastungsschwelle
    pub fabrikErz: f64,
    pub fabrikAuslastung: f64,
    /// Rust: Zermuerben auf die lohnendste schwach verteidigte Stelle (Wert/oertliche Abwehr; 1 an)
    pub schwachstelle: f64,
    /// Rust: Aussenposten-Wachen nach Schwachstellen-Rechnung (1 an; schadete im Duell, Standard 0)
    pub schwachAussen: f64,
    /// Rust: Zugaenge zum ganzen Kolonie-Raum sichern statt zur Basismitte (1 an; braucht gelaende)
    pub zugang: f64,
    /// Rust: Zugangsgruppen an den weiteren Paessen (1 an)
    pub zugangGruppen: f64,
    /// Rust (Truppnetz): Zermuerben waehlt Ziel und Plan per gelerntem Netz (1 an); Risiko = Gewicht des eigenen Verlusts;
    /// Mindestertrag je Startwert; truppAbwehr: dasselbe Netz aus Feindsicht bestimmt die Wachen an den Aussenposten
    pub truppPlan: f64,
    pub truppRisiko: f64,
    pub truppMin: f64,
    pub truppAbwehr: f64,
    /// Modell-Schleife: Schalter fuer den Code eines laufenden Versuchs (0 aus, 1 an) -- der Versuchscode prueft
    /// `PAR(z.spieler).versuch >= 0.5`; das Duell misst Standard+versuch=1 gegen Standard
    pub versuch: f64,
    /// Modell-Schleife: freie Stellschraube des Versuchs (Standard 1.0), z. B. ein Faktor oder eine Schwelle
    pub versuchWert: f64,
    /// Ende der Wertung in Ticks (0 = 200000); Nutzer 2026-11-18: Missionen-Partien bis 1 000 000
    pub wertungTick: f64,
    /// Missionen statt festem Basisangriff (Nutzer 2026-11-18): Ueberfall, Erz/Pass/Vorposten halten, Entscheidungsschlag, Abwehr (1 an)
    pub missionen: f64,
    /// Missionen: Planungshorizont (Ticks) fuer Erzwerte, hoechstens gleichzeitige Missionen, Rueckzug unter diesem Anteil
    /// des Startwerts, verlangte Ueberlegenheit vor Ort, Wert eines Sieges (Erz)
    pub missionHorizont: f64,
    pub missionMax: f64,
    pub missionRueck: f64,
    pub missionUeberleg: f64,
    pub missionSieg: f64,
    /// Missionen lernen aus ihren Verlusten (1 an): Abwehrschaetzung je Art mal (Vorwissen + gemessen) / (Vorwissen +
    /// geschaetzt); Vorwissen-Faktoren aus der Lernrunde fuer Ueberfall, Halten (Erz/Pass/Vorposten), Entscheidung, Abwehr
    pub missionLernen: f64,
    pub missionKorrUeberfall: f64,
    pub missionKorrHalten: f64,
    pub missionKorrEntscheidung: f64,
    pub missionKorrAbwehr: f64,
    /// Rust: Angriff ueber zwei getrennte Wege (1 an, 0 aus); Umweg-Faktor von daheim; Mindestabstand der Aufmarschpunkte
    pub parallel: f64,
    pub parallelUmweg: f64,
    pub parallelAbstand: f64,
    /// Rust: Reihen, die in einem Gang gleichzeitig in Reichweite kommen (Truppe > Breite x Tiefe -> teilen; 0 = immer)
    pub parallelTiefe: f64,
    /// Moral (Rust): Wert einer Moralstufe in Bewohnern je Bewohner (Waehrung von kolonieNutzen)
    pub moralWert: f64,
    /// Erz (Rust, nur mit kampfModell): Bedarf aus dem Bebauungsplan, Schmelze vor Garage/RCC, Sparen fuer die Schmelze (0 = aus)
    pub erzAusbau: f64,
    /// Wachtuerme der Kolonie (Rust, Stellschrauben fuers Training; Standard = bisheriges Verhalten):
    /// Abstand der Turmlinie vor dem aeussersten Gebaeude (Kacheln), Abstand der Tuerme in der Linie und zwischen den Reihen,
    /// Gewicht der Feindrichtung gegen Seiten, von denen schon angegriffen wurde, und Faktor fuer Turmwaffen, die ueber Mauern
    /// schiessen (Saeure, ESG, RPG, Thor, EMP, Stickyfoam), in der Waffenwahl des Turms
    pub turmVorne: f64,
    pub turmAbstand: f64,
    pub turmFront: f64,
    pub turmUeberMauer: f64,
    /// Gefechtsfuehrung (Rust, nur mit kampfModell): Ziele im Nahkampf alle 4 Ticks verteilen (0 = aus); Radius des Gefechts
    /// und wie weit eine Einheit ueber ihre Reichweite hinaus fuer ein Ziel vorruecken darf (Kacheln)
    pub gefecht: f64,
    pub gefechtRadius: f64,
    pub gefechtSpiel: f64,
    /// Stickyfoam-Anteil der Angriffstruppe (Plymouth, Rust; 0 = keiner)
    pub schaumAnteil: f64,
    /// Gefechtsfuehrung: ausserhalb feindlicher Feuerreichweite stellen, wo es deutlich besser ist (0 = aus)
    pub gefechtStellung: f64,
    /// Gefechtsfuehrung: Sperrzeit zwischen zwei Ausweichzuegen einer Einheit (Ticks)
    pub gefechtSperre: f64,
    /// Rust: Abstand halten bei Reichweitenvorteil (1 an), Sperre zwischen zwei Rueckzuegen in Ticks
    pub gefechtAbstand: f64,
    pub gefechtAbstandSperre: f64,
    /// Gefechtsfuehrung: Sicherheitszuschlag auf die feindliche Reichweite (Kacheln)
    pub gefechtReichweitePlus: f64,
    /// Gefechtsfuehrung: Gewicht je Schuetze: 0 = jeder zaehlt 1, 1 = nach Kampfwert
    pub gefechtBedrohungWert: f64,
    /// Gefechtsfuehrung: Faktor fuer Tuerme in der Bedrohung
    pub gefechtTurmGewicht: f64,
    /// Gefechtsfuehrung: wie weit eine Einheit ausweichen darf (Kacheln)
    pub gefechtAusweich: f64,
    /// Gefechtsfuehrung: um welchen Anteil der jetzigen Bedrohung das neue Feld besser sein muss
    pub gefechtBesser: f64,
    /// Gefechtsfuehrung: ab welchem Schaden (Anteil HP) eine Einheit ganz herausgeht
    pub gefechtRueckzugSchaden: f64,
    /// Gefechtsfuehrung: Strafe, wenn eine Schadenseinheit ihr Ziel aus der Reichweite verliert
    pub gefechtZielStrafe: f64,
    /// Gefechtsfuehrung: dasselbe fuer EMP-/Schaum-Einheiten
    pub gefechtStuetzeStrafe: f64,
    /// Gefechtsfuehrung: Gewicht der Wegstrecke beim Ausweichen
    pub gefechtWeg: f64,
    /// Bebauungsplan: erwartetes Bevoelkerungswachstum im Horizont (Anteil), Suchradius um die Basis, Gewicht dichter Packung
    pub vorplanWachstum: f64,
    pub vorplanRadius: f64,
    pub vorplanDicht: f64,
    /// Kampfmodell (Rust): 0 = altes (Erz-Wertmodell, feste EMP-Beimischung 25 %), 1 = Arena-8-Modell mit EMP-Wert und
    /// EMP-Anteil nach Nutzen je Erz
    pub kampfModell: f64,
    /// Messung (Rust): alle 100 Ticks LocationEnumerator am Bakenziel in die Spur (0 = aus)
    pub sonde: f64,
}
impl Default for Parameter {
    fn default() -> Self {
        Parameter {
            reaktion: 6000.0,
            horizont: 20000.0,
            ueberlegenheit: 1.5,
            fabrikGewinn: 0.8,
            reserveStart: 1.3,
            reserveLernrate: 0.3,
            reserveRueckgang: 0.97,
            reserveMax: 2.5,
            turmAnteil: 0.25,
            zermuerbAbstand: 8000.0,
            schmelzeNutzen: 1.0,
            wohnraumGrenze: 85.0,
            freischaltWert: 0.3,
            nahRadius: 22.0,
            heimHalten: 1.0,
            steigerung: 1.15,
            empSchwelle: 1.0,
            schmelzenJeMine: 2.0,
            vorsorge: 60.0,
            seltenGewinn: 0.25,
            risikoAnteil: 0.75,
            angriffMarge: 0.03,
            rueckzugAb: 0.8,
            lernen: 1.0,
            gelaende: 0.0,
            engMin: 6.0,
            engMax: 32.0,
            engBreite: 14.0,
            engTuerme: 3.0,
            engHinter: 2.0,
            zweitbasis: 0.0,
            zbMinWeg: 28.0,
            zbNutzen: 1.0,
            zbTuerme: 3.0,
            zbFabrik: 1.0,
            zbStruktur: 0.0,
            zbFeindAbstand: 1.2,
            engDringlichkeit: 60.0,
            engTurmFaktor: 2.0,
            engMauerBreite: 3.0,
            zbArmee: 1.0,
            zbAnzahl: 1.0,
            zbOffen: 0.0,
            zbBedrohung: 50.0,
            zbFeind: 1.0,
            zbErz: 1.0,
            zbArbeiter: 1.0,
            zbUeberschuss: 1.0,
            platzFern: 12.0,
            roehreKachel: 20.0,
            vorplan: 1.0,
            moralStill: 0.0,
            wehrlos: 1.0,
            welle: 0.0,
            paarung: 0.0,
            turmNetz: 0.0,
            turmOhneNetz: 0.55,
            wissArbeit: 0.0,
            wissAusbau: 0.0,
            forschungVorrang: 0.0,
            fabrikWirtschaft: 0.0,
            seltenNutzen: 0.0,
            fabrikLaeuft: 0.0,
            abwehrRueckkehr: 0.0,
            lueckeKlug: 0.0,
            turmRoehre: 0.0,
            mauerLinie: 0.0,
            mauerLaenge: 4.0,
            torBreite: 2.0,
            torAbstand: 10.0,
            bauReihe: 0.0,
            mauerSchub: 4.0,
            zugangMauer: 0.0,
            zmAbstand: 8.0,
            zmKlein: 3.0,
            zmMax: 14.0,
            zmHinter: 2.0,
            zmSchild: 5.0,
            fwAufruest: 11600.0,
            fwFahrt: 40.0,
            fwWert: 16.0,
            fwVorlauf: 6000.0,
            fwNach: 5000.0,
            fwBedrohung: 60.0,
            fwVorrang: 0.9,
            eroeffLabor: 80.0,
            eroeffAus: 0.0,
            eroeffBis: 25000.0,
            eroeffWarten: 3000.0,
            bauGleichzeitig: 1.0,
            wacheRand: 5.0,
            wacheMax: 30.0,
            forschKampf: 1.0,
            seltenOhne: 0.3,
            tempoWert: 0.0,
            geleit: 0.0,
            panzerFrueh: 0.0,
            panzerKlasse: 0.0,
            panzerModell: 0.0,
            zielTreue: 1.0,
            vorausBau: 0.0,
            schmelzeSpaet: 1.0,
            schmelzeEroeff: 0.0,
            marschHeil: 0.0,
            aufmarschFeind: 0.0,
            marschSammeln: 0.0,
            moralBau: 0.0,
            bauStau: 0.0,
            platzErreichbar: 0.0,
            schmelzeRisiko: 0.0,
            lkwWechsel: 0.0,
            sparFrist: 0.0,
            startBaken: 0.0,
            startSchmelze: 0.0,
            mineSeite: 0.0,
            kigaNachLynx: 0.0,
            eroeffFolge: 0.0,
            moralFest: 0.0,
            moralHalten: 0.0,
            missionsModus: 0.0,
            stufe: -1.0,
            rolle: 0.0,
            eigeneAngriffe: 1.0,
            schonfrist: 0.0,
            ueberlegMin: 1.0,
            testAngriff: 0.0,
            testPosten: 0.0,
            testPostenX: 0.0,
            testPostenY: 0.0,
            testPostenGroesse: 1.0,
            testBakeX: 0.0,
            testBakeY: 0.0,
            minenUmkreis: 0.0,
            minenJeSorte: 0.0,
            wacheVorBasis: 0.0,
            wachePatrouille: 0.0,
            wachePatrouilleTakt: 2000.0,
            bezirke: 0.0,
            gebietAuto: 0.0,
            kernLuft: 0.0,
            roehrenVorrang: 0.0,
            roehreAnNetz: 0.0,
            kigaBremse: 0.0,
            kigaAus: 20.0,
            kigaAn: 15.0,
            missionZiel: 0.0,
            raumhafen: 0.0,
            spinnen: 0.0,
            aufstieg: 0.0,
            mhd: 0.0,
            solar: 0.0,
            zbWache: 4.0,
            zbWacheDruck: 6.0,
            zbVorrang: 3000.0,
            testStaerke: 1.0,
            marschEtappe: 12.0,
            marschRadius: 6.0,
            marschAnteil: 0.75,
            sparBedrohung: 50.0,
            medVorrang: 0.0,
            bauAbbruch: 0.0,
            wohnVorrang: 0.0,
            faktorNeu: 0.0,
            laserTuerme: 0.0,
            laserTurmAnteil: 0.0,
            laserKeinAngriff: 0.0,
            panzerBis: 80000.0,
            geleitRadius: 8.0,
            geleitMax: 45.0,
            geleitUeber: 1.2,
            fabrikErz: 0.0,
            fabrikAuslastung: 1.0,
            schwachstelle: 0.0,
            schwachAussen: 0.0,
            zugang: 0.0,
            zugangGruppen: 1.0,
            truppPlan: 0.0,
            truppRisiko: 1.0,
            truppMin: 0.15,
            truppAbwehr: 0.0,
            versuch: 0.0,
            versuchWert: 1.0,
            wertungTick: 0.0,
            missionen: 0.0,
            missionHorizont: 20000.0,
            missionMax: 4.0,
            missionRueck: 0.6,
            missionUeberleg: 1.5,
            missionSieg: 60000.0,
            missionLernen: 0.0,
            missionKorrUeberfall: 1.0,
            missionKorrHalten: 1.0,
            missionKorrEntscheidung: 1.0,
            missionKorrAbwehr: 1.0,
            parallel: 0.0,
            parallelUmweg: 1.4,
            parallelAbstand: 8.0,
            parallelTiefe: 3.0,
            moralWert: 1.0,
            erzAusbau: 1.0,
            turmVorne: 3.0,
            turmAbstand: 2.0,
            turmFront: 2.0,
            turmUeberMauer: 1.0,
            gefecht: 1.0,
            gefechtRadius: 10.0,
            gefechtSpiel: 2.0,
            schaumAnteil: 0.0,
            gefechtStellung: 1.0,
            gefechtSperre: 40.0,
            gefechtAbstand: 1.0,
            gefechtAbstandSperre: 12.0,
            gefechtReichweitePlus: 1.0,
            gefechtBedrohungWert: 1.0,
            gefechtTurmGewicht: 1.0,
            gefechtAusweich: 2.0,
            gefechtBesser: 0.25,
            gefechtRueckzugSchaden: 0.5,
            gefechtZielStrafe: 200.0,
            gefechtStuetzeStrafe: 30.0,
            gefechtWeg: 1.0,
            vorplanWachstum: 0.5,
            vorplanRadius: 14.0,
            vorplanDicht: 8.0,
            kampfModell: 1.0,
            sonde: 0.0,

        }
    }
}
