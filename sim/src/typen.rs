//! Einheitentypen mit den map_id-Nummern des Originals (erzeugt aus gamedata.json des Harness). Generated: unit type ids.
#![allow(dead_code)]
pub type Typ = u16;
pub const KEINER: Typ = 0;
pub const BIG_TRUCK: Typ = 1;
pub const CON_TRUCK: Typ = 2;
pub const SPIDER: Typ = 3;
pub const SCORPION: Typ = 4;
pub const LIGHT_TANK: Typ = 5;
pub const MED_TANK: Typ = 6;
pub const BIG_TANK: Typ = 7;
pub const SURVEYOR: Typ = 8;
pub const MINER: Typ = 9;
pub const GEO_METRO: Typ = 10;
pub const SCOUT: Typ = 11;
pub const BULL_DOZER: Typ = 12;
pub const MOBILE_HOME: Typ = 13;
pub const REPAIR_TRUCK: Typ = 14;
pub const WALL_TRUCK: Typ = 15;
pub const SCAT: Typ = 16;
pub const TUBE: Typ = 17;
pub const WALL: Typ = 18;
pub const WALL_LAVA: Typ = 19;
pub const WALL_MICROBE: Typ = 20;
pub const MINE: Typ = 21;
pub const MINE_ADV: Typ = 22;
pub const TOWER_GUARD: Typ = 23;
pub const TOWER_LIGHT: Typ = 24;
pub const STORAGE_ORE: Typ = 25;
pub const STORAGE_RARE: Typ = 26;
pub const FORUM: Typ = 27;
pub const COMMAND: Typ = 28;
pub const ASE: Typ = 29;
pub const RESIDENCE: Typ = 30;
pub const ROBOT_COMMAND: Typ = 31;
pub const TRADE: Typ = 32;
pub const LAB: Typ = 33;
pub const MED_CENTER: Typ = 34;
pub const NURSERY: Typ = 35;
pub const SOLAR: Typ = 36;
pub const RECREATION: Typ = 37;
pub const UNIVERSITY: Typ = 38;
pub const AGRIDOME: Typ = 39;
pub const DIRT: Typ = 40;
pub const GARAGE: Typ = 41;
pub const MAGMA_WELL: Typ = 42;
pub const METEOR_DEF: Typ = 43;
pub const GEOTHERMAL: Typ = 44;
pub const FACT_ANDROID: Typ = 45;
pub const FACT_LUXURY: Typ = 46;
pub const FACT_STRUCTURE: Typ = 47;
pub const FACT_VEHICLE: Typ = 48;
pub const LAB_STANDARD: Typ = 49;
pub const LAB_ADV: Typ = 50;
pub const OBSERVATORY: Typ = 51;
pub const RESIDENCE_P: Typ = 52;
pub const RESIDENCE_E: Typ = 53;
pub const SMELTER: Typ = 54;
pub const SPACEPORT: Typ = 55;
pub const SMELTER_ADV: Typ = 56;
pub const GORF: Typ = 57;
pub const TOKAMAK: Typ = 58;
pub const ACID: Typ = 59;
pub const EMP: Typ = 60;
pub const LASER: Typ = 61;
pub const MICROWAVE: Typ = 62;
pub const RAIL_GUN: Typ = 63;
pub const CANNON: Typ = 64;
pub const SELF_DESTRUCT: Typ = 65;
pub const SELF_DESTRUCT_ADV: Typ = 66;
pub const TURRET_DESTRUCT: Typ = 67;
pub const TURRET_DESTRUCT_ADV: Typ = 68;
pub const SCUTTLE_DESTRUCT: Typ = 69;
pub const SPAM: Typ = 70;
pub const FOAM: Typ = 71;
pub const THORS_HAMMER: Typ = 72;
pub const SCORPION_WEAPON: Typ = 73;
pub const EMP_BIG: Typ = 74;
pub const BFG: Typ = 75;
pub const LIGHTNING: Typ = 76;
pub const TORNADO: Typ = 77;
pub const EARTHQUAKE: Typ = 78;
pub const ERUPTION: Typ = 79;
pub const METEOR: Typ = 80;
pub const BEACON: Typ = 81;
pub const MAGMA_VENT: Typ = 82;
pub const FUMAROLE: Typ = 83;
pub const WRECK_FLAG: Typ = 84;
pub const BLD_EXPL_SML: Typ = 85;
pub const BLD_EXPL_MED: Typ = 86;
pub const BLD_EXPL_BIG: Typ = 87;
pub const EDWARD: Typ = 88;
pub const SOLAR_SAT: Typ = 89;
pub const SPACE_1: Typ = 90;
pub const SPACE_2: Typ = 91;
pub const SPACE_3: Typ = 92;
pub const SPACE_4: Typ = 93;
pub const SPACE_5: Typ = 94;
pub const SPACE_6: Typ = 95;
pub const SPACE_7: Typ = 96;
pub const SPACE_8: Typ = 97;
pub const SPACE_9: Typ = 98;
pub const SPACE_10: Typ = 99;
pub const SPACE_11: Typ = 100;
pub const SPACE_12: Typ = 101;
pub const SPACE_13: Typ = 102;
pub const SPACE_14: Typ = 103;
pub const SPACE_15: Typ = 104;
pub const SULV: Typ = 105;
pub const RLV: Typ = 106;
pub const EMP_MISSILE: Typ = 107;
pub const LUX_1: Typ = 108;
pub const LUX_2: Typ = 109;
pub const LUX_3: Typ = 110;
pub const SHUTTLE: Typ = 111;
pub const SPIDER_PACK: Typ = 112;
pub const SCORPION_PACK: Typ = 113;
pub const AMBIENT_ANIM: Typ = 114;
/// (map_id, Code wie in op2-sim/sheets, Anzeigename)
pub const NAMEN: [(Typ, &str, &str); 114] = [
    (1, "BIG_TRUCK", "Transporter"),
    (2, "CON_TRUCK", "KonFaz"),
    (3, "SPIDER", "Spinne"),
    (4, "SCORPION", "Skorpion"),
    (5, "LIGHT_TANK", "Lynx"),
    (6, "MED_TANK", "Panther"),
    (7, "BIG_TANK", "Tiger"),
    (8, "SURVEYOR", "Robot-Vermesser"),
    (9, "MINER", "Robo-Miner"),
    (10, "GEO_METRO", "GeoFaz"),
    (11, "SCOUT", "Scout"),
    (12, "BULL_DOZER", "Robo-Dozer"),
    (13, "MOBILE_HOME", "Evakuierungstransporter"),
    (14, "REPAIR_TRUCK", "Reparaturfahrzeug"),
    (15, "WALL_TRUCK", "Grabfahrzeug"),
    (16, "SCAT", "Lufttransporter mit geringer Kapazität"),
    (17, "TUBE", "Röhre"),
    (18, "WALL", "Wand"),
    (19, "WALL_LAVA", "Lavawand"),
    (20, "WALL_MICROBE", "Mikrobenwand"),
    (21, "MINE", "Gewöhnliche Erzmine "),
    (22, "MINE_ADV", "Spezielle Erzmine "),
    (23, "TOWER_GUARD", "Wachposten"),
    (24, "TOWER_LIGHT", "Leuchtturm"),
    (25, "STORAGE_ORE", "Gewöhnliches Lager"),
    (26, "STORAGE_RARE", "Spezielles Lager"),
    (27, "FORUM", "Forum"),
    (28, "COMMAND", "Kommandozentrale"),
    (29, "ASE", "MHD-Generator"),
    (30, "RESIDENCE", "Wohnmodul"),
    (31, "ROBOT_COMMAND", "Robot-Kommandozentrale"),
    (32, "TRADE", "Handelszentrum"),
    (33, "LAB", "Labor Klasse 1"),
    (34, "MED_CENTER", "Medizinisches Zentrum"),
    (35, "NURSERY", "Kindergarten"),
    (36, "SOLAR", "Sonnenenergie-Anlage"),
    (37, "RECREATION", "Freizeitanlage"),
    (38, "UNIVERSITY", "Universität"),
    (39, "AGRIDOME", "Agrarkuppel"),
    (40, "DIRT", "KEIT"),
    (41, "GARAGE", "Garage"),
    (42, "MAGMA_WELL", "Magmabrunnen"),
    (43, "METEOR_DEF", "Meteorverteidigung"),
    (44, "GEOTHERMAL", "Erdwärmekraftwerk"),
    (45, "FACT_ANDROID", "Arachnidenfabrik"),
    (46, "FACT_LUXURY", "Verbrauchsgüterfabrik"),
    (47, "FACT_STRUCTURE", "Gebäudefabrik"),
    (48, "FACT_VEHICLE", "Fahrzeugfabrik"),
    (49, "LAB_STANDARD", "Labor Klasse 2"),
    (50, "LAB_ADV", "Labor Klasse 3"),
    (51, "OBSERVATORY", "Observatorium"),
    (52, "RESIDENCE_P", "Geschütztes Wohnmodul"),
    (53, "RESIDENCE_E", "Gehobenes Wohnmodul"),
    (54, "SMELTER", "Gewöhnlicher Hochofen"),
    (55, "SPACEPORT", "Raumhafen"),
    (56, "SMELTER_ADV", "Spezialhochofen"),
    (57, "GORF", "MER"),
    (58, "TOKAMAK", "Tokamak"),
    (59, "ACID", "Säurewolke"),
    (60, "EMP", "EMP"),
    (61, "LASER", "Laser"),
    (62, "MICROWAVE", "Mikrowelle"),
    (63, "RAIL_GUN", "Trägergewehr"),
    (64, "CANNON", "RGG"),
    (65, "SELF_DESTRUCT", "Sternfackel"),
    (66, "SELF_DESTRUCT_ADV", "Supernova"),
    (67, "TURRET_DESTRUCT", "Sternfackel"),
    (68, "TURRET_DESTRUCT_ADV", "Supernova"),
    (69, "SCUTTLE_DESTRUCT", "Normale Fahrzeug-Selbstzerstörung "),
    (70, "SPAM", "ESG"),
    (71, "FOAM", "Klebschaum"),
    (72, "THORS_HAMMER", "Thors Hammer"),
    (73, "SCORPION_WEAPON", ""),
    (74, "EMP_BIG", "Schaden durch EMP-Rakete"),
    (75, "BFG", ""),
    (76, "LIGHTNING", "Sturm"),
    (77, "TORNADO", "Wirbelsturm"),
    (78, "EARTHQUAKE", "Erdbeben"),
    (79, "ERUPTION", "Vulkanausbruch"),
    (80, "METEOR", "Meteor"),
    (81, "BEACON", "Minensignale"),
    (82, "MAGMA_VENT", "Magmaschächte"),
    (83, "FUMAROLE", "Dampfquellen"),
    (84, "WRECK_FLAG", ""),
    (85, "BLD_EXPL_SML", "Heftige Explosion eines Gebäudes"),
    (86, "BLD_EXPL_MED", "Gewaltige Explosion eines Gebäudes"),
    (87, "BLD_EXPL_BIG", "Katastrophale Explosion eines Gebäudes"),
    (88, "EDWARD", "EDWARD-Satellit"),
    (89, "SOLAR_SAT", "Solar-Satellit"),
    (90, "SPACE_1", "Ionen-Antriebmodul"),
    (91, "SPACE_2", "Fusions-Antriebmodul"),
    (92, "SPACE_3", "Kommando-Modul"),
    (93, "SPACE_4", "Treibstoffmodul"),
    (94, "SPACE_5", "Wohnraumring"),
    (95, "SPACE_6", "Sensoreinheit"),
    (96, "SPACE_7", "Weltraumdock"),
    (97, "SPACE_8", "Stasis-Systeme"),
    (98, "SPACE_9", "Orbital-Einheit"),
    (99, "SPACE_10", "Phoenix-Modul"),
    (100, "SPACE_11", "Spezialerz-Ladung"),
    (101, "SPACE_12", "Gewöhn.-Erz-Ladung"),
    (102, "SPACE_13", "Nahrungsmittel-Ladung"),
    (103, "SPACE_14", "Evakuierungsmodul"),
    (104, "SPACE_15", "Kindermodul"),
    (105, "SULV", "Rakete"),
    (106, "RLV", "Shuttle"),
    (107, "EMP_MISSILE", "EMP-Rakete"),
    (108, "LUX_1", "Impuls-Gegenstände"),
    (109, "LUX_2", "Waren"),
    (110, "LUX_3", "Luxuswaren"),
    (111, "SHUTTLE", "Interkolonie-Shuttle"),
    (112, "SPIDER_PACK", "Spinne 3er-Pack"),
    (113, "SCORPION_PACK", "Skorpion 3er-Pack"),
    (114, "AMBIENT_ANIM", "Hübsch, nicht?"),
];
pub fn code(t: Typ) -> &'static str { NAMEN.iter().find(|n| n.0 == t).map(|n| n.1).unwrap_or("?") }
pub fn von_code(c: &str) -> Typ { NAMEN.iter().find(|n| n.1.eq_ignore_ascii_case(c)).map(|n| n.0).unwrap_or(0) }
