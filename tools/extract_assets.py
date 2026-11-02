#!/usr/bin/env python3
"""Extrae y organiza los assets de Plants vs. Zombies (main.pak).

Formato del contenedor (reverse engineered de la copia de STEAM):

    cabecera:  c0 4a c0 ba 00 00 00 00 00   (9 bytes)
    directorio: repetido hasta un byte 0x00
        u8   nombre_len
        u8[] nombre          (ruta con '\\')
        u32  tamaño_registro (incluye la cabecera del registro)
        u32  checksum        (no validado; no es CRC32 estándar)
        u32  desconocido
        u8   flag            (0 normal, 128 visto en el último archivo)
    datos (en el mismo orden que el directorio):
        si empieza por d4 fe ad de:
            u32 magia
            u32 tamaño_descomprimido
            zlib( tamaño_registro - 8 bytes )
        si no:
            bytes crudos de tamaño_registro

Todo el archivo está XOR-eado byte a byte con 0xF7.

Uso:
    python3 tools/extract_assets.py                 # desde la ruta por defecto
    python3 tools/extract_assets.py --pak RUTA --out assets
    python3 tools/extract_assets.py --force         # sobrescribe existentes

Diseño de salida (el mismo criterio que ya usa el proyecto):

    assets/animations/<Entidad>/*.reanim
    assets/textures/<Entidad>/*.png|jpg
    assets/images/**                 (UI, fondos, zombatar, ...)
    assets/particles/**              (XML + PNG de partículas)
    assets/sounds/**                 (ogg/au/mo3)
    assets/data/**                   (fuentes)
    assets/properties/**             (XML/TXT del juego)
    assets/compiled/**               (versiones compiladas, opcionales)

Los archivos que ya existen NO se sobrescriben salvo con --force, para no
perder ajustes manuales (p. ej. assets/animations/Zombie/Zombie.reanim).
"""

from __future__ import annotations

import argparse
import hashlib
import re
import struct
import sys
import zlib
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path

XOR_KEY = 0xF7
MAGIC_DIR = bytes.fromhex("c04ac0ba")
MAGIC_RECORD = bytes.fromhex("d4feadde")

DEFAULT_PAK = Path("/home/lukas/Proyectos/JUEGOS PARA REA/PVZ/main.pak")
DEFAULT_OUT = Path("assets")

# Texturas del .reanim que pertenecen a una entidad concreta.
TEXTURE_ALIASES = {
    "zombie": "Zombie",
    "z": "Zombie",
    "zombiedancerhead": "Zombie",
    "zombieswon": "ZombiesWon",
    "zombieswonscreen": "ZombiesWon",
    "splitpea": "SplitPea",
    "spikerock": "Spikerock",
    "pumpkin": "Pumpkin",
    "coin": "Coin",
    "coinglow": "Coin",
    "treefood": "TreeFood",
    "treeofwisdomclouds": "TreeOfWisdom",
    "zenwateringcan": "ZenGarden",
    "zengarden": "ZenGarden",
    "facialhair": "Zombatar",
    "hair": "Zombatar",
    "eyewear": "Zombatar",
    "hats": "Zombatar",
    "tidbits": "Zombatar",
    "accessories": "Zombatar",
}

# Efectos/props genéricos: no son una planta ni un zombie.
COMMON_TEXTURE_TOKENS = {
    "anim",
    "sprout",
    "startset",
    "startready",
    "startplant",
    "textfadeon",
    "ground",
    "fullscreen",
    "finalwave",
}

# Prefijos con variantes numéricas: tree1..tree9, fire1..fire8, splash1..6,
# leaf1..4, sun1..3, etc. El límite (dígito o fin) evita confundir SunFlower.
NUMERIC_PREFIX = re.compile(
    r"^(fire|splash|leafbunch|leaf|puff|tree|sun|hammer|rake|sodroll|portal|rain|loadbar)(\d|$)"
)

NUMERIC_OWNER = {
    "tree": "Tree",
    "fire": "Common",
    "splash": "Common",
    "leaf": "Common",
    "leafbunch": "Common",
    "puff": "Common",
    "sun": "Sun",
    "hammer": "Hammer",
    "rake": "Rake",
    "sodroll": "SodRoll",
    "portal": "Portal",
    "rain": "Rain",
    "loadbar": "LoadBar",
}


def owner_for_texture(filename: str) -> str:
    stem = Path(filename).stem
    token = re.split(r"[_-]", stem)[0]
    low = token.lower()

    if low in TEXTURE_ALIASES:
        return TEXTURE_ALIASES[low]

    m = NUMERIC_PREFIX.match(low)
    if m:
        return NUMERIC_OWNER[m.group(1)]

    if low.startswith("treefood"):
        return "TreeFood"

    if low in COMMON_TEXTURE_TOKENS:
        return "Common"

    # Capitaliza la primera letra del token para que las carpetas sean
    # homogéneas (blover -> Blover), sin tocar el resto del nombre.
    return token[:1].upper() + token[1:] if token else "Common"


REANIM_SPECIAL = {
    "peashootersingle": "PeaShooter",
    "lawnmoweredzombie": "Zombie",
    "z": "Zombie",
    "zombieswon": "ZombiesWon",
    "loadbar_zombiehead": "LoadBar",
    "digger_rising_dirt": "Digger",
    "treeofwisdomclouds": "TreeOfWisdom",
    "treeofwisdom": "TreeOfWisdom",
    "spikerock": "Spikerock",
    "fire": "Common",
    "splash": "Common",
    "puff": "Common",
    "startreadysetplant": "Common",
    "textfadeon": "Common",
    "finalwave": "Common",
}


def owner_for_reanim(filename: str) -> str:
    stem = Path(filename).stem
    low = stem.lower()

    if low == "zombieswon":
        return "ZombiesWon"
    if low.startswith("zombie"):
        return "Zombie"
    if low.startswith("credits"):
        return "Credits"
    if low.startswith("zengarden"):
        return "ZenGarden"
    if low.startswith("coin_"):
        return "Coin"
    if low.startswith("portal_"):
        return "Portal"
    if low.startswith("rain_"):
        return "Rain"
    if low.startswith("loadbar"):
        return "LoadBar"
    if low.startswith("zombatar"):
        return "Zombatar"
    if low == "selectorscreen":
        return "SelectorScreen"

    if low in REANIM_SPECIAL:
        return REANIM_SPECIAL[low]

    # `ZombiesWon`, `fire`, `splash`, `Puff`...
    token = re.split(r"[_-]", stem)[0]
    return token[:1].upper() + token[1:] if token else "Common"


@dataclass(frozen=True)
class Entry:
    name: str
    size: int
    pos: int


def read_pak(path: Path) -> tuple[bytes, list[Entry], int]:
    raw = path.read_bytes()
    data = bytes(b ^ XOR_KEY for b in raw)
    if data[:4] != MAGIC_DIR:
        raise SystemExit(f"{path}: cabecera inesperada {data[:4].hex()}")

    entries: list[Entry] = []
    off = 9
    while off < len(data):
        name_len = data[off]
        if name_len == 0 or name_len > 200:
            break
        name = data[off + 1 : off + 1 + name_len].decode("latin1")
        p = off + 1 + name_len
        size, _checksum, _unknown = struct.unpack_from("<III", data, p)
        entries.append(Entry(name=name, size=size, pos=p + 13))
        off = p + 13
    # `off` es ahora el inicio de los datos (fin del directorio).
    return data, entries, off


def iter_payloads(data: bytes, entries: list[Entry], data_start: int):
    pos = data_start
    for entry in entries:
        record = data[pos : pos + entry.size]
        if record[:4] == MAGIC_RECORD:
            # u32 magia + u32 tamaño_descomprimido
            yield entry.name, zlib.decompress(record[8:])
        else:
            yield entry.name, record
        pos += entry.size
    if pos != len(data):
        print(
            f"⚠️  Quedan {len(data) - pos} bytes sin consumir (esperado 0)",
            file=sys.stderr,
        )


def classify(name: str) -> str:
    """Devuelve la ruta relativa dentro de assets/."""
    if name.startswith("reanim\\"):
        base = name.split("\\")[-1]
        if base.lower().endswith(".reanim"):
            return f"animations/{owner_for_reanim(base)}/{base}"
        return f"textures/{owner_for_texture(base)}/{base}"
    parts = name.split("\\")
    top = parts[0]
    # images, particles, sounds, data, properties, compiled -> misma estructura
    return "/".join(parts)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--pak", type=Path, default=DEFAULT_PAK)
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    ap.add_argument("--force", action="store_true", help="sobrescribe archivos existentes")
    ap.add_argument(
        "--manifest",
        type=Path,
        default=None,
        help="ruta del manifiesto TSV (por defecto <out>/ASSET_MANIFEST.tsv)",
    )
    args = ap.parse_args()

    if not args.pak.is_file():
        raise SystemExit(f"No existe el .pak: {args.pak}")

    data, entries, data_start = read_pak(args.pak)
    manifest_path = args.manifest or (args.out / "ASSET_MANIFEST.tsv")

    stats: Counter[str] = Counter()
    owner_stats: Counter[str] = Counter()
    written = skipped = 0
    rows: list[tuple[str, str, int, str]] = []

    for name, payload in iter_payloads(data, entries, data_start):
        rel = classify(name)
        dest = args.out / rel
        sha = hashlib.sha256(payload).hexdigest()
        rows.append((name.replace("\\", "/"), rel, len(payload), sha))
        stats[rel.split("/")[0]] += 1
        if rel.startswith(("animations/", "textures/")):
            owner_stats[rel.split("/")[1]] += 1

        if dest.exists() and not args.force:
            skipped += 1
            continue
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(payload)
        written += 1

    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    with manifest_path.open("w", encoding="utf-8", newline="\n") as fh:
        fh.write("source\tdestination\tbytes\tsha256\n")
        for src, rel, size, sha in rows:
            fh.write(f"{src}\t{rel}\t{size}\t{sha}\n")

    print(f"📦 {args.pak} -> {args.out}")
    print(f"   entradas del .pak: {len(entries)}")
    print(f"   escritos: {written}   preservados: {skipped}")
    print(f"   manifiesto: {manifest_path}")
    print("   por categoría:")
    for k, v in sorted(stats.items()):
        print(f"     {v:5d}  {k}/")
    print("   por entidad (animations + textures):")
    for k, v in sorted(owner_stats.items(), key=lambda kv: (-kv[1], kv[0]))[:15]:
        print(f"     {v:5d}  {k}")
    if len(owner_stats) > 15:
        print(f"     ... y {len(owner_stats) - 15} entidades más")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
