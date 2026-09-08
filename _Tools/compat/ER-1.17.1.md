# ER 1.17.1 static compatibility audit

Date: 2026-09-08. Source baseline: 5942689.
Installed Worldwide PC EXE: file/product version 2.7.1.0, 87,042,128 bytes.
SHA-256: `1A3547101327F65D0C76DA2F9190AC0AA66871EA42BAE2AECC61E11A8B597891`.

| Target | 2.7.0.0 RVA | 2.7.1.0 RVA |
| --- | --- | --- |
| Movie setup | 0xE22D90 | 0xE22E00 |
| Movie init | 0xE230E0 | 0xE23150 |
| State step | 0xE22720 | 0xE22790 |
| State 0 | 0xE230B0 | 0xE23120 |
| State 1 | 0xE23550 | 0xE235C0 |
| Resource ready | 0x1EDE730 | 0x1EDE7A0 |
| Ready return address | 0xE235D7 | 0xE23647 |
| Movie tick | 0xE23970 | 0xE239E0 |
| Movie render | 0xE233C0 | 0xE23430 |
| Draw submit | 0x1AEC7A0 | 0x1AEC810 |
| Bink open wrapper | 0x1E85CE0 | 0x1E85D50 |
| State-7 cleanup | 0xE22E90 | 0xE22F00 |
| MovieImp store instruction | 0xDF19BD | 0xDF1A2D |
| MovieImp global | 0x458B928 | 0x458B928 |
| StaffRoll last slot | 0x747CD0 | 0x747CD0 |
| StaffRoll constructor | 0x8BEF00 | 0x8BEF00 |

The existing setup and state-7 signatures each occur exactly once in the new
EXE. The state marker still copies +0x44 to +0x40; cleanup still accesses
+0xA8/+0xB8. The RIP-relative MovieImp store resolves to 0x458B928.
The call at 0xE23642 resolves to resource-ready 0x1EDE7A0 and returns to
0xE23647. All ten diagnostic entry signatures pass at the listed addresses.
These are address/signature checks, not a proof that entire functions or every
engine field are unchanged.

## Pinned fsrs dependency

The pinned revision remains eae96dfec94fd9cf6f9d24813c8d08f72019f243.
Its ER rva::get() does not support 2.7.1.0. This project does not call that
dispatcher: it uses FromStatic/FromSingleton name-based discovery and direct
fields of CSMenuManImp, CSNowLoadingHelper, CSFeManImp, WorldChrMan and CSTaskImp.
Task-group iteration is a field read, not task registration. Subclass Deref
uses the embedded base field; it does not perform a VMT/version lookup.
Therefore a dependency version-dispatch patch is not needed for current call
sites. Adding future fsrs native methods requires re-auditing this assumption.

## Verification and limits

- New EXE: checker passes with exact SHA-256, native signatures and call linkage.
- Retained 2.6.2.0 baseline: checker passes. 2.7.0.0 profile is unchanged;
  its earlier binary was not re-tested during this audit.
- Locked offline release build and both Rust tests pass; diff check passes.
- DX12 bridge and one-shot movie lifecycle were not modified.
- Loading timer +0x734, loading flags +0xEC..+0xEF, HUD/player/task field
  reads remain as before; their current in-game semantics are not proven by
  the Movie signature checks or the existing type-size tests.
- Required in-game checks: title animation, direct new game/CG skip and end,
  normal gameplay FPS, other in-game movies, and frozen return-title frame.
