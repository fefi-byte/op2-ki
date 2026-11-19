//! Ganze Partie mit der uebertragenen C++-KI (op2ki::cpp) auf op2-sim: Missionsstart wie KI_Mehrspieler (InitProc),
//! dann je Tick ki::schritt fuer beide Spieler (AIProc). Whole game with the translated C++ AI on op2-sim.
use anyhow::Context;
use op2_sim::{Rules, Sim};
use crate::api_voll::*;
use op2ki::cpp::{self, welt::{Loc, Welt}, zustand::Zustand};

use crate::welt_sim::{SimWelt, UX};

pub struct Partie2 { pub welt: SimWelt, pub z: Vec<(i32, Box<Zustand>)> }

impl Partie2 {
    /// Wie die Kartenmissionen KI_K_<karte> (ki_rust/karten_missionen.py): Szenario des op2-missionhost (Baken, Quellen,
    /// Startplaetze), Spieler 1 Plymouth am Startplatz von Spieler 0, Spieler 2 Eden am Startplatz von Spieler 1,
    /// sonst wie KI_Mehrspieler. `parameter[i]`: Stellschrauben-Text fuer Spieler i+1 (name=wert je Zeile).
    pub fn szenario(files: &op2_formats::GameFiles, rules: &Rules, scn: &serde_json::Value, seed: u64, parameter: [Option<String>; 2]) -> anyhow::Result<Partie2> {
        cpp::globale::zuruecksetzen();
        // Die Duell-Missionen bringen diesen Satz mit (ki::parameterText aus KI_Mehrspieler); der eigene Text ueberschreibt einzelne Werte.
        const MISSION: &str = include_str!("../daten/mission_param.txt");
        for (i, t) in parameter.iter().enumerate() {
            if let Some(t) = t { cpp::globale::parameterText.with(|p| p.borrow_mut()[i + 1] = Some(format!("{MISSION}\n{t}"))); }
            else { cpp::globale::parameterText.with(|p| p.borrow_mut()[i + 1] = Some(MISSION.into())); }
        }
        let karte = scn["map"].as_str().unwrap_or("on2_01.map").to_string();
        let mut sim = Sim::new(op2_sim::load_terrain(files, &karte)?, rules.clone(), seed);
        let (b, h) = (sim.terrain().width as i32, sim.terrain().height as i32);
        sim.human = 0b111; sim.plymouth = 1 << 1; sim.difficulty = 1; sim.colony_start = 1;
        // Pruefschalter OP2KI_FRAKTION_TAUSCH=1: Spieler 2 Plymouth, Spieler 1 Eden (Fraktion vom Startplatz trennen; im
        // Original nicht vorgesehen -- die Missionen setzen Player[1].GoPlymouth()).
        if std::env::var("OP2KI_FRAKTION_TAUSCH").is_ok_and(|v| v == "1") { sim.plymouth = 1 << 2; }
        // OP2KI_GEFECHT_NETZ=<datei>: Gefechtsnetz fuer beide Spieler (wirkt nur bei PAR gefecht >= 1.5). Die Sim hat keinen
        // Wissensordner; ohne Netz fiele gefecht 1.5 still auf die Handsteuerung zurueck -- darum Fehler statt Rueckfall.
        if let Ok(f) = std::env::var("OP2KI_GEFECHT_NETZ") {
            let t = std::fs::read_to_string(&f).with_context(|| format!("Gefechtsnetz {f}"))?;
            let n = cpp::gefecht_netz::Netz::aus_text(&t).with_context(|| format!("Gefechtsnetz {f} unlesbar"))?;
            for sp in [1, 2] { cpp::gefecht_netz::setze(sp, Some(n.clone())); }
        }
        for p in 0..3u8 { sim.colony_mut(p); sim.set_tech_level(p, 3); }
        for p in [1u8, 2] { let c = sim.colony_mut(p); c.workers = 26; c.scientists = 13; c.kids = 15; c.common = 8000; c.food = 3000; c.recalc = true; }
        let mut welt = SimWelt::neu(sim);
        let mut cc: [Option<(i32, i32)>; 2] = [None, None];
        let mut fz: [Vec<(i32, i32)>; 2] = [Vec::new(), Vec::new()];
        for s in scn["structures"].as_array().into_iter().flatten() {
            let t = s["type"].as_str().unwrap_or(""); let (x, y) = (s["tile"][0].as_i64().unwrap_or(0) as i32, s["tile"][1].as_i64().unwrap_or(0) as i32);
            let o = s["owner"].as_i64().unwrap_or(-1);
            match t {
                "BEACON" => { let e = &s["extra"]; welt.CreateBeacon(mapMiningBeacon, x + UX, y, e["oreType"].as_i64().unwrap_or(0).max(0) as i32, e["yield"].as_i64().unwrap_or(1).max(0) as i32, -1); }   // die Duell-Missionen legen Baken mit VariantRandom an
                "MAGMA_VENT" => { welt.CreateBeacon(mapMagmaVent, x + UX, y, 0, 0, 0); }
                "FUMAROLE" => { welt.CreateBeacon(mapFumarole, x + UX, y, 0, 0, 0); }
                "COMMAND" if (0..2).contains(&o) => { cc[o as usize] = Some((x, y)); }
                _ => {}
            }
        }
        for u in scn["spawn"].as_array().into_iter().flatten() {
            let o = u["owner"].as_i64().unwrap_or(-1);
            if (0..2).contains(&o) { fz[o as usize].push((u["tile"][0].as_i64().unwrap_or(0) as i32, u["tile"][1].as_i64().unwrap_or(0) as i32)); }
        }
        // Alle Startplaetze der Original-Mission (Spieler 0..N); der Seed waehlt das Paar wie startwahl.cpp der Mission
        let mut ccn: Vec<Option<(i32, i32)>> = vec![None; 8];
        let mut fzn: Vec<Vec<(i32, i32)>> = vec![Vec::new(); 8];
        for s in scn["structures"].as_array().into_iter().flatten() {
            let o = s["owner"].as_i64().unwrap_or(-1);
            if s["type"].as_str() == Some("COMMAND") && (0..8).contains(&o) && ccn[o as usize].is_none() { ccn[o as usize] = Some((s["tile"][0].as_i64().unwrap_or(0) as i32, s["tile"][1].as_i64().unwrap_or(0) as i32)); }
        }
        for u in scn["spawn"].as_array().into_iter().flatten() {
            let o = u["owner"].as_i64().unwrap_or(-1);
            if (0..8).contains(&o) { fzn[o as usize].push((u["tile"][0].as_i64().unwrap_or(0) as i32, u["tile"][1].as_i64().unwrap_or(0) as i32)); }
        }
        let _ = (&cc, &fz);
        let plaetze: Vec<(i32, i32)> = (0..8).filter_map(|o| ccn[o].or_else(|| {
            let v = &fzn[o]; if v.is_empty() { return None; } let n = v.len() as f64;
            Some(((v.iter().map(|p| p.0).sum::<i32>() as f64 / n).round() as i32, (v.iter().map(|p| p.1).sum::<i32>() as f64 / n).round() as i32))
        })).collect();
        let n = plaetze.len().max(2) as i64;
        let k = (((seed as i64 - 1) % (n * (n - 1))) + n * (n - 1)) % (n * (n - 1));
        let pa = (k / (n - 1)) as usize; let mut pb = (k % (n - 1)) as usize; if pb >= pa { pb += 1; }
        let start = vec![plaetze[pa], plaetze[pb]];
        eprintln!("Startwahl: Seed {seed} -> Spieler 1 an Platz {pa} {:?}, Spieler 2 an Platz {pb} {:?} von {n}", plaetze[pa], plaetze[pb]);
        let mut z = Vec::new();
        for (i, p) in [1, 2].into_iter().enumerate() {
            let mut zs = Zustand::null();
            cpp::start(&mut welt, &mut zs, p, Loc::new(UX, 0), b, h, start[i].0, start[i].1, true, 3, 3, 500);
            z.push((p, zs));
        }
        welt.sim.forced_morale[1] = 0xFF;
        // Die Gruppen laufen im Missionstakt (tick() oben), nicht in end_cycle.
        welt.sim.groups_mut().external = true;
        Ok(Partie2 { welt, z })
    }

    /// Ein Tick wie GameImpl::ProcessGameCycle (op2-missionhost runtime.rs): Kolonie-Updates (begin_cycle), bei
    /// `tick & 3 == 3` der Missionstakt -- KI-Gruppen, dann AIProc (ki::schritt fuer beide) --, dann der Rest des Takts.
    pub fn tick(&mut self) {
        self.welt.sim.begin_cycle();
        // OP2KI_BEOBACHTE=tick:objektnummer -- Zustand einer Einheit innerhalb des Ticks (Messung)
        let beob: Option<(u32, u16)> = std::env::var("OP2KI_BEOBACHTE").ok().and_then(|v| { let (a, b) = v.split_once(':')?; Some((a.parse().ok()?, b.parse().ok()?)) });
        let zeig = |sim: &op2_sim::Sim, wo: &str| if let Some((t, nr)) = beob { if sim.tick() == t { for u in sim.units().iter().filter(|u| u.index == nr && u.alive) {
            eprintln!("beob {} {}: cmd {:?} act {:?} dir {} busy {} wait {} motion {:?}", t, wo, u.cmd, u.action, u.dir, u.busy, u.wait, u.motion); } } };
        zeig(&self.welt.sim, "nach begin_cycle");
        if self.welt.sim.tick() & 3 == 3 {
            self.welt.sim.update_groups();
            zeig(&self.welt.sim, "nach update_groups");
            for (p, zs) in self.z.iter_mut() {
                cpp::globale::g_kraftSpieler.with(|g| g.set(*p));
                cpp::schritt(&mut self.welt, zs);
            }
        }
        zeig(&self.welt.sim, "vor end_cycle");
        self.welt.sim.end_cycle();
        if let Some((t, nr)) = beob { if self.welt.sim.tick() == t + 1 { for u in self.welt.sim.units().iter().filter(|u| u.index == nr && u.alive) {
            eprintln!("beob {} nach end_cycle: cmd {:?} act {:?} dir {} busy {} wait {} motion {:?}", t, u.cmd, u.action, u.dir, u.busy, u.wait, u.motion); } } }
    }
}
