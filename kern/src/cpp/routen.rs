//! Rust (Nutzer 2026-11-18): Angriffsrouten durchs Gelaende statt eines festen Versatzes. Bis zu drei Wege zum Ziel, die
//! durch verschiedene Zugaenge fuehren: erst der kuerzeste, dann wird der Streifen um jeden gefundenen Weg verteuert und neu
//! gesucht (Paesse im Felsriegel, Einfahrten der Basis). Je Route: Wegpunkte, Laenge, Gefahr durch Feindarmee und Tuerme
//! entlang des Wegs, schmalste Stelle.
#![allow(non_snake_case)]
use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::gelaende::befahrbar;
use super::hilfen::*;
use super::welt::Welt;
use super::zustand::Zustand;

const N8: [(i32, i32, i32); 8] = [(1, 0, 10), (-1, 0, 10), (0, 1, 10), (0, -1, 10), (1, 1, 14), (1, -1, 14), (-1, 1, 14), (-1, -1, 14)];
/// Abstand der Wegpunkte (Kacheln) und wie weit vor dem Ziel der letzte liegt
const WP_SCHRITT: usize = 8;
const WP_VOR_ZIEL: usize = 8;
pub const MAX_ROUTEN: usize = 3;
pub const MAX_WP: usize = 16;

#[derive(Clone, Debug)]
pub struct Route { pub weg: Vec<(i32, i32)>, pub punkte: Vec<(i32, i32)>, pub laenge: f32 }

/// Merkmale einer Route fuer das Truppnetz
#[derive(Clone, Copy, Debug, Default)]
pub struct RoutenInfo { pub laenge: f32, pub umweg: f32, pub gefahrArmee: f32, pub gefahrTurm: f32, pub minFeind: f32, pub breite: f32, pub anzahl: f32 }

pub struct Raster { pub frei: Vec<bool>, pub b: i32, pub h: i32 }

thread_local! {
    static RASTER: RefCell<Option<(i32, i32, i32, std::rc::Rc<Raster>)>> = const { RefCell::new(None) };   // Spieler, Tick, Breite, Raster
    static CACHE: RefCell<Vec<((i32, i32, i32, i32, i32), std::rc::Rc<Vec<Route>>)>> = const { RefCell::new(Vec::new()) };
}
pub fn zuruecksetzen() { RASTER.with(|r| *r.borrow_mut() = None); CACHE.with(|c| c.borrow_mut().clear()); }

/// Befahrbarkeit der Karte (Zelltypen), alle 2000 Ticks neu
pub fn raster(welt: &mut dyn Welt, z: &Zustand) -> std::rc::Rc<Raster> {
    let t = welt.Tick();
    if let Some(r) = RASTER.with(|r| r.borrow().as_ref().filter(|e| e.0 == z.spieler && t - e.1 < 2000 && t >= e.1 && e.2 == z.breite).map(|e| e.3.clone())) { return r; }
    let (b, h) = (z.breite.clamp(1, 512), z.hoehe.clamp(1, 512));
    let mut frei = vec![false; (b * h) as usize];
    for y in 0..h { for x in 0..b { let l = engine(welt, z, x, y); frei[(y * b + x) as usize] = befahrbar(welt.GetCellType(l)); } }
    let r = std::rc::Rc::new(Raster { frei, b, h });
    RASTER.with(|x| *x.borrow_mut() = Some((z.spieler, t, z.breite, r.clone())));
    r
}

fn dijkstra(r: &Raster, strafe: &[u16], start: (i32, i32), ziel: (i32, i32)) -> Option<Vec<(i32, i32)>> {
    let (b, h) = (r.b, r.h);
    let ok = |x: i32, y: i32| x >= 0 && y >= 0 && x < b && y < h;
    // Start und Ziel koennen auf Gebaeuden liegen: ihr Umfeld zaehlt als frei
    let nahe = |x: i32, y: i32, p: (i32, i32), rr: i32| (x - p.0).abs() <= rr && (y - p.1).abs() <= rr;
    let frei = |x: i32, y: i32| ok(x, y) && (r.frei[(y * b + x) as usize] || nahe(x, y, start, 2) || nahe(x, y, ziel, 3));
    let n = (b * h) as usize;
    let mut d = vec![u32::MAX; n]; let mut von = vec![u32::MAX; n];
    let mut q = BinaryHeap::new();
    if !ok(start.0, start.1) || !ok(ziel.0, ziel.1) { return None; }
    let si = (start.1 * b + start.0) as usize; d[si] = 0; q.push(Reverse((0u32, si as u32)));
    let zi = (ziel.1 * b + ziel.0) as usize;
    while let Some(Reverse((dv, i))) = q.pop() {
        let i = i as usize; if dv > d[i] { continue; }
        if i == zi { break; }
        let (x, y) = ((i as i32) % b, (i as i32) / b);
        for (dx, dy, c) in N8 {
            let (nx, ny) = (x + dx, y + dy);
            if !frei(nx, ny) { continue; }
            if dx != 0 && dy != 0 && !frei(x + dx, y) && !frei(x, y + dy) { continue; }
            let j = (ny * b + nx) as usize;
            let nd = dv + c as u32 * (1 + strafe[j] as u32);
            if nd < d[j] { d[j] = nd; von[j] = i as u32; q.push(Reverse((nd, j as u32))); }
        }
    }
    if d[zi] == u32::MAX { return None; }
    let mut w = vec![ziel]; let mut c = zi;
    while c != si { c = von[c] as usize; w.push(((c as i32) % b, (c as i32) / b)); if w.len() > 8192 { return None; } }
    w.reverse();
    Some(w)
}

fn laenge(w: &[(i32, i32)]) -> f32 { w.windows(2).map(|p| if p[0].0 != p[1].0 && p[0].1 != p[1].1 { 1.414 } else { 1.0 }).sum() }

/// Bis zu MAX_ROUTEN verschiedene Wege von `start` nach `ziel` (Kacheln). Weitere Wege hoechstens 1,8 x so lang wie der
/// kuerzeste und hoechstens 60 % ihrer Kacheln im Streifen (2 Kacheln) um einen frueheren Weg.
pub fn routen_raster(r: &Raster, start: (i32, i32), ziel: (i32, i32)) -> Vec<Route> {
    let n = (r.b * r.h) as usize;
    let mut strafe = vec![0u16; n];
    let mut nah = vec![false; n];
    let mut out: Vec<Route> = Vec::new();
    for _versuch in 0..MAX_ROUTEN + 2 {
        if out.len() >= MAX_ROUTEN { break; }
        let Some(w) = dijkstra(r, &strafe, start, ziel) else { break };
        let l = laenge(&w);
        let mitte = &w[4.min(w.len())..w.len().saturating_sub(10)];
        if let Some(k) = out.first() {
            if l > 1.8 * k.laenge { break; }
            let geteilt = mitte.iter().filter(|&&(x, y)| nah[(y * r.b + x) as usize]).count();
            if !mitte.is_empty() && geteilt * 10 > mitte.len() * 6 {
                // derselbe Zugang: Strafe erhoehen und weitersuchen
                for &(x, y) in mitte { band(r, &mut strafe, &mut nah, x, y, false); }
                continue;
            }
        }
        for &(x, y) in mitte { band(r, &mut strafe, &mut nah, x, y, true); }
        let mut punkte: Vec<(i32, i32)> = Vec::new();
        let ende = w.len().saturating_sub(WP_VOR_ZIEL);
        let mut i = WP_SCHRITT;
        while i < ende && punkte.len() < MAX_WP { punkte.push(w[i]); i += WP_SCHRITT; }
        if ende > 0 && punkte.len() < MAX_WP && punkte.last() != Some(&w[ende - 1]) { punkte.push(w[ende - 1]); }
        out.push(Route { weg: w, punkte, laenge: l });
    }
    out
}

fn band(r: &Raster, strafe: &mut [u16], nah: &mut [bool], x: i32, y: i32, markieren: bool) {
    for dy in -3..=3 { for dx in -3..=3 {
        let (nx, ny) = (x + dx, y + dy);
        if nx < 0 || ny < 0 || nx >= r.b || ny >= r.h { continue; }
        let j = (ny * r.b + nx) as usize;
        strafe[j] = strafe[j].saturating_add(if dx.abs() <= 1 && dy.abs() <= 1 { 6 } else { 3 }).min(60);
        if markieren && dx.abs() <= 2 && dy.abs() <= 2 { nah[j] = true; }
    } }
}

/// Routen mit Zwischenspeicher (Start auf 4 Kacheln gerundet, je 1000 Ticks neu)
pub fn routen(welt: &mut dyn Welt, z: &Zustand, start: (i32, i32), ziel: (i32, i32)) -> std::rc::Rc<Vec<Route>> {
    let key = (z.spieler, start.0 / 4, start.1 / 4, ziel.0 * 1024 + ziel.1, welt.Tick() / 1000);
    if let Some(v) = CACHE.with(|c| c.borrow().iter().find(|e| e.0 == key).map(|e| e.1.clone())) { return v; }
    let r = raster(welt, z);
    let v = std::rc::Rc::new(routen_raster(&r, start, ziel));
    CACHE.with(|c| { let mut c = c.borrow_mut(); c.retain(|e| e.0 .4 == key.4); if c.len() > 64 { c.clear(); } c.push((key, v.clone())); });
    v
}

/// Merkmale einer Route: Feindarmee nach kleinstem Abstand zum Weg (voll bis 5, nichts ab 20 Kacheln), Tuerme in
/// 7 Kacheln des Wegs; beides ohne die letzten 8 Wegkacheln (die zaehlen als Abwehr am Ziel). Je eigenem Wert.
pub fn bewerten(r: &Raster, rs: &[Route], k: usize, feind: &[(i32, i32, i32)], tuerme: &[(i32, i32, i32)], eigen: f64) -> RoutenInfo {
    let ro = &rs[k];
    let kurz = rs[0].laenge.max(1.0);
    let teil = &ro.weg[..ro.weg.len().saturating_sub(8)];
    let dmin = |p: (i32, i32)| -> f64 {
        let mut m = i32::MAX;
        for q in teil.iter().step_by(2) { let d = (q.0 - p.0).pow(2) + (q.1 - p.1).pow(2); if d < m { m = d; } }
        if m == i32::MAX { 1e3 } else { (m as f64).sqrt() }
    };
    let mut ga = 0.0; let mut mf = 1e3f64;
    for &(x, y, w) in feind { let d = dmin((x, y)); mf = mf.min(d); ga += w as f64 * (1.0 - (d - 5.0) / 15.0).clamp(0.0, 1.0); }
    let mut gt = 0.0;
    for &(x, y, w) in tuerme { if dmin((x, y)) <= 7.0 { gt += w as f64; } }
    RoutenInfo {
        laenge: ro.laenge, umweg: ro.laenge / kurz - 1.0,
        gefahrArmee: (ga / eigen.max(1.0)).min(4.0) as f32, gefahrTurm: (gt / eigen.max(1.0)).min(4.0) as f32,
        minFeind: (mf / 50.0).min(1.5) as f32, breite: (breite(r, &ro.weg) as f32 / 10.0).min(3.0), anzahl: rs.len() as f32 / 3.0,
    }
}

/// Freiraum an einer Kachel: Schachbrettabstand zum naechsten Hindernis (hoechstens 8); Gangbreite = 2 x Freiraum - 1
pub fn freiraum(r: &Raster, x: i32, y: i32) -> i32 {
    let frei = |x: i32, y: i32| x >= 0 && y >= 0 && x < r.b && y < r.h && r.frei[(y * r.b + x) as usize];
    for d in 1..8 { for o in -d..=d { if !frei(x + o, y - d) || !frei(x + o, y + d) || !frei(x - d, y + o) || !frei(x + d, y + o) { return d; } } }
    8
}

/// Schmalste Stelle eines Wegs (unteres Viertel der Gangbreite je Wegkachel, ohne Anfang und die letzten 8 Kacheln)
fn breite(r: &Raster, w: &[(i32, i32)]) -> i32 {
    let frei = |x: i32, y: i32| x >= 0 && y >= 0 && x < r.b && y < r.h && r.frei[(y * r.b + x) as usize];
    let raum = |x: i32, y: i32| -> i32 { for d in 1..8 { for o in -d..=d { if !frei(x + o, y - d) || !frei(x + o, y + d) || !frei(x - d, y + o) || !frei(x + d, y + o) { return d; } } } 8 };
    let teil = &w[2.min(w.len())..w.len().saturating_sub(8)];
    if teil.is_empty() { return 16; }
    let mut v: Vec<i32> = teil.iter().step_by(2).map(|&(x, y)| {
        let mut m = 0; for dy in -2..=2 { for dx in -2..=2 { if frei(x + dx, y + dy) { m = m.max(raum(x + dx, y + dy)); } } } 2 * m - 1
    }).collect();
    v.sort();
    v[v.len() / 4]
}
