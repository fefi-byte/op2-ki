//! Hauptablauf: start() beim Missionsstart, schritt() je TAKT (Reihenfolge aller Bereiche), Buchfuehrung (Ziel, Verluste, Sieg, Zufluss).
use super::*;
use super::super::*;
use super::{WERTUNG_TICK, SCAN_TAKT, B_ANZAHL, GEBAEUDE, OreTypeCommon, Bar3, Bar2, VariantRandom, KI_MESSUNG};

/// C++: void start(Zustand& z, int spieler, LOCATION ursprung, int breite, int hoehe, int x, int y, bool ausruestung, int ziel = 1, int startStufe = 0, int zi  (op2_ki.hpp:7016)
// Einmal in InitProc. `ursprung` = Engine-Koordinate der Editor-Kachel (0,0), z.B. MkXY(1, 1).
// `ziel`: 1 = EDWARD-Satellit ins All schicken (Standard), 0 = nur die Kolonie aufbauen.
// `startStufe`: Umfang der Startausruestung (siehe startausruestung()).
pub fn start(welt: &mut dyn Welt, z: &mut Zustand, spieler: i32, ursprung: Loc, breite: i32, hoehe: i32, x: i32, y: i32, ausruestung: bool, ziel: i32, startStufe: i32, zielWert: i32) {
    z.plaetze = [Platz::default(); 48];
    z.roehren = [Roehre::default(); 48];
    z.aktiv = 1; z.spieler = spieler;
    super::super::modus::zuruecksetzen(z);
    super::super::befehl::zuruecksetzen(z);
    parameterLaden(welt, z);                             // Stellschrauben (Standard oder KI-Parameter-P<n>.txt)
    super::super::modus::anwenden(welt, z);              // Missionsmodus: Stufe und Rolle auf den Parametersatz
    z.ux = ursprung.x; z.uy = ursprung.y;
    z.breite = breite; z.hoehe = hoehe;
    z.bx = x; z.by = y;
    z.naechsterSchritt = 0; z.naechsteSpur = 0;
    // DoResearch nimmt den Tabellenindex. DoResearch takes the table index.
    z.forschungModus = 1; z.forschungLab = 0;
    z.kitTyp = 0; z.fzTyp = 0; z.anzahlPlaetze = 0; z.anzahlRoehren = 0;
    z.kitPauseBis = 0; z.fzPauseBis = 0;
    for i in 0..8 { z.kitGesperrtTyp[i] = 0; z.kitGesperrtBis[i] = 0; }
    for i in 0..32 { z.meldungenSeit[i] = 0; }
    for i in 0..MAX_MINEN { z.minenId[i] = 0; z.schmelzeId[i] = 0; }
    z.bakeZiel = 0; z.bakeMiner = 0; z.bakeSeit = 0; z.minerSeit = 0; z.anzahlBaken = 0;
    z.minerWeg = 1; z.minenGesehen = 0; z.minenBauAn = 0;
    z.kbN = 0; z.kbStand = 0;
    for i in 0..8 { z.sperrBake[i] = 0; z.sperrBis[i] = 0; }
    z.ladeConvec = 0; z.ladeSeit = 0;
    z.ziel = ziel; z.zielErreicht = 0;
    // Missionsmodus: Ziel aus der Mission (Parameter missionZiel, spaeter Editor-Befehl), z.B. 1 Satellit -> Raumhafen
    if PAR(spieler).missionZiel > 0.0 { z.ziel = PAR(spieler).missionZiel as i32; }
    z.raumSchritt = 0; z.raumSeit = 0; z.raumVersuche = 0; z.raumAuftrag = 0;
    for i in 0..2 { z.seltenMineId[i] = 0; z.seltenSchmelzeId[i] = 0; }
    z.seltenBake = 0; z.seltenBakeSeit = 0; z.seltenMiner = 0; z.seltenMinerSeit = 0;
    z.bauAn = 0; z.naechsterAnschluss = 0;
    z.anzahlGeparkt = 0; z.naechsteArbeit = 0; z.anzahlLeer = 0;
    z.techGemeldet = [0; 512]; z.naechsteTechPruefung = 0;
    z.moralFrei = 0; z.naechsteVerteilung = 0; z.fabrikStill = 0;
    for i in 0..MAX_VF { z.fabrikStillW[i] = 0; z.fabrikDrin[i] = 0; }
    z.wunschArbeiter = 0; z.wunschRang = 0; z.wunschSeit = 0;
    z.naechsterScan = 0; z.feindArmee = 0; z.feindNah = 0; z.feindGeb = 0; z.feindX = 0; z.feindY = 0;
    z.feindBestWert = 0; z.feindBestTyp = 0; z.feindBestWaffe = 0;
    for i in 0..4 { z.forschGestartet[i] = 0; }
    z.empRakete = 0; z.empAuftrag = 0; z.empAuftragSeit = 0; z.empStart = 0; z.empZielX = 0; z.empZielY = 0;
    z.aufmarschX = 0; z.aufmarschY = 0; z.aufmarschSeit = 0;
    z.feindFabriken = 0; z.feindWT = 0; z.eigeneArmee = 0; z.sollArmee = 0; z.bedrohung = 0;
    z.letzterAngriff = 0; z.angriffe = 0; z.feindGemeldet = 0; z.wacheAn = 0; z.wacheAnzahl = 0;
    z.staerksterAngriff = 0; z.eigeneGeb = 0;
    z.feindPosN = 0; z.feindMinD2 = 1 << 30; z.feindNX = 0; z.feindNY = 0; z.eigeneMobil = 0; z.eigeneWTWert = 0;
    z.kampfModus = 0; z.kampfModusSeit = 0; z.angriffStart = 0;
    z.angriffSektoren = 0; z.naechsteVorhersage = 0;
    z.messErz = 0; z.messSelten = 0; z.messTick = 0; z.zuflussErz = 0.0; z.zuflussSelten = 0.0;
    for i in 0..8 { z.wellen[i] = 0; }
    z.anzWellen = 0; z.welleSpitze = 0; z.erzUeberschuss = 0; z.welleStart = 0; z.wellenAbstand = 0;
    z.feindUnterwegs = 0; z.feindWachstum = 0.0; z.feindArmeeVorher = 0; z.turmAnteil = PAR(z.spieler).turmAnteil; z.reserve = PAR(z.spieler).reserveStart; z.welleEigenStart = 0; z.welleEigenMin = 0; z.laufNr = 0; z.naechstesSpeichern = 0;
    z.erzJeRouteGelernt = 0.0; z.verlustSumme = 0; z.maxBew = 0; z.ueberstanden = 0; z.feindVernichtet = 0;
    z.moralSumme = 0.0; z.moralN = 0;
    z.zweiteSchmelzeTick = 0; z.lynxFreiSeit = 0; z.siegTick = 0; z.niederlageTick = 0; z.verlustGebaeude = 0; z.verlustEinheiten = 0; z.feindHatteBasis = 0;
    z.anzFeindId = 0; z.anzEigenId = 0; z.zerstoertWert = 0; z.verlorenWert = 0; z.bewertet = 0; z.zerstoertHeim = 0; z.verlorenHeim = 0;
    z.laborWunsch = 0; z.laborWert = 0; z.laborFuer = 0; z.naechsterLaborCheck = 0;
    for i in 0..4 { z.laborNutzen[i] = 0; }
    z.letztesKampfFz = 0; z.letzterBau = 0; z.kampfBedarfSeit = 0; z.bauDrangSeit = 0; z.naechstePruefung = 0; z.kampfLaeuft = 0;
    z.sammelAn = 0; z.angriffAn = 0; z.fabrikAn = 0; z.fabrikVf = 0;
    z.angriff2An = 0; z.reserveAn = 0; z.reserveZiel = 0; z.reserveOrt = -2; z.reserveSeit = 0;
    z.zielFz = 0; z.zielWaffe = 0; z.wacheZiel = 0; z.sammelZiel = 0; z.gruppenBedarf = 0; z.kampfFzAnzahl = 0;
    z.angriffArt = 0; z.angriffWunsch = 0; z.angriffZerstoertStart = 0; z.angriffVerlorenStart = 0; z.letzterAngriffStart = 0;
    z.bilStart = 0; z.bilN = 0; z.bilFN = 0; z.bilRueck = 0; z.bilRueckErz = 0; z.jagdZiel = 0; z.jagdSeit = 0; z.jagdTakt = 0;
    z.zielEmp = 0; z.bilUeber = 0; z.spinneTakt = 0; for i in 0..8 { z.spinneZiel[i] = 0; z.spinneId[i] = 0; }
    z.garDran = 0; z.garDranSeit = 0; z.garRaus = 0; z.garRausSeit = 0; z.garZaehl = 0; for i in 0..6 { z.garBucht[i] = 0; z.garFrei[i] = 0; }
    z.bilE = 0; z.bilD = 0; z.bilArmee = 0; z.bilKontakt = 0; z.bilErste = 0; z.bilNach = -1; z.bilKTick = 0;
    for i in 0..6 { z.bilK[i] = -1; } for i in 0..3 { z.bilK2[i] = -1; } z.bilGruppe = 0; z.bilZerHeim = 0; z.bilVerHeim = 0;
    for i in 0..3 { z.angriffQuote[i] = 1.0; }
    z.zermuerbAbstand = PAR(z.spieler).zermuerbAbstand as i32; z.angriffZiel = 0; z.feindPotenzial = 0;
    for i in 0..6 { z.aussenId[i] = 0; z.aussenX[i] = 0; z.aussenY[i] = 0; z.aussenAngriff[i] = 0; z.aussenAn[i] = 0; }
    z.startErz = welt.Ore(spieler); z.startSelten = welt.RareOre(spieler);
    let mut p = Puffer::default();
    p.f(&format!("{{\"t\":{},\"art\":\"start\",\"p\":{},\"breite\":{},\"hoehe\":{},\"bx\":{},\"by\":{},\"ux\":{},\"uy\":{},\"eden\":{},\"ziel\":{},\"startStufe\":{}}}",
        welt.Tick(), spieler, breite, hoehe, x, y, z.ux, z.uy, if welt.IsEden(spieler) { 1 } else { 0 }, ziel, startStufe));
    p.schreiben(welt);
    z.zielWert = if zielWert > 0 { zielWert } else { 500 };
    if PAR(z.spieler).lernen != 0.0 { wissenLaden(welt, z); }
    else { melde(welt, z, "lernen", mapNone, z.bx, z.by, "Langzeit-Wissen aus (lernen=0) -- Lauf mit festen Parametern"); }
    if ausruestung {
        startausruestung(welt, z, x, y, startStufe);
        // PAR startBaken (2026-11-21): die Baken vor der Startbasis erfassen -- sonst kennt platzFrei sie noch nicht und setzt ein
        // Gebaeude auf den Minenplatz (ml2_31 Platz 1: Robo-Miner kam nie an die Bake, keine Mine, Platz 1 verlor 16 von 16)
        if startStufe >= 3 && PAR(z.spieler).startBaken >= 0.5 { super::erz::bakenErfassen(welt, z, 0); }
        if startStufe >= 3 { basisAufbauen(welt, z); }
        bakenSichern(welt, z);
    }
}

/// C++: void messung(Zustand& z)  (op2_ki.hpp:7103)
// Nur im Messbuild (-DKI_MESSUNG): ein fertiges Agridome bei Tick ~9000 stilllegen (DoIdle) und bei
// ~10600 wieder einschalten (DoUnIdle).
pub fn messung(welt: &mut dyn Welt, z: &mut Zustand) {
    let mut id = MESSUNG_ID.with(|s| *s.borrow());
    let t = welt.Tick();
    if id == 0 && t >= 9000 {
        for u in welt.PlayerBuildingEnum(z.spieler, mapAgridome) {
            if welt.GetCurAction(u) != moDone || welt.IsBusy(u) { continue; }
            id = u;
            MESSUNG_ID.with(|s| *s.borrow_mut() = id);
            let lo = welt.Location(u);
            let (lx, ly) = (edX(welt, z, lo), edY(welt, z, lo));
            let txt = format!("vor DoIdle: Aktion {}, Strom {}, Arbeiter {}, stillgelegt {}",
                welt.GetCurAction(u), if welt.HasPower(u) { 1 } else { 0 }, if welt.HasWorkers(u) { 1 } else { 0 }, welt.GetNumIdleBuildings(z.spieler));
            melde(welt, z, "messung", mapAgridome, lx, ly, &txt);
            welt.DoIdle(u);
            break;
        }
        return;
    }
    if id == 0 { return; }
    let u: Id = id;
    if !welt.IsLive(u) { return; }
    if t % 200 == 3 || (t >= 10600 && t < 10620) {
        let lo = welt.Location(u);
        let (lx, ly) = (edX(welt, z, lo), edY(welt, z, lo));
        let txt = format!("Agridome {}: Aktion {}, beschaeftigt {}, Strom {}, Arbeiter {}, stillgelegt {}, Arbeiter frei {}",
            id, welt.GetCurAction(u), if welt.IsBusy(u) { 1 } else { 0 }, if welt.HasPower(u) { 1 } else { 0 }, if welt.HasWorkers(u) { 1 } else { 0 },
            welt.GetNumIdleBuildings(z.spieler), welt.GetNumAvailableWorkers(z.spieler));
        melde(welt, z, "messung", mapAgridome, lx, ly, &txt);
    }
    if t >= 10600 && t < 10620 {
        welt.DoUnIdle(u);
        let lo = welt.Location(u);
        let (lx, ly) = (edX(welt, z, lo), edY(welt, z, lo));
        melde(welt, z, "messung", mapAgridome, lx, ly, "DoUnIdle gegeben");
    }
}

/// C++: void schritt(Zustand& z)  (op2_ki.hpp:7139)
// Jeden AIProc-Aufruf; arbeitet intern im Takt. (Der Hook fremdSchritt entfaellt hier.)
/// Ein KI-Schritt (alle TAKT Ticks): Lage erheben, Gegner scannen, Buchfuehrung, dann die Bereiche der Reihe nach --
/// Arbeit, Fahrzeuge, Kampf, Garage, Reparatur, Erz, Bebauungsplan, Roehren, Forschung, Ausbildung, Raumfahrt, Spur.
/// Rust 2026-11-18: Buchfuehrung in eigene Funktionen (Verhalten gleich).
pub fn schritt(welt: &mut dyn Welt, z: &mut Zustand) {
    if z.aktiv == 0 { return; }
    let t = welt.Tick();
    // Gefechtsfuehrung (Rust): jeder AIProc-Aufruf, solange gekaempft wird
    g_kraftSpieler.with(|v| v.set(z.spieler));
    super::gefecht::fuehren(welt, z);
    if t < z.naechsterSchritt { return; }
    z.naechsterSchritt = t + TAKT;
    hinweis(welt, z, "ki::anfang");
    g_kraftSpieler.with(|v| v.set(z.spieler));
    // Moral freigeben: der Codegen setzt am Ende von InitProc ForceMoraleGood fuer alle Spieler.
    moralFreigeben(welt, z);
    let mut l = Lage::default();
    hinweis(welt, z, "ki::erhebe"); erhebe(welt, z, &mut l);
    super::erweiterung::weltbild(welt, z, &l);   // Messung (Rust), einmal je Spieler beim ersten Schritt
    startvorratEinlagern(welt, z, &mut l);
    hinweis(welt, z, "ki::karteScannen"); karteScannen(welt, z);          // Gegner-Lagebild (alle SCAN_TAKT Ticks)
    zielPruefen(welt, z, &l, t);
    statistik(welt, z, &l, t);
    verlusteUndSieg(welt, z, t);
    wertungUndSpeichern(welt, z, t);
    zuflussMessen(welt, z, &l, t);
    hinweis(welt, z, "ki::arbeitErheben"); arbeitErheben(welt, z, &mut l);      // nach dem letzten erhebe(): Arbeiter-Rangliste und Vormerkungen
    z.erzUeberschuss = if bilanz(welt, z, &l).engpassErz <= 0 { 1 } else { 0 };
    // Ohne Kommandozentrale: der ConVec mit dem CC-Bausatz baut auf dem Mittelplatz.
    super::bau::vorratPruefen(welt, z);   // Bausaetze zu Spielbeginn? vor dem ersten Bauauftrag
    hinweis(welt, z, "ki::bauGruppe"); bauGruppe(welt, z, &l);
    hinweis(welt, z, "ki::vorrat"); super::bau::vorratAufbauen(welt, z);   // Bausaetze, die schon in den Buchten liegen
    hinweis(welt, z, "ki::arbeit"); arbeit(welt, z, &l);            // Arbeiter-Rangliste: stilllegen / einschalten
    if t % 200 < TAKT { hinweis(welt, z, "ki::leerlauf"); leerlauf(welt, z, &l); }   // untaetige Gebaeude bei Personalmangel stilllegen
    hinweis(welt, z, "ki::fahrzeuge"); fahrzeuge(welt, z, &l);
    hinweis(welt, z, "ki::kampfFuehrung"); kampfFuehrung(welt, z, &l);
    super::kampf::kamera(welt, z);
    hinweis(welt, z, "ki::empRakete"); empRaketeBauen(welt, z, &l);     // Plymouth: EMP-Rakete im Raumhafen
    hinweis(welt, z, "ki::garage"); garageAblauf(welt, z, &l);
    hinweis(welt, z, "ki::reparatur"); reparatur(welt, z, &l);          // beschaedigte Gebaeude reparieren
    hinweis(welt, z, "ki::seltenErz"); seltenErz(welt, z, &l);          // vor erz(): reserviert den Robo-Miner fuer Seltenerz
    hinweis(welt, z, "ki::erz"); erz(welt, z, &l);
    super::zweitbasis::schritt(welt, z, &l);   // Erweiterung (Rust), nur mit PAR.zweitbasis
    super::erweiterung::passMauern(welt, z);
    if !super::befehl::drehbuch(z) { super::mauer::zugangSchritt(welt, z); }   // Rust, nur mit PAR.zugangMauer   // Erweiterung (Rust), nur mit PAR.gelaende
    hinweis(welt, z, "ki::vorplan"); super::vorplan::planen(welt, z, &l);   // Bebauungsplan (Rust)
    hinweis(welt, z, "ki::anschluss"); anschluss(welt, z, &l);         // Roehren ueber die BuildingGroup
    super::bezirke::roehrenSchritt(welt, z);   // Missionsmodus (PAR bezirke): Roehrenachsen zu den Bezirken
    if !super::befehl::drehbuch(z) { super::bezirke::festungSchritt(welt, z); }   // Missionsmodus: Tuerme der Raumhafen-Festung
    if !super::befehl::drehbuch(z) { super::zweitbasis::spinnenSchritt(welt, z); }   // Missionsmodus, Plymouth: Spinnen je Basis
    super::befehl::schritt(welt, z, &l);   // Befehle der Mission: Fahrzeuge der Phase, Phase fertig, Angriffsstand
    hinweis(welt, z, "ki::vorhersageMelden"); vorhersageMelden(welt, z, &l);   // Techbaum-Vorhersage in die Spur (alle 10000 Ticks)
    hinweis(welt, z, "ki::laborBedarf"); laborBedarf(welt, z, &l);        // fehlt der Forschung ein Labor? (alle 1000 Ticks)
    hinweis(welt, z, "ki::selbstpruefung"); selbstpruefung(welt, z, &l);   // Widersprueche erkennen (alle 2000 Ticks)
    hinweis(welt, z, "ki::forschung"); forschung(welt, z, &l);
    hinweis(welt, z, "ki::ausbildung"); ausbildung(welt, z, &l);        // Uni: Wissenschaftler ausbilden
    hinweis(welt, z, "ki::wissenschaftlerVerteilen"); wissenschaftlerVerteilen(welt, z, &l);   // laufende Forschungen
    hinweis(welt, z, "ki::raumfahrt"); raumfahrt(welt, z, &l); solarSatellit(welt, z, &l);
    if t >= z.naechsteSpur {
        z.naechsteSpur = t + SPUR_TAKT;
        hinweis(welt, z, "ki::spur"); spur(welt, z, &l);
        if welt.Tick() % 1000 < SPUR_TAKT { hinweis(welt, z, "ki::lkwDiagnose"); lkwDiagnose(welt, z); }
    }
    if KI_MESSUNG { messung(welt, z); }
    hinweis(welt, z, "ende");
    // op2::crash::g_hinweis = nullptr entfaellt
}

/// Moral freigeben: der Codegen setzt am Ende von InitProc ForceMoraleGood fuer alle Spieler
pub(super) fn moralFreigeben(welt: &mut dyn Welt, z: &mut Zustand) {
    // PAR moralFest: die feste Moral der Mission (ForceMoraleGood, 75) bleibt -- wie die Mehrspieler-Option "stabile Moral"
    if z.moralFrei == 0 && (super::planung::moralFestAktiv(welt, z) || PAR(z.spieler).moralHalten >= 0.5) { z.moralFrei = 1; melde(welt, z, "info", mapNone, z.bx, z.by, "Moral bleibt fest auf 75 (stabile Moral wie im Mehrspieler)"); return; }
    if z.moralFrei == 0 {
        welt.FreeMoraleLevel(z.spieler);
        z.moralFrei = 1;
        let ml = welt.MoraleLevel(z.spieler);
        melde(welt, z, "info", mapNone, z.bx, z.by, &format!("Moral freigegeben (FreeMoraleLevel), Stufe jetzt {}", ml));
        // (tempo::faktor-Meldung entfaellt)
    }
}

/// Startvorrat einlagern, sobald es Lagerplatz gibt (siehe Zustand::startErz); danach Lage neu erheben
pub(super) fn startvorratEinlagern(welt: &mut dyn Welt, z: &mut Zustand, l: &mut Lage) {
    if z.startErz > 0 && l.erzMax > 0 {
        let neu = if z.startErz < l.erzMax { z.startErz } else { l.erzMax };
        if neu > l.erz { welt.SetOre(z.spieler, neu); }
        melde(welt, z, "info", mapCommonOreSmelter, z.bx, z.by, &format!("Startvorrat eingelagert: {} Erz (Lager {})", neu, l.erzMax));
        z.startErz = 0;
        hinweis(welt, z, "ki::erhebe"); erhebe(welt, z, l);
    }
    if z.startSelten > 0 && l.seltenMax > 0 {
        let neu = if z.startSelten < l.seltenMax { z.startSelten } else { l.seltenMax };
        if neu > l.selten { welt.SetRareOre(z.spieler, neu); }
        melde(welt, z, "info", mapRareOreSmelter, z.bx, z.by, &format!("Startvorrat eingelagert: {} Seltenerz (Lager {})", neu, l.seltenMax));
        z.startSelten = 0;
        hinweis(welt, z, "ki::erhebe"); erhebe(welt, z, l);
    }
}

/// Bevoelkerungsziel bzw. Gegner vernichtet: Ziel erreicht
pub(super) fn zielPruefen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32) {
    if z.zielErreicht == 0 && z.ziel == 2 && z.zielWert > 0 && l.menschen >= z.zielWert {
        z.zielErreicht = t;
        melde(welt, z, "ziel", mapResidence, z.bx, z.by, &format!("ZIEL ERREICHT: {} Einwohner (Ziel {})", l.menschen, z.zielWert));
        welt.logZeile(&format!("Aufbau-KI Spieler {}: ZIEL ERREICHT -- {} Einwohner (Tick {})", z.spieler, l.menschen, t));
    }
    if z.zielErreicht == 0 && z.ziel == 3 && z.feindGemeldet != 0 && z.feindGeb == 0 {
        z.zielErreicht = t;
        melde(welt, z, "ziel", mapNone, z.bx, z.by, "ZIEL ERREICHT: Gegner vernichtet");
        welt.logZeile(&format!("Aufbau-KI Spieler {}: ZIEL ERREICHT -- Gegner vernichtet (Tick {})", z.spieler, t));
    }
}

/// Einmal je 200 Ticks: hoechste Bevoelkerung, zweite Schmelze, Moral-Mittel, Feind vernichtet
pub(super) fn statistik(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32) {
    if t % 200 < TAKT {                                    // einmal je 200 Ticks (Versatz-unabhaengig, Duell 8)
        if l.menschen > z.maxBew { z.maxBew = l.menschen; }
        if z.zweiteSchmelzeTick == 0 && l.geb[mapCommonOreSmelter as usize] >= 2 { z.zweiteSchmelzeTick = t; }
        z.moralSumme += moralZahl(welt, z) as f64; z.moralN += 1;
        if z.feindVernichtet == 0 && z.feindGemeldet != 0 && z.feindGeb == 0 { z.feindVernichtet = t; }
    }
}

/// Verluste zaehlen (auch nach der Wertung: die Angriffsquoten lernen daraus); Sieg/Niederlage, wenn Kommandozentrale und
/// Strukturfabrik weg sind
pub(super) fn verlusteUndSieg(welt: &mut dyn Welt, z: &mut Zustand, t: i32) {
    if t % SCAN_TAKT < TAKT {
        hinweis(welt, z, "ki::verlusteZaehlen");
        // Abweichung (Borrow): z als Kopie fuer die Lesezugriffe (spieler, bx, by, ux, uy -- werden hier nicht veraendert)
        let zk: Zustand = *z;
        verlusteZaehlen(welt, &mut z.feindId, &mut z.feindWert, &mut z.feindPos, &mut z.anzFeindId, &mut z.zerstoertWert, &mut z.zerstoertHeim, true, &zk, None, None);
        let zk: Zustand = *z;
        verlusteZaehlen(welt, &mut z.eigenId, &mut z.eigenWert, &mut z.eigenPos, &mut z.anzEigenId, &mut z.verlorenWert, &mut z.verlorenHeim, false, &zk,
            Some(&mut z.verlustGebaeude), Some(&mut z.verlustEinheiten));
        // Sieg/Niederlage: Kommandozentrale und Strukturfabrik weg. Einmal gesetzt, bleibt es.
        if t <= wertungTick(z.spieler) {
            let mut feindBasis = false;
            let mut eigeneBasis = false;
            for q in 0..welt.NoPlayers() {
                let f = q != z.spieler && !welt.IsAlliedTo(z.spieler, q);
                if !f && q != z.spieler { continue; }
                let mut da = false;
                for u in welt.PlayerBuildingEnum(q, mapCommandCenter) { if welt.IsLive(u) { da = true; } }
                for u in welt.PlayerBuildingEnum(q, mapStructureFactory) { if welt.IsLive(u) { da = true; } }
                if f { feindBasis = feindBasis || da; } else { eigeneBasis = da; }
            }
            if feindBasis { z.feindHatteBasis = 1; }
            if z.siegTick == 0 && z.niederlageTick == 0 && z.feindHatteBasis != 0 && !feindBasis {
                z.siegTick = t; ergebnisSchreiben(welt, z); melde(welt, z, "lernen", mapNone, z.bx, z.by, &format!("SIEG bei Tick {}", t));
            }
            if z.siegTick == 0 && z.niederlageTick == 0 && t > 500 && !eigeneBasis {
                z.niederlageTick = t; ergebnisSchreiben(welt, z); melde(welt, z, "lernen", mapNone, z.bx, z.by, &format!("NIEDERLAGE bei Tick {}", t));
            }
        }
    }
}

/// Wertung bei WERTUNG_TICK, Wissen alle 5000 Ticks speichern
pub(super) fn wertungUndSpeichern(welt: &mut dyn Welt, z: &mut Zustand, t: i32) {
    if z.bewertet == 0 && t >= wertungTick(z.spieler) {
        z.bewertet = 1;
        let lp = laufPunkte(welt, z);
        melde(welt, z, "lernen", mapNone, z.bx, z.by, &format!("Wertung bei Tick {}: {} Punkte (Ziel {} {}, zerstoert {}, verloren {}, hoechstens {} Einwohner)",
            wertungTick(z.spieler), lp, z.ziel, if z.zielErreicht != 0 { "erreicht" } else { "offen" }, z.zerstoertWert, z.verlorenWert, z.maxBew));
        hinweis(welt, z, "ki::wissenSpeichern"); wissenSpeichern(welt, z);
    }
    if t >= z.naechstesSpeichern && z.bewertet == 0 { hinweis(welt, z, "ki::wissenSpeichern"); wissenSpeichern(welt, z); z.naechstesSpeichern = t + 5000; }
}

/// Zufluss messen: Netto-Aenderung des Vorrats je 1000 Ticks, gleitend gemittelt. Bei vollem Lager zaehlt die Messung nicht.
pub(super) fn zuflussMessen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32) {
    if t - z.messTick >= 200 {
        if z.messTick != 0 {
            let f = 1000.0 / (t - z.messTick) as f64;
            let de = l.erz - z.messErz;
            let ds = l.selten - z.messSelten;
            if l.erz < l.erzMax { z.zuflussErz = z.zuflussErz * 0.95 + de as f64 * f * 0.05; }
            if l.selten < l.seltenMax { z.zuflussSelten = z.zuflussSelten * 0.95 + ds as f64 * f * 0.05; }
        }
        z.messErz = l.erz; z.messSelten = l.selten; z.messTick = t;
    }
}
