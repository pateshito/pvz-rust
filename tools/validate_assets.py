#!/usr/bin/env python3
"""Valida que las imágenes referenciadas por los .reanim existan en assets/.

Los `.reanim` referencian sprites por id interno, p. ej.:

    <i>IMAGE_REANIM_PEASHOOTER_HEAD</i>
    <i>IMAGE_REANIM_ZOMBIE_HEAD</i>

La convención del proyecto es que el archivo sea el id sin `IMAGE_REANIM_`,
sin distinguir mayúsculas/minúsculas y con extensión `.png`/`.jpg`:

    PeaShooter_Head.png
    Zombie_head.png

Resolución (la misma que `src/assets/paths.rs`):
    1. `textures/<entidad>/`
    2. `textures/Common/`
    3. cualquier otra carpeta de `textures/`
    4. `images/`, `particles/` y `data/` (sombras, fondos, bocadillos...)

Uso:
    python3 tools/validate_assets.py
    python3 tools/validate_assets.py --strict   # falla si hay referencias sin resolver

Salida:
    - Referencias totales y resueltas (en su entidad, en textures/ o en images/).
    - Lista de ids realmente sin resolver, con candidatos encontrados.
"""

from __future__ import annotations

import argparse
import collections
import os
import re
import sys
from pathlib import Path

IMAGE_RE = re.compile(r"<i>(.*?)</i>")
IMAGE_PREFIX = "IMAGE_REANIM_"


def index_textures(assets: Path) -> dict[str, list[tuple[str, str]]]:
    """stem en minúsculas -> [(entidad, archivo)]."""
    index: dict[str, list[tuple[str, str]]] = collections.defaultdict(list)
    textures = assets / "textures"
    if not textures.is_dir():
        raise SystemExit(f"No existe {textures}")
    for root, _dirs, files in os.walk(textures):
        owner = os.path.basename(root)
        for name in files:
            stem = os.path.splitext(name)[0].lower()
            index[stem].append((owner, name))
    return index


def index_other_images(assets: Path) -> dict[str, list[str]]:
    """stem en minúsculas -> rutas relativas fuera de textures/."""
    index: dict[str, list[str]] = collections.defaultdict(list)
    for top in ("images", "particles", "data"):
        base = assets / top
        if not base.is_dir():
            continue
        for root, _dirs, files in os.walk(base):
            for name in files:
                stem = os.path.splitext(name)[0].lower()
                index[stem].append(str(Path(root, name).relative_to(assets)))
    return index


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--assets", type=Path, default=Path("assets"))
    ap.add_argument("--strict", action="store_true")
    args = ap.parse_args()

    textures = index_textures(args.assets)
    other = index_other_images(args.assets)
    animations = args.assets / "animations"

    total = resolved = same_owner = resolved_other = 0
    unresolved: collections.Counter[str] = collections.Counter()
    owners_with_refs: collections.Counter[str] = collections.Counter()

    for root, _dirs, files in os.walk(animations):
        owner = os.path.basename(root)
        for name in sorted(files):
            if not name.endswith(".reanim"):
                continue
            text = (Path(root) / name).read_text(encoding="latin1", errors="replace")
            for match in IMAGE_RE.finditer(text):
                raw = match.group(1).strip()
                if not raw:
                    continue
                total += 1
                stem = raw[len(IMAGE_PREFIX) :] if raw.startswith(IMAGE_PREFIX) else raw
                candidates = textures.get(stem.lower())
                if candidates:
                    resolved += 1
                    if any(candidate_owner == owner for candidate_owner, _ in candidates):
                        same_owner += 1
                elif other.get(stem.lower()):
                    # Sombras, fondos, bocadillos... viven fuera de textures/.
                    resolved_other += 1
                else:
                    unresolved[raw] += 1
                    owners_with_refs[owner] += 1

    print(f"🖼️  referencias de imagen: {total}")
    print(f"   resueltas:            {resolved + resolved_other}")
    print(f"   resueltas en su entidad: {same_owner}")
    print(f"   resueltas en images/:  {resolved_other}")
    print(f"   sin resolver:         {sum(unresolved.values())} ({len(unresolved)} ids distintos)")
    if unresolved:
        print("\n   ids sin resolver:")
        for raw, count in unresolved.most_common():
            stem = raw[len(IMAGE_PREFIX) :] if raw.startswith(IMAGE_PREFIX) else raw
            hints = other.get(stem.lower(), [])
            hint = f"  -> {', '.join(hints[:3])}" if hints else ""
            print(f"     {count:3d}x {raw}{hint}")
    if args.strict and unresolved:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
