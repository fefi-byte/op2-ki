//! Wissen und Parameter: Parametertext lesen (parameterLaden), Ergebnis/Punkte, Verluste zaehlen, Langzeitwissen laden/speichern.
use super::*;
use super::super::*;
use super::{WERTUNG_TICK, SCAN_TAKT, B_ANZAHL, GEBAEUDE, OreTypeCommon, Bar3, Bar2, VariantRandom, KI_MESSUNG};

/// C++: void wissenPfad(char* out, int n, const char* datei)  (op2_ki.hpp:6762)
// Abweichung: der Ordner (op2::log::kiPath) kommt aus welt.dateiLesen/dateiSchreiben -- hier nur der Dateiname.
pub fn wissenPfad(welt: &mut dyn Welt, out: &mut String, n: i32, datei: &str) {
    *out = datei.to_string();
}

/// C++: void parameterLaden(Zustand& z)  (op2_ki.hpp:6772)
// Parameter aus logs/KI-Parameter-P<n>.txt (name=wert je Zeile, # = Kommentar). Unbekannte Namen werden gemeldet.
/// Spielstand der oeffentlichen Fassung: "name=wert" je Zeile, "eden:"/"ply:" nur fuer die Fraktion
const SPIELSTAND: &str = "lernen=0\nreaktion=15000.0000\nhorizont=9233.9408\nueberlegenheit=1.2702\nfabrikGewinn=0.8052\nreserveStart=0.8000\nreserveLernrate=0.4760\nreserveRueckgang=1.0000\nreserveMax=4.0000\nturmAnteil=0.8000\nzermuerbAbstand=2000.0000\nschmelzeNutzen=1.4966\nwohnraumGrenze=98.0000\nfreischaltWert=0.5160\nnahRadius=40.0000\nheimHalten=1.6362\nsteigerung=1.5000\nvorsorge=57.4926\nschmelzenJeMine=2.0000\ngefecht=1.5\npaarung=1\nturmNetz=1\nwissArbeit=1\nfabrikErz=1\nforschungVorrang=1\nfabrikWirtschaft=1\nseltenNutzen=1\nfabrikLaeuft=1\nabwehrRueckkehr=1\nzweitbasis=1\nparallel=0\nturmRoehre=1\nbauReihe=1\nforschKampf=10\neden:tf_5401=0.3\ntempoWert=1\npanzerKlasse=1\neden:panzerFrueh=3\nmedVorrang=1\nbauAbbruch=8000\neden:sparBedrohung=200\nvorausBau=1\nzielTreue=1.5\nschmelzeEroeff=1\ngelaende=1\nmarschSammeln=1\nwohnVorrang=3\nstartBaken=1\nmineSeite=1\neroeffFolge=1\n";

pub fn parameterLaden(welt: &mut dyn Welt, z: &mut Zustand) {
    let sp = (z.spieler & 7) as usize;
    let mut q = Parameter::default();
    let speichern = |q: &Parameter| par.with(|p| p.borrow_mut()[sp] = *q);
    speichern(&q);
    techFaktoren.with(|t| t.borrow_mut()[sp].clear());
    let mut datei = format!("KI-Parameter-P{}.txt", z.spieler);
    let mut pfad = String::new();
    wissenPfad(welt, &mut pfad, 600, &datei);
    let f = welt.dateiLesen(&pfad);
    // Spielstand (oeffentliche Fassung): die gelernten Grundwerte und die Standard-Stellschrauben der Entwicklung
    // (daten/mission_param.txt + werkzeug/schleife/standard.txt) kommen zuerst -- Mission und Datei ueberschreiben einzelne Werte
    let text = parameterText.with(|t| t.borrow()[sp].clone());
    let text = Some(format!("{}{}", SPIELSTAND, text.unwrap_or_default()));
    if f.is_none() && text.is_none() { return; }
    fn feld<'a>(q: &'a mut Parameter, name: &str) -> Option<&'a mut f64> {
        Some(match name {
            "reaktion" => &mut q.reaktion, "horizont" => &mut q.horizont, "ueberlegenheit" => &mut q.ueberlegenheit,
            "fabrikGewinn" => &mut q.fabrikGewinn, "reserveStart" => &mut q.reserveStart, "reserveLernrate" => &mut q.reserveLernrate,
            "reserveRueckgang" => &mut q.reserveRueckgang, "reserveMax" => &mut q.reserveMax, "turmAnteil" => &mut q.turmAnteil,
            "zermuerbAbstand" => &mut q.zermuerbAbstand, "schmelzeNutzen" => &mut q.schmelzeNutzen, "wohnraumGrenze" => &mut q.wohnraumGrenze,
            "freischaltWert" => &mut q.freischaltWert,
            "nahRadius" => &mut q.nahRadius, "heimHalten" => &mut q.heimHalten, "steigerung" => &mut q.steigerung, "vorsorge" => &mut q.vorsorge,
            "seltenGewinn" => &mut q.seltenGewinn, "schmelzenJeMine" => &mut q.schmelzenJeMine, "empSchwelle" => &mut q.empSchwelle,
            "risikoAnteil" => &mut q.risikoAnteil, "angriffMarge" => &mut q.angriffMarge, "rueckzugAb" => &mut q.rueckzugAb,
            "lernen" => &mut q.lernen,
            "gelaende" => &mut q.gelaende,
            "engMin" => &mut q.engMin,
            "engMax" => &mut q.engMax,
            "engBreite" => &mut q.engBreite,
            "engTuerme" => &mut q.engTuerme,
            "engHinter" => &mut q.engHinter,
            "zweitbasis" => &mut q.zweitbasis,
            "zbMinWeg" => &mut q.zbMinWeg,
            "zbNutzen" => &mut q.zbNutzen,
            "zbTuerme" => &mut q.zbTuerme,
            "zbFabrik" => &mut q.zbFabrik,
            "zbStruktur" => &mut q.zbStruktur,
            "zbFeindAbstand" => &mut q.zbFeindAbstand,
            "engDringlichkeit" => &mut q.engDringlichkeit,
            "engTurmFaktor" => &mut q.engTurmFaktor,
            "engMauerBreite" => &mut q.engMauerBreite,
            "zbArmee" => &mut q.zbArmee, "zbAnzahl" => &mut q.zbAnzahl, "zbOffen" => &mut q.zbOffen, "zbBedrohung" => &mut q.zbBedrohung, "zbFeind" => &mut q.zbFeind, "zbErz" => &mut q.zbErz, "zbArbeiter" => &mut q.zbArbeiter, "zbUeberschuss" => &mut q.zbUeberschuss,
            "platzFern" => &mut q.platzFern,
            "roehreKachel" => &mut q.roehreKachel,
            "vorplan" => &mut q.vorplan, "moralStill" => &mut q.moralStill, "wehrlos" => &mut q.wehrlos, "welle" => &mut q.welle, "paarung" => &mut q.paarung, "turmNetz" => &mut q.turmNetz, "turmOhneNetz" => &mut q.turmOhneNetz, "wissArbeit" => &mut q.wissArbeit, "wissAusbau" => &mut q.wissAusbau, "forschungVorrang" => &mut q.forschungVorrang, "fabrikWirtschaft" => &mut q.fabrikWirtschaft, "seltenNutzen" => &mut q.seltenNutzen, "fabrikLaeuft" => &mut q.fabrikLaeuft, "abwehrRueckkehr" => &mut q.abwehrRueckkehr, "lueckeKlug" => &mut q.lueckeKlug, "turmRoehre" => &mut q.turmRoehre, "mauerLinie" => &mut q.mauerLinie, "mauerLaenge" => &mut q.mauerLaenge, "torBreite" => &mut q.torBreite, "torAbstand" => &mut q.torAbstand, "bauReihe" => &mut q.bauReihe, "mauerSchub" => &mut q.mauerSchub, "zugangMauer" => &mut q.zugangMauer, "zmAbstand" => &mut q.zmAbstand, "zmKlein" => &mut q.zmKlein, "zmMax" => &mut q.zmMax, "zmHinter" => &mut q.zmHinter, "zmSchild" => &mut q.zmSchild, "fwAufruest" => &mut q.fwAufruest, "fwFahrt" => &mut q.fwFahrt, "fwWert" => &mut q.fwWert, "fwVorlauf" => &mut q.fwVorlauf, "fwNach" => &mut q.fwNach, "fwBedrohung" => &mut q.fwBedrohung, "fwVorrang" => &mut q.fwVorrang, "eroeffLabor" => &mut q.eroeffLabor, "eroeffAus" => &mut q.eroeffAus, "eroeffBis" => &mut q.eroeffBis, "eroeffWarten" => &mut q.eroeffWarten, "bauGleichzeitig" => &mut q.bauGleichzeitig, "wacheRand" => &mut q.wacheRand, "wacheMax" => &mut q.wacheMax, "forschKampf" => &mut q.forschKampf, "seltenOhne" => &mut q.seltenOhne, "tempoWert" => &mut q.tempoWert, "geleit" => &mut q.geleit, "panzerFrueh" => &mut q.panzerFrueh, "panzerKlasse" => &mut q.panzerKlasse, "panzerModell" => &mut q.panzerModell, "zielTreue" => &mut q.zielTreue, "vorausBau" => &mut q.vorausBau, "schmelzeSpaet" => &mut q.schmelzeSpaet, "schmelzeEroeff" => &mut q.schmelzeEroeff, "marschHeil" => &mut q.marschHeil, "aufmarschFeind" => &mut q.aufmarschFeind, "marschSammeln" => &mut q.marschSammeln, "moralBau" => &mut q.moralBau, "bauStau" => &mut q.bauStau, "platzErreichbar" => &mut q.platzErreichbar, "schmelzeRisiko" => &mut q.schmelzeRisiko, "lkwWechsel" => &mut q.lkwWechsel, "sparFrist" => &mut q.sparFrist, "startBaken" => &mut q.startBaken, "startSchmelze" => &mut q.startSchmelze, "mineSeite" => &mut q.mineSeite, "kigaNachLynx" => &mut q.kigaNachLynx, "eroeffFolge" => &mut q.eroeffFolge, "moralFest" => &mut q.moralFest, "moralHalten" => &mut q.moralHalten, "missionsModus" => &mut q.missionsModus, "stufe" => &mut q.stufe, "rolle" => &mut q.rolle, "eigeneAngriffe" => &mut q.eigeneAngriffe, "schonfrist" => &mut q.schonfrist, "ueberlegMin" => &mut q.ueberlegMin, "testAngriff" => &mut q.testAngriff, "testStaerke" => &mut q.testStaerke, "testPosten" => &mut q.testPosten, "testPostenX" => &mut q.testPostenX, "testPostenY" => &mut q.testPostenY, "testPostenGroesse" => &mut q.testPostenGroesse, "testBakeX" => &mut q.testBakeX, "testBakeY" => &mut q.testBakeY, "minenUmkreis" => &mut q.minenUmkreis, "minenJeSorte" => &mut q.minenJeSorte, "wacheVorBasis" => &mut q.wacheVorBasis, "wachePatrouille" => &mut q.wachePatrouille, "wachePatrouilleTakt" => &mut q.wachePatrouilleTakt, "bezirke" => &mut q.bezirke, "gebietAuto" => &mut q.gebietAuto, "kernLuft" => &mut q.kernLuft, "roehrenVorrang" => &mut q.roehrenVorrang, "roehreAnNetz" => &mut q.roehreAnNetz, "kigaBremse" => &mut q.kigaBremse, "kigaAus" => &mut q.kigaAus, "kigaAn" => &mut q.kigaAn, "missionZiel" => &mut q.missionZiel, "raumhafen" => &mut q.raumhafen, "spinnen" => &mut q.spinnen, "aufstieg" => &mut q.aufstieg, "mhd" => &mut q.mhd, "solar" => &mut q.solar, "zbWache" => &mut q.zbWache, "zbWacheDruck" => &mut q.zbWacheDruck, "zbVorrang" => &mut q.zbVorrang, "marschEtappe" => &mut q.marschEtappe, "marschRadius" => &mut q.marschRadius, "marschAnteil" => &mut q.marschAnteil, "sparBedrohung" => &mut q.sparBedrohung, "medVorrang" => &mut q.medVorrang, "bauAbbruch" => &mut q.bauAbbruch, "wohnVorrang" => &mut q.wohnVorrang, "faktorNeu" => &mut q.faktorNeu, "laserTuerme" => &mut q.laserTuerme, "laserTurmAnteil" => &mut q.laserTurmAnteil, "laserKeinAngriff" => &mut q.laserKeinAngriff, "panzerBis" => &mut q.panzerBis, "geleitRadius" => &mut q.geleitRadius, "geleitMax" => &mut q.geleitMax, "geleitUeber" => &mut q.geleitUeber, "fabrikErz" => &mut q.fabrikErz, "fabrikAuslastung" => &mut q.fabrikAuslastung, "schwachstelle" => &mut q.schwachstelle, "schwachAussen" => &mut q.schwachAussen, "zugang" => &mut q.zugang, "zugangGruppen" => &mut q.zugangGruppen, "truppPlan" => &mut q.truppPlan, "truppRisiko" => &mut q.truppRisiko, "truppMin" => &mut q.truppMin, "truppAbwehr" => &mut q.truppAbwehr, "versuch" => &mut q.versuch, "versuchWert" => &mut q.versuchWert, "wertungTick" => &mut q.wertungTick, "missionen" => &mut q.missionen, "missionHorizont" => &mut q.missionHorizont, "missionMax" => &mut q.missionMax, "missionRueck" => &mut q.missionRueck, "missionUeberleg" => &mut q.missionUeberleg, "missionSieg" => &mut q.missionSieg, "missionLernen" => &mut q.missionLernen, "missionKorrUeberfall" => &mut q.missionKorrUeberfall, "missionKorrHalten" => &mut q.missionKorrHalten, "missionKorrEntscheidung" => &mut q.missionKorrEntscheidung, "missionKorrAbwehr" => &mut q.missionKorrAbwehr, "parallel" => &mut q.parallel, "parallelUmweg" => &mut q.parallelUmweg, "parallelAbstand" => &mut q.parallelAbstand, "parallelTiefe" => &mut q.parallelTiefe, "moralWert" => &mut q.moralWert, "erzAusbau" => &mut q.erzAusbau,
            "turmVorne" => &mut q.turmVorne, "turmAbstand" => &mut q.turmAbstand, "turmFront" => &mut q.turmFront, "turmUeberMauer" => &mut q.turmUeberMauer,
            "gefecht" => &mut q.gefecht, "gefechtRadius" => &mut q.gefechtRadius, "gefechtSpiel" => &mut q.gefechtSpiel, "schaumAnteil" => &mut q.schaumAnteil, "gefechtStellung" => &mut q.gefechtStellung, "gefechtSperre" => &mut q.gefechtSperre, "gefechtAbstand" => &mut q.gefechtAbstand, "gefechtAbstandSperre" => &mut q.gefechtAbstandSperre,
            "gefechtReichweitePlus" => &mut q.gefechtReichweitePlus, "gefechtBedrohungWert" => &mut q.gefechtBedrohungWert, "gefechtTurmGewicht" => &mut q.gefechtTurmGewicht, "gefechtAusweich" => &mut q.gefechtAusweich, "gefechtBesser" => &mut q.gefechtBesser, "gefechtRueckzugSchaden" => &mut q.gefechtRueckzugSchaden, "gefechtZielStrafe" => &mut q.gefechtZielStrafe, "gefechtStuetzeStrafe" => &mut q.gefechtStuetzeStrafe, "gefechtWeg" => &mut q.gefechtWeg, "vorplanWachstum" => &mut q.vorplanWachstum,
            "vorplanRadius" => &mut q.vorplanRadius, "vorplanDicht" => &mut q.vorplanDicht,
            "kampfModell" => &mut q.kampfModell,
            "sonde" => &mut q.sonde,

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
        let mut name = &zeile[..gl];
        // Fraktions-Praefix (Nutzer 2026-11-19: Eden anders spielen): "eden:name=wert" nur fuer Eden, "ply:name=wert" nur
        // fuer Plymouth -- beide Seiten koennen denselben Text bekommen
        if let Some(rest) = name.strip_prefix("eden:") { if !welt.IsEden(z.spieler) { continue; } name = rest; }
        else if let Some(rest) = name.strip_prefix("ply:") { if welt.IsEden(z.spieler) { continue; } name = rest; }
        let w = match scan_f64(&zeile[gl..], "=") { Some(w) => w, None => continue };
        if let Some(id) = name.strip_prefix("tf_").and_then(|s| s.trim().parse::<i32>().ok()) {
            techFaktoren.with(|t| t.borrow_mut()[sp].insert(id, w.max(0.0)));
            n += 1;
            continue;
        }
        let mut bekannt = false;
        if let Some(fe) = feld(&mut q, name) { *fe = w; bekannt = true; n += 1; }
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
    grenze(&mut q.gefechtAusweich, 1.0, 5.0); grenze(&mut q.gefechtBesser, 0.0, 1.0); grenze(&mut q.gefechtRueckzugSchaden, 0.05, 1.0); grenze(&mut q.gefechtSperre, 4.0, 400.0);
    grenze(&mut q.turmVorne, 1.0, 20.0); grenze(&mut q.turmAbstand, 1.0, 5.0); grenze(&mut q.turmFront, 1.0, 10.0); grenze(&mut q.turmUeberMauer, 0.5, 3.0);
    speichern(&q);
    melde(welt, z, "lernen", mapNone, z.bx, z.by, &format!("{} Parameter aus {} geladen (reaktion {:.0}, ueberlegenheit {:.2}, reserveStart {:.2}, lernen {:.0})",
        n, datei, q.reaktion, q.ueberlegenheit, q.reserveStart, q.lernen));
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
        let zeitWert = |t: i32| { let wt = wertungTick(z.spieler); let rest = if wt - t > 0 { wt - t } else { 0 }; 2000 + (8000.0 * rest as f64 / WERTUNG_TICK as f64) as i32 };
        if z.siegTick != 0 { return zeitWert(z.siegTick) + k; }
        if z.niederlageTick != 0 { return -zeitWert(z.niederlageTick) + k; }
        return k;
    }
    if z.ziel == 2 {
        // Bevoelkerung: Anteil am Ziel, dazu die selbst angerichtete Zerstoerung und Verluste nur halb so schwer.
        let anteil = if z.zielWert > 0 { if z.maxBew * 1000 / z.zielWert > 1000 { 1000 } else { z.maxBew * 1000 / z.zielWert } } else { 0 };
        p += anteil + z.zerstoertWert / 10;
        if z.zielErreicht != 0 { p += (wertungTick(z.spieler) - z.zielErreicht) / 200; }
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

/// C++: void wissenLaden(Zustand& z)  (op2_ki.hpp:6933)
pub fn wissenLaden(welt: &mut dyn Welt, z: &mut Zustand) {
    let datei = format!("KI-Wissen-P{}.txt", z.spieler);
    let mut pfad = String::new();
    wissenPfad(welt, &mut pfad, 600, &datei);
    let f = welt.dateiLesen(&pfad);
    let mut laeufe = 0;
    let mut beste = -(1 << 30);
    let mut besterLauf = 0;
    if let Some(text) = f {
        let q = PAR(z.spieler);
        for zeile in fgets_zeilen(&text, 400) {
            // Reserve aus frueheren Laeufen: um 20 % Richtung Ausgangswert zurueckbilden, hoechstens reserveMax
            if let Some(d) = scan_f64(&zeile, "reserve=").filter(|d| *d >= 1.0 && *d <= 4.0) {
                z.reserve = q.reserveStart + (d - q.reserveStart) * 0.8;
                if z.reserve > q.reserveMax { z.reserve = q.reserveMax; }
            }
            else if let Some(d) = scan_f64(&zeile, "erzJeRoute=").filter(|d| *d > 0.0) { z.erzJeRouteGelernt = d; }
            else if let Some(d) = scan_f64(&zeile, "turmAnteil=").filter(|d| *d >= 0.0 && *d <= 1.0) { z.turmAnteil = d; }
            else if let Some(i) = scan_i32(&zeile, "zermuerbAbstand=").filter(|i| *i >= 4000 && *i <= 30000) { z.zermuerbAbstand = i; }
            else if let Some(d) = scan_f64(&zeile, "quoteZiel=") { z.angriffQuote[1] = d; }
            else if let Some(d) = scan_f64(&zeile, "quoteZermuerben=") { z.angriffQuote[2] = d; }
            else if let Some(d) = scan_f64(&zeile, "quoteBasis=") { z.angriffQuote[0] = d; }
            else if let Some(i) = scan_i32(&zeile, "laeufe=") { laeufe = i; }
            else if zeile.starts_with("lauf=") {
                let nr = scan_i32(&zeile[5..], "").unwrap_or(0);
                if let Some(pp) = zeile.find("punkte=") {
                    if let Some(pk) = scan_i32(&zeile[pp + 7..], "") {
                        if pk > beste { beste = pk; besterLauf = nr; }
                    }
                }
            }
        }
    }
    z.laufNr = laeufe + 1;
    if laeufe != 0 {
        melde(welt, z, "lernen", mapNone, z.bx, z.by, &format!("Wissen geladen: {} Laeufe bisher, bester Lauf {} mit {} Punkten -- Reserve {:.2}, Erz je Route {:.0}",
            laeufe, besterLauf, beste, z.reserve, z.erzJeRouteGelernt));
    } else {
        melde(welt, z, "lernen", mapNone, z.bx, z.by, &format!("Noch kein Wissen ({}) -- Reserve {:.2}", pfad, z.reserve));
    }
}

/// C++: void wissenSpeichern(Zustand& z)  (op2_ki.hpp:6972)
pub fn wissenSpeichern(welt: &mut dyn Welt, z: &mut Zustand) {
    if PAR(z.spieler).lernen == 0.0 { ergebnisSchreiben(welt, z); return; }
    let datei = format!("KI-Wissen-P{}.txt", z.spieler);
    let mut pfad = String::new();
    wissenPfad(welt, &mut pfad, 600, &datei);
    // Alte Laufzeilen behalten (ohne diesen Lauf), hoechstens 19
    let mut alt: Vec<String> = Vec::new();
    if let Some(text) = welt.dateiLesen(&pfad) {
        for zeile in fgets_zeilen(&text, 400) {
            if !zeile.starts_with("lauf=") { continue; }
            let nr = scan_i32(&zeile[5..], "").unwrap_or(0);
            if nr == z.laufNr { continue; }
            if alt.len() == 19 { alt.remove(0); }
            // snprintf(alt[na++], 400, "%s", zeile): hoechstens 399 Bytes
            let mut s = zeile.clone();
            if s.len() > 399 { let mut e = 399; while !s.is_char_boundary(e) { e -= 1; } s.truncate(e); }
            alt.push(s);
        }
    }
    let mut f = String::new();
    f.push_str("# KI-Wissen der Aufbau-KI (op2_ki.hpp) -- wird automatisch geschrieben\n");
    f.push_str(&format!("laeufe={}\nreserve={:.3}\nturmAnteil={:.3}\n", z.laufNr, z.reserve, z.turmAnteil));
    f.push_str(&format!("zermuerbAbstand={}\nquoteBasis={:.3}\nquoteZiel={:.3}\nquoteZermuerben={:.3}\n", z.zermuerbAbstand,
        z.angriffQuote[0], z.angriffQuote[1], z.angriffQuote[2]));
    // Erzertrag je Route: gelernter Wert, gleitend mit diesem Lauf
    let re = erzRouten(welt, z);
    if re > 0 && z.zuflussErz / re as f64 > 0.0 {
        let jetzt = z.zuflussErz / re as f64;
        z.erzJeRouteGelernt = if z.erzJeRouteGelernt > 0.0 { z.erzJeRouteGelernt * 0.8 + jetzt * 0.2 } else { jetzt };
    }
    if z.erzJeRouteGelernt > 0.0 { f.push_str(&format!("erzJeRoute={:.1}\n", z.erzJeRouteGelernt)); }
    for a in alt.iter() { f.push_str(a); }
    let t = welt.Tick();
    let kl = if l_kolonieLebt(welt, z) { 1 } else { 0 };
    let lp = laufPunkte(welt, z);
    f.push_str(&format!("lauf={} tick={} zielArt={} ziel={} gefechte={} ueberstanden={} zerstoert={} verloren={} maxBew={} moral={} feindbasis={} kolonie={} reserve={:.2} punkte={}\n",
        z.laufNr, if t < wertungTick(z.spieler) { t } else { wertungTick(z.spieler) }, z.ziel, z.zielErreicht, z.anzWellen, z.ueberstanden,
        z.zerstoertWert, z.verlorenWert, z.maxBew,
        if z.moralN != 0 { (z.moralSumme / z.moralN as f64) as i32 } else { 0 }, z.feindVernichtet, kl, z.reserve, lp));
    welt.dateiSchreiben(&pfad, &f);
}
