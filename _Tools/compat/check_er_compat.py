#!/usr/bin/env python3
"""Validate dynamic-title-bg's native targets in an Elden Ring executable."""

from __future__ import annotations

import argparse
import hashlib
import struct
import sys
from dataclasses import dataclass
from pathlib import Path


SETUP_SIGNATURE = bytes.fromhex(
    "48895c2408574883ec3080b93001000000498bc00f297424200fb6fa0f28f3"
    "488bd90f85850000004881c1c000000066"
)
CLEANUP_SIGNATURE = bytes.fromhex(
    "40534883ec20488bd933d2488b89a8000000e859e8ffff488b8bb80000004885"
    "c97406488b01ff5048"
)
STATE_LAYOUT_SIGNATURE = bytes.fromhex("486343448943404c397358")


@dataclass(frozen=True)
class Layout:
    name: str
    sha256: str
    setup_rva: int
    movie_imp_global_rva: int
    state_step_rva: int
    cleanup_state7_rva: int
    movie_imp_store_rva: int
    diagnostics: tuple[tuple[str, int, bytes], ...]


COMMON_DIAGNOSTIC_SIGNATURES = (
    ("movie_open_wrapper", bytes.fromhex("4055535657415441")),
    ("movie_ins_init", bytes.fromhex("488bc45541564157")),
    ("movie_state0", bytes.fromhex("40534883ec20488b")),
    ("movie_state1", bytes.fromhex("40534883ec6048c7")),
    ("movie_resource_ready", bytes.fromhex("40534883ec20488b")),
    ("movie_tick", bytes.fromhex("40534883ec20488b")),
    ("movie_render", bytes.fromhex("40534883ec30488b")),
    ("movie_draw_submit", bytes.fromhex("48895c240848896c")),
    ("staffroll_last_slot", bytes.fromhex("4889542410534881")),
    ("staffroll_ctor", bytes.fromhex("488bc45557415648")),
)


def diagnostics(rvas: tuple[int, ...]) -> tuple[tuple[str, int, bytes], ...]:
    return tuple(
        (name, rva, signature)
        for (name, signature), rva in zip(COMMON_DIAGNOSTIC_SIGNATURES, rvas)
    )


LAYOUTS = (
    Layout(
        name="ER 2.7.0.0",
        sha256="D1A84083C6C7C7902162FF098F7D86812839AA6B3575959398857E539C488134",
        setup_rva=0xE22D90,
        movie_imp_global_rva=0x458B928,
        state_step_rva=0xE22720,
        cleanup_state7_rva=0xE22E90,
        movie_imp_store_rva=0xDF19BD,
        diagnostics=diagnostics(
            (
                0x1E85CE0,
                0xE230E0,
                0xE230B0,
                0xE23550,
                0x1EDE730,
                0xE23970,
                0xE233C0,
                0x1AEC7A0,
                0x747CD0,
                0x8BEF00,
            )
        ),
    ),
    Layout(
        name="ER 2.6.2.0",
        sha256="34102B1C08BB5F769A724427A6F70FE29B3B732C31CF73693F861C48D3492DDB",
        setup_rva=0xE20F90,
        movie_imp_global_rva=0x45878A8,
        state_step_rva=0xE20920,
        cleanup_state7_rva=0xE21090,
        movie_imp_store_rva=0xDEFBBD,
        diagnostics=diagnostics(
            (
                0x1E83EE0,
                0xE212E0,
                0xE212B0,
                0xE21750,
                0x1EDC930,
                0xE21B70,
                0xE215C0,
                0x1AEA9A0,
                0x746E80,
                0x8BDD60,
            )
        ),
    ),
)


class PeImage:
    def __init__(self, path: Path) -> None:
        self.path = path
        self.data = path.read_bytes()
        if self.data[:2] != b"MZ":
            raise ValueError("missing DOS MZ signature")
        pe_offset = struct.unpack_from("<I", self.data, 0x3C)[0]
        if self.data[pe_offset : pe_offset + 4] != b"PE\0\0":
            raise ValueError("missing PE signature")
        coff = pe_offset + 4
        section_count = struct.unpack_from("<H", self.data, coff + 2)[0]
        optional_size = struct.unpack_from("<H", self.data, coff + 16)[0]
        optional = coff + 20
        if struct.unpack_from("<H", self.data, optional)[0] != 0x20B:
            raise ValueError("expected a PE32+ image")
        self.image_size = struct.unpack_from("<I", self.data, optional + 56)[0]
        section_table = optional + optional_size
        self.sections: list[tuple[int, int, int, int]] = []
        for index in range(section_count):
            entry = section_table + index * 40
            virtual_size, rva, raw_size, raw_offset = struct.unpack_from(
                "<IIII", self.data, entry + 8
            )
            self.sections.append((rva, virtual_size, raw_offset, raw_size))

    def read_rva(self, rva: int, size: int) -> bytes:
        for section_rva, virtual_size, raw_offset, raw_size in self.sections:
            span = max(virtual_size, raw_size)
            if section_rva <= rva and rva + size <= section_rva + span:
                relative = rva - section_rva
                if relative + size > raw_size:
                    raise ValueError(f"RVA 0x{rva:X} is not backed by file data")
                return self.data[raw_offset + relative : raw_offset + relative + size]
        raise ValueError(f"RVA 0x{rva:X} is outside mapped sections")


def matches(image: PeImage, rva: int, expected: bytes) -> bool:
    try:
        return image.read_rva(rva, len(expected)) == expected
    except ValueError:
        return False


def layout_errors(image: PeImage, layout: Layout) -> list[str]:
    errors: list[str] = []
    if not matches(image, layout.setup_rva, SETUP_SIGNATURE):
        errors.append(f"setup signature mismatch at 0x{layout.setup_rva:X}")
    if not matches(image, layout.cleanup_state7_rva, CLEANUP_SIGNATURE):
        errors.append(f"state-7 cleanup mismatch at 0x{layout.cleanup_state7_rva:X}")
    if not matches(image, layout.state_step_rva + 0x40, STATE_LAYOUT_SIGNATURE):
        errors.append(f"MovieIns state layout mismatch at 0x{layout.state_step_rva + 0x40:X}")

    try:
        store = image.read_rva(layout.movie_imp_store_rva, 7)
        if store[:3] != b"\x48\x89\x05":
            errors.append(
                f"MovieImp global store opcode mismatch at 0x{layout.movie_imp_store_rva:X}"
            )
        else:
            displacement = struct.unpack_from("<i", store, 3)[0]
            target = layout.movie_imp_store_rva + 7 + displacement
            if target != layout.movie_imp_global_rva:
                errors.append(
                    "MovieImp global store targets "
                    f"0x{target:X}, expected 0x{layout.movie_imp_global_rva:X}"
                )
    except ValueError as error:
        errors.append(str(error))

    for name, rva, signature in layout.diagnostics:
        if not matches(image, rva, signature):
            errors.append(f"diagnostic {name} mismatch at 0x{rva:X}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("exe", type=Path, help="path to eldenring.exe")
    args = parser.parse_args()

    try:
        image = PeImage(args.exe)
    except (OSError, ValueError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 2

    digest = hashlib.sha256(image.data).hexdigest().upper()
    print(f"Executable : {image.path}")
    print(f"Size       : {len(image.data):,} bytes")
    print(f"Image size : 0x{image.image_size:X}")
    print(f"SHA-256    : {digest}")

    for layout in LAYOUTS:
        errors = layout_errors(image, layout)
        if errors:
            continue
        hash_status = "exact" if digest == layout.sha256 else "signature-only"
        print(f"PASS       : {layout.name} ({hash_status} hash match)")
        print(f"Setup RVA  : 0x{layout.setup_rva:X}")
        print(f"MovieImp   : 0x{layout.movie_imp_global_rva:X}")
        print(f"Cleanup    : 0x{layout.cleanup_state7_rva:X}")
        print(f"Diagnostics: {len(layout.diagnostics)} targets validated")
        return 0

    print("FAIL: executable does not match a supported ER layout", file=sys.stderr)
    for layout in LAYOUTS:
        print(f"  {layout.name}:", file=sys.stderr)
        for error in layout_errors(image, layout):
            print(f"    - {error}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
