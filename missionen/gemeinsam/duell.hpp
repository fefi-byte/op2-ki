// duell.hpp -- Startplaetze und Spielende fuer zwei Seiten (Spieler 1 und 2, bei "Du gegen die KI" du an Platz 2).
// Nicht Teil der KI.
#pragma once
#include <cstdlib>
#include "op2_log.hpp"

namespace startwahl {
// Der Seed (Umgebungsvariable OP2H_SEED, sonst Zufall) waehlt das Startplatz-Paar aus den n Plaetzen X/Y der Karte.
// Ergebnis: Spieler 1 an Platz a, Spieler 2 an Platz b.
inline int a = -1, b = -1;
inline void paar(int n, const int* X, const int* Y) {
  if (a >= 0) return;
  int s = 0; char e[32];
  if (GetEnvironmentVariableA("OP2H_SEED", e, sizeof e)) s = std::atoi(e);
#ifdef OP2KI_FEST_SEED
  else s = OP2KI_FEST_SEED;   // Mission mit festem Startplatz-Paar (wie --seed im Simulator)
#else
  else s = TethysGame::GetRand(1000) + 1;
#endif
  const int k = ((s - 1) % (n * (n - 1)) + n * (n - 1)) % (n * (n - 1));
  a = k / (n - 1); b = k % (n - 1); if (b >= a) b++;
  op2::log::linef("Startwahl: Seed %d -> Spieler 1 an Platz %d (%d,%d), Spieler 2 an Platz %d (%d,%d) von %d", s, a, X[a], Y[a], b, X[b], Y[b], n);
}
}  // namespace startwahl

namespace spielende {
// Kerngebaeude eines Spielers; wer keines mehr hat, hat verloren.
inline int kern(int p, bool nurLebende) {
  static const map_id T[] = { mapCommandCenter, mapStructureFactory, mapVehicleFactory, mapCommonOreSmelter, mapTokamak,
                              mapAgridome, mapResidence, mapStandardLab, mapAdvancedLab };
  int n = 0;
  for (map_id t : T) { PlayerBuildingEnum e(p, t); Unit u; while (e.GetNext(u)) if (!nurLebende || u.IsLive()) ++n; }
  return n;
}
}  // namespace spielende
