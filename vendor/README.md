# source2-demo 0.5.10

This is the crates.io 0.5.10 source snapshot, retaining its MIT / Apache-2.0
licenses. Local patches are tracked here rather than modifying a Cargo cache:

* CS2 string-table decoding is limited to `instancebaseline` and `userinfo`,
  preserving all table indices. Resource/engine tables are not match data and
  their binary keys must not be interpreted as null-terminated text. Empty
  string-table payloads are ignored. This matches demoinfocs-golang's policy.
* A zero final-file-info offset is accepted for streaming/server/POV recordings;
  no fabricated player or score data is supplied. Actual parsed ticks and the
  recorded server tick interval determine duration.
* Packet message lengths are checked against the remaining payload before
  allocation/read. The application also validates the outer command envelopes
  and reports truncated downloads precisely.
* Preserve the recorded polymorphic game-mode selector instead of discarding
  it. Decode nested game-mode fields against the declared derived serializer
  (wire selector zero = base, 1..N = derived types). This fixes Deathmatch and
  AnimGraph2 protocol fixtures with an empty `CCSGameModeRules` base serializer.
* Decode `fixed8` / `fixed16` / `fixed32` integer fields (including array and
  vector elements) as raw bits instead of varints, keeping each field's
  declared integer type and sign-extending signed fields. The 2026-11-11 CS2
  update started sending `CCSPlayerController` fields such as
  `m_recentKillQueue` (`uint8[8]`) and `m_iCompetitiveRankType` this way, and a
  varint read of values >= 128 desynced the entity stream. Matches
  demoinfocs-golang v5.3.0 (#764 / #765).

Application-side compatibility handling additionally derives round results
only from observed, stable-roster score increments when modern demos omit
`round_start` / `round_end`, and uses recorded `round_poststart` boundaries.
Voice packet offsets are cumulative end positions; only `num_packets` entries
are used, ignoring the engine's unused padding entries.

Validate changes with real demo fixtures as well as root crate tests before
updating this snapshot. The native application's personal-use license does not
replace this dependency's licenses.
