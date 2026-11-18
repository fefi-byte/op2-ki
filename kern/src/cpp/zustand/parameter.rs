//! Parameter (PAR): alle Stellschrauben mit Doku. Die Voreinstellungen sind die Werte, mit denen die KI im Spiel antritt
//! (gelernte Stellschrauben der Missionen); Missionstext und logs/KI-Parameter-P<n>.txt koennen einzelne ueberschreiben.

/// Stellschrauben je Spieler (C++ struct Parameter).
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
    /// Kein Platz am Roehrenstutzen: freien Platz bis zu diesem Abstand nehmen und per Roehre anschliessen (0 = nie)
    pub platzFern: f64,
    /// Platz ohne Anschluss: Wertung je noetiger Roehrenkachel, in Abstand^2-Kacheln zur Basismitte
    pub roehreKachel: f64,
    /// Waffenwahl nach Paarungen: Gewicht des gemessenen Paarungswerts (0 = ohne)
    pub paarung: f64,
    /// Wachturm ohne Roehrenverbindung zur Zentrale (kein Strom): Anteil seines Kampfwerts
    pub turmOhneNetz: f64,
    /// weitere Fahrzeugfabrik, wenn der Erzzufluss mehr ist als dieser Anteil dessen, was die Fabriken verbauen
    pub fabrikAuslastung: f64,
    /// Moral: Wert einer Moralstufe in Bewohnern je Bewohner (Waehrung von kolonieNutzen)
    pub moralWert: f64,
    /// Wachtuerme der Kolonie:
    /// Abstand der Turmlinie vor dem aeussersten Gebaeude (Kacheln), Abstand der Tuerme in der Linie und zwischen den Reihen,
    /// Gewicht der Feindrichtung gegen Seiten, von denen schon angegriffen wurde, und Faktor fuer Turmwaffen, die ueber Mauern
    /// schiessen (Saeure, ESG, RPG, Thor, EMP, Stickyfoam), in der Waffenwahl des Turms
    pub turmVorne: f64,
    pub turmAbstand: f64,
    pub turmFront: f64,
    pub turmUeberMauer: f64,
    /// Gefechtsfuehrung (gelerntes Netz): Radius des Gefechts (Kacheln) und Sperrzeit zwischen zwei Ausweichzuegen einer
    /// Einheit (Ticks)
    pub gefechtRadius: f64,
    pub gefechtSperre: f64,
    /// Bebauungsplan: erwartetes Bevoelkerungswachstum im Horizont (Anteil), Suchradius um die Basis, Gewicht dichter Packung
    pub vorplanWachstum: f64,
    pub vorplanRadius: f64,
    pub vorplanDicht: f64,
}
impl Default for Parameter {
    fn default() -> Self {
        Parameter {
            reaktion: 15000.0,
            horizont: 9233.9408,
            ueberlegenheit: 1.2702,
            fabrikGewinn: 0.8052,
            reserveStart: 0.8,
            reserveLernrate: 0.476,
            reserveRueckgang: 1.0,
            reserveMax: 4.0,
            turmAnteil: 0.8,
            zermuerbAbstand: 2000.0,
            schmelzeNutzen: 1.4966,
            wohnraumGrenze: 98.0,
            freischaltWert: 0.516,
            nahRadius: 40.0,
            heimHalten: 1.6362,
            steigerung: 1.5,
            empSchwelle: 1.0,
            schmelzenJeMine: 2.0,
            vorsorge: 57.4926,
            seltenGewinn: 0.25,
            risikoAnteil: 0.75,
            angriffMarge: 0.03,
            rueckzugAb: 0.8,
            platzFern: 12.0,
            roehreKachel: 20.0,
            paarung: 1.0,
            turmOhneNetz: 0.55,
            fabrikAuslastung: 1.0,
            moralWert: 1.0,
            turmVorne: 3.0,
            turmAbstand: 2.0,
            turmFront: 2.0,
            turmUeberMauer: 1.0,
            gefechtRadius: 10.0,
            gefechtSperre: 40.0,
            vorplanWachstum: 0.5,
            vorplanRadius: 14.0,
            vorplanDicht: 8.0,
        }
    }
}
