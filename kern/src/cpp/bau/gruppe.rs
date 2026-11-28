//! BuildingGroup: Bauauftraege an die ConVecs (bauGruppe), Roehrenauftraege (roehren).
use super::*;
use super::super::*;
use super::{GEBAEUDE, TECH_RAUMFAHRT, B_ZIEL};

/// C++: void bauGruppe(Zustand& z, const Lage& l)  (op2_ki.hpp:2892)
pub fn bauGruppe(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    stauLoesen(welt, z);
    let p = z.spieler;
    let t = welt.Tick();
    // Warteplatz der Gruppe: unter der Basis, frei von Gebaeuden und Erzwegen; alle 2000 Ticks neu pruefen (die Basis waechst)
    if z.bauAn == 0 || t % 2000 < TAKT {
        let mut rx = z.bx;
        let mut ry = z.by + 9;
        freierWarteplatz(welt, z, &mut rx, &mut ry, 2, false);
        let gp = g_platz.with(|v| v.borrow()[(p & 7) as usize][3]);
        let ruhe = Rect::new(engine(welt, z, rx - 3, ry - 1), engine(welt, z, rx + 3, ry + 2));
        if z.bauAn == 0 {
            z.bau = welt.CreateBuildingGroup(p);
            welt.SetRect(z.bau, ruhe);
            // Soll der Gruppe: ohne angemeldeten Earthworker baut sie keine Roehren (Lauf 7)
            welt.SetTargCount(z.bau, mapEarthworker, mapNone, 1);
            z.bauAn = 1;
            melde(welt, z, "info", mapStructureFactory, rx, ry, &format!("BuildingGroup angelegt (Warteplatz {},{}, frei von Gebaeuden und Erzwegen)", rx, ry));
        } else if gp[0] != rx || gp[1] != ry {
            welt.SetRect(z.bau, ruhe);
            melde(welt, z, "info", mapStructureFactory, rx, ry, &format!("Warteplatz der BuildingGroup verlegt nach {},{} (alter Platz belegt)", rx, ry));
        }
        g_platz.with(|v| { v.borrow_mut()[(p & 7) as usize][3] = [rx, ry]; });
    }
    // Einheiten aufnehmen: fertige, ruhige Strukturfabriken und alle ConVecs
    for u in welt.PlayerBuildingEnum(p, mapStructureFactory) {
        if bereit(welt, u) && !inGruppe(welt, z.bau, u) && !super::super::zweitbasis::gehoertBasis(z, u) && !super::super::zweitbasis::amPosten(welt, z, u) {
            welt.TakeUnit(z.bau, u);
            let lo = welt.Location(u);
            let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
            melde(welt, z, "info", mapStructureFactory, x, y, &format!("Strukturfabrik {} in die BuildingGroup", u));
        }
    }
    let mut getragen = [0i32; 128];
    for u in welt.PlayerVehicleEnum(p) {
        if super::super::zweitbasis::gehoertBasis(z, u) || super::super::zweitbasis::reserviert(welt, z, u) { continue; }
        if z.vorratCv > 0 && u == z.vorratCv { continue; }   // baut die Startbausaetze daneben   // Missionsmodus: Einheit der zweiten Basis
        if welt.GetType(u) == mapEarthworker && !inGruppe(welt, z.bau, u) { welt.TakeUnit(z.bau, u); continue; } // baut die Roehren
        if welt.GetType(u) != mapConVec { continue; }
        if !inGruppe(welt, z.bau, u) { welt.TakeUnit(z.bau, u); }
        let k = welt.GetCargo(u);
        // Nur ruhige ConVecs zaehlen: waehrend des Baus traegt der ConVec den Bausatz noch.
        if k > 0 && k < 128 && k != mapNone && !welt.IsBusy(u) && welt.GetCurAction(u) == moDone { getragen[k as usize] += 1; }
    }
    // Aufgezeichnete Plaetze nachfuehren
    let mut offen = 0;
    for i in 0..z.anzahlPlaetze as usize {
        let q = z.plaetze[i];
        if q.zustand != P_FAHRT && q.zustand != P_BAUT { continue; }
        let eigen = super::super::zweitbasis::gruppeAm(z, q.x0 as i32, q.y0 as i32) == z.bau;
        if stehtDa(welt, z, &q) {
            z.plaetze[i].zustand = P_STEHT; z.plaetze[i].seit = t;
            melde(welt, z, "fertig", q.typ as i32, q.x0 as i32, q.y0 as i32, "Gebaeude steht (im Bau oder fertig)");
            continue;
        }
        if !eigen { continue; }   // baut die Gruppe der zweiten Basis
        if z.vorratPlatz == i as i32 + 1 { continue; }   // baut der Vorrat-ConVec
        offen += 1;
        if t - q.seit > GEDULD_GRUPPE && selten(welt, z, 1, 3000) {
            melde(welt, z, "warte", q.typ as i32, q.x0 as i32, q.y0 as i32,
                  &format!("BuildingGroup baut seit {} Ticks noch nicht (Erz {}, Seltenerz {}, Strukturfabrik {})",
                           t - q.seit, l.erz, l.selten, if l.sf != 0 { "da" } else { "fehlt" }));
        }
    }
    // PAR bauAbbruch (2026-11-20): ein aufgezeichneter Bau, der nach so vielen Ticks noch nicht steht, sperrt die Gruppe fuer
    // alles andere (mp4_08: ein Wachturm 33000 Ticks, Medical Center und Schmelze kamen nie). Die Welt kennt kein Loeschen
    // einzelner Auftraege: Platz sperren, BuildingGroup aufloesen und neu anlegen, die anderen offenen Plaetze freigeben.
    let ab = PAR(p).bauAbbruch as i32;
    if ab > 0 && offen > 0 {
        let mut haengt: Option<usize> = None;
        for i in 0..z.anzahlPlaetze as usize {
            let q = z.plaetze[i];
            // Missionsmodus: ferne Plaetze (Aussenposten) bekommen Fahrzeit dazu -- die ConVecs holen jeden Bausatz daheim
            // (ml4_05: Seltenerz-Schmelze 40 Kacheln weg, Bausatz erst nach 7400 Ticks unterwegs)
            let geduld = if super::super::modus::mission(z) {
                let d = ((((q.x0 as i32 - z.bx).pow(2) + (q.y0 as i32 - z.by).pow(2)) as f64).sqrt() - 15.0).max(0.0);
                ab + (d * 150.0) as i32
            } else { ab };
            if (q.zustand == P_FAHRT || q.zustand == P_BAUT) && t - q.seit > geduld && super::super::zweitbasis::gruppeAm(z, q.x0 as i32, q.y0 as i32) == z.bau { haengt = Some(i); break; }
        }
        if let Some(i) = haengt {
            let q = z.plaetze[i];
            for j in 0..z.anzahlPlaetze as usize {
                let fremd = super::super::zweitbasis::gruppeAm(z, z.plaetze[j].x0 as i32, z.plaetze[j].y0 as i32) != z.bau;
                if (z.plaetze[j].zustand == P_FAHRT || z.plaetze[j].zustand == P_BAUT) && !fremd { z.plaetze[j].zustand = if j == i { P_GESPERRT } else { P_FREI }; }
            }
            let g = z.bau; welt.Destroy(g); z.bauAn = 0;
            // Missionsmodus: mit der Gruppe sind auch ihre Roehrenauftraege weg -- offen gefuehrte Roehren freigeben, sonst
            // wartete anschluss fuer immer auf sie (cm03: 6 Kacheln 130 000 Ticks, die Fahrzeugfabrik blieb ohne Personal).
            // Im Mehrspieler-Modus (noch) nicht, damit er gleich bleibt.
            if super::super::modus::mission(z) {
                for i in 0..z.anzahlRoehren.max(0) as usize { if z.roehren[i].zustand == 1 { z.roehren[i].zustand = 2; } }
            }
            melde(welt, z, "diagnose", q.typ as i32, q.x0 as i32, q.y0 as i32,
                  &format!("Bauauftrag abgebrochen nach {} Ticks ohne Fortschritt -- Platz gesperrt, BuildingGroup neu", t - q.seit));
            return;
        }
    }
    // Bausaetze, die ConVecs schon tragen (Startausruestung), sofort aufzeichnen
    // (Kommandozentrale zuerst: an ihr haengen die Anschlussplaetze aller anderen)
    for j in 0..128i32 {
        let k = if j == 0 { mapCommandCenter } else if j == mapCommandCenter { 0 } else { j };
        if k <= 0 { continue; }
        while getragen[k as usize] > geplant(welt, z, k) {
            if !bauAufzeichnen(welt, z, k, "Bausatz liegt schon auf einem ConVec") { getragen[k as usize] = 0; break; }
            offen += 1;
        }
    }
    // Neues Gebaeude erst, wenn weniger als PAR bauGleichzeitig offen sind (Standard 1: erst wenn nichts mehr offen ist),
    // ein ConVec dafuer frei ist und eine Strukturfabrik in der Gruppe arbeitet. Gemessen 2026-11-19 mit 2: die
    // BuildingGroup baut die Liste trotzdem nacheinander ab, der zweite Bau wartete 6000-9000 Ticks und band sein Erz;
    // Sparring 2/9 statt 4/9 -- der Engpass sind die Arbeiter, nicht die Zahl der Auftraege
    let gleichzeitig = (PAR(p).bauGleichzeitig as i32).max(1);
    if offen >= gleichzeitig || l.sf == 0 || !inGruppe(welt, z.bau, l.sf) { return; }
    if offen > 0 && offen >= welt.PlayerVehicleEnum(p).into_iter().filter(|&u| welt.GetType(u) == mapConVec && welt.IsLive(u)).count() as i32 { return; }
    // Erz der schon offenen Bauten (noch nicht bezahlt) zaehlt mit
    let mut offenErz = 0;
    for i in 0..z.anzahlPlaetze as usize {
        let q = z.plaetze[i];
        if q.zustand == P_FAHRT || q.zustand == P_BAUT { offenErz += welt.GetOreCost(q.typ as i32, p); }
    }
    let w = bauWahl(welt, z, l);
    if w.typ != mapNone && w.typ != mapCommandCenter && istGeparkt(welt, z, l.sf) {
        // Nur wecken, wenn der Bau auch mit den Arbeitern geht, die die Fabrik selbst wieder braucht -- sonst pendelt sie.
        // Wake only if the build still works once the factory takes its own workers back.
        let sfArb = welt.GetWorkersRequired(mapStructureFactory, p);
        let bauArb = welt.GetWorkersRequired(w.typ, p);
        if l.arbFrei - sfArb >= bauArb || standardVolk(welt, l) {
            leerlaufWecken(welt, z, l.sf, &w.warum); // Leerlauf vorbei: erst einschalten, naechster Schritt baut
            return;
        }
        // Arbeiter-Wunsch: Fabrik + Bau zusammen; die Rangliste legt dafuer Unwichtigeres still
        z.wunschArbeiter = sfArb + bauArb;
        z.wunschRang = rang(welt, z, l, w.typ) + if w.bereich == B_ZIEL { 10 } else { 0 };
        z.wunschSeit = welt.Tick();
        if selten(welt, z, 24, 3000) {
            let wn = name(welt, w.typ);
            melde(welt, z, "warte", w.typ, z.bx, z.by, &format!("Strukturfabrik bleibt still: {} braucht Arbeiter, die die Fabrik selbst bindet", wn));
        }
        return;
    }
    if w.typ == mapNone || w.typ == mapCommandCenter {
        if !w.warum.is_empty() && selten(welt, z, 0, 1500) { melde(welt, z, "warte", mapNone, z.bx, z.by, &format!("{}", w.warum)); }
        return;
    }
    // Notfaelle kommen ohne Kostenpruefung aus bauWahl -- hier nachholen (nicht leistbar = warten)
    let kosten = welt.GetOreCost(w.typ, p);
    let kostenSelten = welt.GetRareOreCost(w.typ, p);
    if offen > 0 && kosten + offenErz > l.erz { return; }   // zweiter Bau nur, wenn das Erz fuer alle offenen reicht
    if kosten > l.erz || kostenSelten > l.selten {
        if selten(welt, z, 0, 1500) {
            melde(welt, z, "warte", w.typ, z.bx, z.by, &format!("Erz {}/{}, Seltenerz {}/{} ({})", l.erz, kosten, l.selten, kostenSelten, w.warum));
        }
        return;
    }
    if bauAufzeichnen(welt, z, w.typ, &w.warum) { z.letzterBau = welt.Tick(); }
}

// ---------------------------------------------------------------------------------------------------
// Roehren / tubes: der Earthworker arbeitet die Auftragsliste ab

/// C++: bool roehreFertig(const Zustand& z, const Roehre& r)  (op2_ki.hpp:2999)
pub fn roehreFertig(welt: &mut dyn Welt, z: &Zustand, r: &Roehre) -> bool {
    for y in r.y1 as i32..=r.y2 as i32 {
        for x in r.x1 as i32..=r.x2 as i32 {
            let c = zelle(welt, z, x, y);
            if !zelleRoehre(welt, c) { return false; }
        }
    }
    true
}

