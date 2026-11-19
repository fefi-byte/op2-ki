//! op2-ki gegen op2-ki auf op2-sim: eine ganze Mehrspieler-Partie ohne Spiel, schreibt die KI-Spur (JSON-Zeilen).
//! op2-ki vs op2-ki on op2-sim: a whole multiplayer game without the game, writes the AI trace (JSON lines).
//! Aufruf: op2ki-sim --game-dir <Outpost 2> --szenario <scn.json> [--seed N] [--ticks N] [--ende] [--p1 "a=1;b=2"]
//!         [--p2 "..."] [--spur <datei>]
//! --ende: Schluss 2500 Ticks, nachdem ein Spieler Kommandozentrale und Strukturfabrik verloren hat.
#![allow(non_snake_case)]
mod api_voll;
mod partie2;
mod typen;
mod welt_sim;
use anyhow::{Context, Result};
use op2_sim::Rules;
use op2ki::cpp::welt::Welt;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let opt = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let dir = opt("--game-dir").context("--game-dir fehlt")?;
    let files = op2_formats::GameFiles::open(&dir).context("Spielordner")?;
    let rules = Rules::from_install(&files)?.with_techs(&files, "MULTITEK.TXT")?.with_art(&files).classic();
    let ticks: u32 = opt("--ticks").unwrap_or("100000".into()).parse()?;
    let seed: u64 = opt("--seed").unwrap_or("1".into()).parse()?;
    let lies = |k: &str| opt(k).map(|f| std::fs::read_to_string(&f).unwrap_or(f.replace(';', "\n")));
    let scn: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(opt("--szenario").context("--szenario fehlt")?)?)?;
    let mut p = partie2::Partie2::szenario(&files, &rules, &scn, seed, [lies("--p1"), lies("--p2")])?;
    if let Some(sp) = opt("--spur") { p.welt.spur = Some(std::fs::File::create(sp)?); }
    let ende = args.iter().any(|a| a == "--ende");
    let mut ende_bei: Option<u32> = None;
    while p.welt.sim.tick() < ticks {
        let t = p.welt.sim.tick();
        if ende && t % 100 == 0 && t > 1000 {
            if ende_bei.is_none() {
                let w = &p.welt;
                let basis = |q: i32| [api_voll::mapCommandCenter, api_voll::mapStructureFactory].iter()
                    .any(|&ty| w.PlayerBuildingEnum(q, ty).iter().any(|&u| w.IsLive(u)));
                if !basis(1) || !basis(2) { ende_bei = Some(t + 2500); }
            }
            if ende_bei.is_some_and(|e| t >= e) { eprintln!("Partie entschieden, Ende bei Tick {t}"); break; }
        }
        p.tick();
    }
    eprintln!("{} Ticks", p.welt.sim.tick());
    Ok(())
}
