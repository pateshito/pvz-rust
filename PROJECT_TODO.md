# PVZ-qwen · Mapa del proyecto y lista de quehaceres

> **Fecha:** 2026-11-02
> **Fuente de assets:** `/home/lukas/Proyectos/JUEGOS PARA REA/PVZ/main.pak`
> **Destino:** `/home/lukas/Proyectos/Rust/PVZ-qwen/assets/`
> **Herramientas:** `tools/extract_assets.py`, `tools/validate_assets.py`
> **Estado del árbol de assets:** completo y verificado (3198/3198 archivos del `.pak`).

---

## 0. Resumen ejecutivo

| Área | Antes | Ahora |
|---|---|---|
| Assets extraídos | solo 2 personajes (Zombie, PeaShooter) | **3198 archivos** del `.pak` + `reanimContext.txt` + manifiesto |
| Organización | `textures/<Personaje>/`, `animations/<Personaje>/` | mismo criterio, extendido a **74 entidades** y 6 categorías más |
| Tamaño | ~1,4 MB | **~72 MB** |
| Verificación | manual | script reproducible + hash SHA-256 por archivo |
| Código | visor de 2 personajes | catálogo, visor de cualquiera y nivel de prueba jugable (36 tests OK) |
| Documentación | ninguna | este archivo |

**Lo que ya está hecho**

- [x] Reverse engineering del contenedor `main.pak` (XOR `0xF7` + zlib + directorio propio).
- [x] Extractor reproducible: `tools/extract_assets.py`.
- [x] Extracción completa a `assets/` conservando los archivos que ya existían.
- [x] Verificación: 3198/3198 archivos coinciden byte a byte con el `.pak`; único archivo distinto: `animations/Zombie/Zombie.reanim` (tu versión modificada, preservada).
- [x] Manifiesto `assets/ASSET_MANIFEST.tsv` con origen → destino, tamaño y SHA-256.
- [x] Validador de referencias `.reanim` → texturas: `tools/validate_assets.py` (3013/3013).
- [x] Catálogo de assets (`src/assets/catalog.rs`) y resolución genérica de imágenes (`src/assets/paths.rs`).
- [x] `cargo test`: 36/36 tests pasan con el árbol nuevo.

**Lo que falta para “completar el proyecto”**

El proyecto hoy es un **visor de animaciones**, no un juego. Falta convertir el visor en un juego de Plants vs. Zombies: catálogo data-driven, tablero, plantas, zombies, proyectiles, soles, oleadas, UI, audio, progresión y empaquetado. El detalle está en la sección 4.

---

## 1. Cómo está ahora

### 1.1 Árbol del repositorio

```text
PVZ-qwen/
├── Cargo.toml                 Bevy 0.15, regex, rand, serde, bevy_asset_loader
├── Cargo.lock
├── src/
│   ├── main.rs                Arranque, estados Menu/Play/TestMode, catálogo
│   ├── reanim.rs              Parser .reanim (regex) + modelo de tracks/estados
│   ├── resources.rs           CharacterLibraries, SelectedCharacter, ViewerState
│   ├── components.rs          Marcadores ECS (CharacterRoot, CharacterPart, HUD…)
│   ├── menu.rs                Menú Jugar/Test; Jugar aún no dibuja el tablero
│   ├── viewer.rs              Lista de entidades, cualquier .reanim, zoom, estilos
│   ├── systems.rs             Movimiento horizontal de prueba
│   ├── peashooter.rs          Reglas especiales de estados del PeaShooter
│   ├── utils.rs               Rutas de assets, nombre de entidad, hud
│   ├── character.rs           Piezas/poses compartidas por visor y partida
│   ├── game/                  board, defs, level, sim y view (la partida en pantalla)
│   └── assets/
│       ├── catalog.rs         Escaneo de animations/ → EntityCatalog
│       └── paths.rs           Índice de texturas + resolución IMAGE_REANIM_*
├── tools/
│   ├── extract_assets.py      Extractor/organizador del .pak
│   └── validate_assets.py     Validador reanim → texturas
├── assets/                    (ver 1.2)
├── target/
└── PROJECT_TODO.md            este archivo
```

### 1.2 Árbol de assets actual (ya reorganizado)

```text
assets/
├── ASSET_MANIFEST.tsv         3198 filas: origen, destino, bytes, sha256
├── animations/                146 .reanim agrupados por entidad
│   ├── Zombie/                40 reanims (base + variantes)
│   ├── PeaShooter/            3 (PeaShooter, PeaShooterSingle, reanimContext.txt)
│   ├── Common/                6 efectos (fire, splash, Puff, FinalWave…)
│   ├── Credits/               19
│   ├── ZenGarden/             5
│   ├── <Planta>/              1 por planta (SunFlower, CherryBomb, Wallnut…)
│   └── <UI/props>/            SelectorScreen, SlotMachine, Zombatar, LoadBar…
├── textures/                  1817 PNG/JPG agrupados por entidad
│   ├── Zombie/                757
│   ├── Zombatar/              168
│   ├── Common/                52 (hojas, fuego, splash, sombras…)
│   ├── PeaShooter/            21
│   ├── SunFlower/             29
│   ├── <Planta>/              texturas de cada planta
│   └── <UI/props>/            CrazyDave, SelectorScreen, Credits, Tree…
├── images/                    572 (UI, fondos, almanac, tienda, zombatar)
│   ├── reloaded/              45 (arte de la versión “reloaded”)
│   └── _UNUSED/               2
├── sounds/                    171 (167 .ogg, 2 .au, 2 .mo3)
├── particles/                 194 (112 .xml + 81 .png + 1 .trail)
├── compiled/                  251 (versiones compiladas del juego)
│   ├── particles/             107 .xml.compiled
│   └── reanim/                144 .reanim.compiled
├── data/                      42 (fuentes: .txt + .png/.gif)
└── properties/                5 (xml/txt del juego)
```

**Conteo por categoría**

| Categoría | Archivos | Origen en el `.pak` |
|---|---:|---|
| `textures/` | 1817 | `reanim/*.png|jpg` |
| `images/` | 572 | `images/**` |
| `compiled/` | 251 | `compiled/**` |
| `particles/` | 194 | `particles/**` |
| `sounds/` | 171 | `sounds/**` |
| `animations/` | 146 | `reanim/*.reanim` |
| `data/` | 42 | `data/**` |
| `properties/` | 5 | `properties/**` |
| **Total `.pak`** | **3198** | |

**Entidades principales**

- 40 reanims de zombie en `animations/Zombie/` (base, cono, cubo, bailarín, futbolista, gargantuar, jefe, etc.).
- 757 texturas de zombie en `textures/Zombie/` (base + todas las variantes).
- Plantas con animación propia: `SunFlower`, `CherryBomb`, `Wallnut`, `PotatoMine`, `SnowPea`, `Chomper`, `PuffShroom`, `SunShroom`, `FumeShroom`, `Gravebuster`, `HypnoShroom`, `ScaredyShroom`, `IceShroom`, `DoomShroom`, `LilyPad`, `Squash`, `ThreePeater`, `Tanglekelp`, `Jalapeno`, `Spikerock`, `Torchwood`, `Tallnut`, `SeaShroom`, `Plantern`, `Cactus`, `Blover`, `SplitPea`, `Starfruit`, `Pumpkin`, `Magnetshroom`, `Cabbagepult`, `Pot`, `Cornpult`, `Coffeebean`, `Garlic`, `Umbrellaleaf`, `Marigold`, `Melonpult`, `GatlingPea`, `TwinSunflower`, `GloomShroom`, `Cattail`, `WinterMelon`, `GoldMagnet`, `CobCannon`, `Imitater`, `FirePea`, `PeaShooter`.
- UI/props/otros: `SelectorScreen`, `Credits`, `ZenGarden`, `Zombatar`, `CrazyDave`, `SlotMachine`, `ZombiesWon`, `LoadBar`, `TreeOfWisdom`, `Tree`, `Coin`, `Sun`, `Diamond`, `Rain`, `Portal`, `Rake`, `Hammer`, `SodRoll`, `Digger`, `Stinky`, `LawnMower`, `PoolCleaner`, `RoofCleaner`.

### 1.3 Mapa del código actual

| Módulo | Responsabilidad | Estado |
|---|---|---|
| `main.rs` | Arranque, catálogo, estados Menu/Play/TestMode | Funciona. Sigue precargando Zombie y PeaShooter para la partida |
| `reanim.rs` | Parser XML con regex; `Track`, `Resolved`, `AnimRange`, `AnimState`; detección de rangos `anim_*` | Funcional, sin validación estricta |
| `peashooter.rs` | Une bloques del `.reanim` en 2 estados (`idle`, `shooting`) y separa cuerpo/cabeza | Funcional, específico |
| `utils.rs` | `assets_dir()`, `asset_exists()`, `entity_name()`, `image_to_path()` | Resolución genérica; sin tablas por personaje |
| `assets/catalog.rs` | Escanea `animations/` y asocia cada entidad con `textures/` | 74 entidades / 146 reanims |
| `assets/paths.rs` | Índice de texturas e imágenes; resuelve `IMAGE_REANIM_*` | 3013/3013 referencias resueltas |
| `viewer.rs` | Lista de las 74 entidades, cualquier `.reanim`, zoom, HUD y estilos del zombi | Funcional |
| `character.rs` | Crea piezas y aplica pose por frame; `Animator` compartido | Listo para conectar la partida |
| `game/board.rs` | Rejilla 5×9, celdas y coordenadas | Lógica pura + tests |
| `game/defs.rs` | Definiciones de plantas/zombies | PeaShooter y zombie básico |
| `game/level.rs` | Nivel de prueba, oleadas y soles | Lógica pura + tests |
| `game/sim.rs` | Simulación: plantar, disparar, comer, ganar/perder | Lógica pura + tests |
| `game/view.rs` | Dibuja la partida de prueba sobre el césped de día | Lanzaguisantes, zombi básico, guisantes y soles |
| `menu.rs` | Menú Jugar / Test | Funcional |
| `systems.rs` | Mueve el personaje en X | Placeholder |
| `resources.rs` | Recursos globales del visor | Funcional |

**Lo que el código ya sabe hacer**

- Leer `.reanim`, extraer tracks, frames, `x/y/kx/sx/sy`, `f` (activo) e imagen `i`.
- Detectar rangos de animación por tracks `anim_*`.
- Descubrir las **74 entidades** y sus **146 reanims** sin hardcodear.
- Elegir cualquiera de esas entidades en el modo Test y reproducir sus `.reanim` (los que no tienen marcadores `anim_*` se recorren enteros).
- Resolver **cualquier** `IMAGE_REANIM_*` (3013/3013) con índice en memoria.
- Reproducir una animación a `fps`, con cuerpo/cabeza en rangos distintos.
- Ver el zombi normal por estilos (cuerpo, cono, cubo, puerta, bandera, flotador) sin mezclar accesorios.
- Cambiar imagen por frame (parpadeos).
- Ocultar tracks inactivos.
- Menú → visor o partida → volver, pausa, reinicio, movimiento.
- Simular una partida completa sin Bevy: plantar, enfriamientos, soles, guisantes, daño, muerte, oleadas y victoria/derrota.
- Jugar esa partida en el césped de día: clic para plantar, recoger soles, ver al zombi caminar, comer y morir.

**Lo que el código no sabe hacer todavía**

- Reproducir animaciones con `image` distinta por frame de forma completa (solo cambia si el track tiene imagen).
- Pala, carretilla y el resto de plantas y zombies. La partida de prueba (lanzaguisantes contra el zombi básico) ya se dibuja.
- UI de juego (seed packets, almanac, tienda), audio, progresión, guardado.
- Empaquetado/distribución.

### 1.4 Formato de `main.pak` (reverse engineering)

El `.pak` de esta copia **no es un ZIP**. Está ofuscado y comprimido con un formato propio:

```text
archivo completo: XOR byte a byte con 0xF7

cabecera:  c0 4a c0 ba 00 00 00 00 00   (9 bytes)

directorio (repetido):
    u8    nombre_len
    u8[]  nombre          (ruta con '\')
    u32   tamaño_registro (incluye la cabecera del registro)
    u32   checksum        (no es CRC32 estándar; no se usa)
    u32   desconocido
    u8    flag             (0 normal; 128 visto en el último archivo)

datos (mismo orden que el directorio):
    si empieza por d4 fe ad de:
        u32  magia
        u32  tamaño_descomprimido
        zlib(tamaño_registro - 8)
    si no:
        bytes crudos de tamaño_registro
```

Detalles verificados:

- 3198 entradas.
- 251 registros comprimidos con zlib; 2947 almacenados en crudo (PNG, OGG, TXT…).
- El directorio termina en el offset `143506`; los datos ocupan hasta el final (`45453180` bytes).
- El checksum del directorio no coincide con CRC32; se ignora.

> **Nota REA:** no había servidor REA/MCP conectado en esta sesión. `main.pak` es un contenedor propietario no cubierto por las herramientas de archivo estándar, así que se aplicó el análisis directo del formato (el fallback correcto para este caso). El resultado quedó verificado por hash y es reproducible con `tools/extract_assets.py`.

### 1.5 Cobertura actual frente al juego

| Sistema del juego | Assets disponibles | Código actual |
|---|---|---|
| Animaciones de plantas | ✅ 40+ plantas | ⚠️ se ven en el modo Test; la partida no las usa |
| Animaciones de zombies | ✅ 40 variantes | ⚠️ se ven en el modo Test; la partida no las usa |
| Texturas de reanim | ✅ 1817 | ✅ resolución genérica (el visor carga la entidad elegida) |
| UI / menús / almanac | ✅ 572 imágenes | ❌ menú mínimo propio + lista del modo Test |
| Sonidos/música | ✅ 171 | ❌ sin audio |
| Partículas | ✅ 194 | ❌ sin partículas |
| Datos/fuentes | ✅ 42 | ❌ sin uso |
| Tablero y gameplay | — | ⚠️ nivel de prueba en pantalla (un tipo de planta y un zombi) |
| Progresión/guardado | — | ❌ no existe |

### 1.6 Deuda técnica y estado de Git

- **Dump antiguo:** hay **3175 eliminaciones sin commitear** de `assets/assets/**` (la extracción cruda anterior). El árbol nuevo las reemplaza. Hay que decidir si se confirma la eliminación.
- **Assets nuevos:** están sin trackear (3198 archivos + manifiesto). Antes de commitear, revisar `.gitignore` y el peso del repo.
- **Dependencias declaradas y no usadas:** `serde`, `serde_json`, `bevy_asset_loader`; `rand` solo aparece en un comentario.
- **`get_allowlist` eliminada:** el visor muestra todos los tracks con imagen; los accesorios de variantes quedan ocultos por sus `<f>`.
- **Resolución genérica:** `src/assets/paths.rs` indexa 2631 imágenes y resuelve las 3013 referencias (85 en `images/`).
- **Parser por regex:** funciona con estos `.reanim`, pero no valida estructura ni da errores con contexto.
- **Sin tests de assets en CI:** los tests de catálogo/resolución ya existen; falta integrar `validate_assets.py --strict` en CI o `xtask`.
- **Sin README de assets:** la convención está en este documento y en los scripts.

---

## 2. Cómo debería estar

### 2.1 Árbol de assets objetivo (convención definitiva)

La convención que ya usas y que se ha extendido:

```text
assets/
├── animations/<Entidad>/<archivo>.reanim     # una carpeta por entidad
├── textures/<Entidad>/<archivo>.png|jpg      # texturas de sus reanim
├── images/                                   # UI, fondos, almanac, tienda…
├── sounds/                                   # .ogg, .au, .mo3
├── particles/                                # .xml + .png de partículas
├── data/                                     # fuentes
├── properties/                               # xml/txt del juego
├── compiled/                                 # opcional: datos compilados
└── ASSET_MANIFEST.tsv                        # trazabilidad origen→destino
```

Reglas:

1. **Una entidad = una carpeta** en `animations/` y `textures/`.
2. El nombre del archivo **no se cambia** respecto al `.pak`.
3. Los assets compartidos van a `Common/` (hojas, fuego, splash, sombras, arranque).
4. Los assets de UI/props van a su carpeta propia (`SelectorScreen`, `Zombatar`, `Credits`…).
5. Los archivos existentes **no se sobrescriben** salvo `--force`; los ajustes manuales mandan.
6. Cualquier asset nuevo se añade al manifiesto y se valida con `tools/validate_assets.py`.

### 2.2 Arquitectura de código objetivo

```text
src/
├── main.rs                 # arranque y plugins
├── app/
│   ├── states.rs           # Menu, LevelSelect, Playing, Paused, GameOver
│   └── schedule.rs         # orden de sistemas
├── assets/
│   ├── catalog.rs          # escaneo de animations/ y textures/ → EntityCatalog
│   ├── paths.rs            # resolución genérica IMAGE_REANIM_* → archivo
│   └── manifest.rs         # lectura/validación del ASSET_MANIFEST.tsv
├── reanim/
│   ├── model.rs            # Track, Frame, AnimRange, AnimState
│   ├── parser.rs           # parser robusto + errores con contexto
│   ├── player.rs           # muestreo de frames, blending, eventos
│   └── resolve.rs          # imagen + transform por frame
├── entities/
│   ├── plant.rs            # definición data-driven de plantas
│   ├── zombie.rs           # definición data-driven de zombies
│   ├── projectile.rs
│   └── sun.rs
├── board/
│   ├── grid.rs             # 5 carriles × 9 columnas
│   ├── placement.rs        # plantar/recoger/shovel
│   └── waves.rs            # oleadas y difficulty
├── combat/
│   ├── damage.rs
│   ├── collision.rs
│   └── effects.rs
├── ui/
│   ├── menu.rs
│   ├── seed_bank.rs
│   ├── almanac.rs
│   └── hud.rs
├── audio/
│   └── playback.rs
└── data/
    └── game_data.rs        # plantas, zombies, niveles, costes, daños
```

Objetivo de diseño:

- **Data-driven:** añadir una planta/zombie = añadir su carpeta de assets + una entrada de datos, sin tocar el renderer.
- **Un solo player de reanim** para todas las entidades.
- **Resolución genérica de imágenes:** `IMAGE_REANIM_X` → buscar `X` (case-insensitive) en `textures/<Entidad>/` y, si no, en `textures/Common/`, otras carpetas de `textures/` y `images/`.
- **Separación clara:** assets / reanim / entidades / tablero / combate / UI / audio.
- **Tests:** parser, catálogo, resolución de imágenes, simulación de una oleada.

> **Estado:** la capa `assets/` ya existe (`catalog.rs` + `paths.rs`) y está cubierta por tests. El resto del árbol es el objetivo de las fases 2–9.

### 2.3 Mapa de brechas (ahora → objetivo)

| # | Brecha | Impacto | Prioridad |
|---|---|---|---|
| 1 | Catálogo de entidades inexistente | No se pueden cargar los 146 reanims sin hardcodear | P0 |
| 2 | `image_to_path` hardcodeado | Solo funcionan 2 personajes | P0 |
| 3 | Visor no elige personaje | No se puede explorar el catálogo | P1 · hecho en el modo Test |
| 4 | Sin tablero | No hay juego | P1 |
| 5 | Sin plantas/zombies data-driven | No hay gameplay | P1 |
| 6 | Sin proyectiles/soles/colisiones | No hay combate | P1 |
| 7 | Sin oleadas | No hay progresión de nivel | P1 |
| 8 | Sin UI de juego | No se puede jugar de verdad | P2 |
| 9 | Sin audio | Experiencia incompleta | P2 |
| 10 | Sin empaquetado | No se distribuye | P2 |
| 11 | Sin CI de assets | Regresiones silenciosas | P2 |
| 12 | Dump antiguo sin decidir | Ruido en Git | P2 |

---

## 3. Lista de quehaceres (ordenada)

> Leyenda: `P0` = bloquea lo demás · `P1` = necesario para jugar · `P2` = pulido/distribución.

### Fase 0 — Assets (✅ completada)

- [x] **P0** Reverse engineering de `main.pak`.
- [x] **P0** Extractor `tools/extract_assets.py`.
- [x] **P0** Extraer los 3198 archivos.
- [x] **P0** Agrupar texturas y reanims por entidad.
- [x] **P0** Preservar `Zombie.reanim` modificado y `reanimContext.txt`.
- [x] **P0** Manifiesto con SHA-256.
- [x] **P2** Validador `tools/validate_assets.py` (3013/3013 referencias resueltas; 0 sin resolver).
- [ ] **P2** Decidir si se confirman las 3175 eliminaciones de `assets/assets/`.
- [ ] **P2** Añadir `assets/README.md` corto con la convención (opcional; ya está en este doc).
- [ ] **P2** Integrar `validate_assets.py --strict` en CI o en un `xtask`.
- [x] **P2** Resolver los 12 ids de `images/` con la búsqueda genérica en `images/` (ya no hace falta alias).

### Fase 1 — Catálogo y resolución genérica (✅ completada)

- [x] **P0** Crear `src/assets/catalog.rs`: escanear `assets/animations/*/` y construir `EntityCatalog`.
- [x] **P0** Asociar cada `.reanim` con su carpeta de texturas (`animations/<X>` ↔ `textures/<X>`).
- [x] **P0** Crear `src/assets/paths.rs` con resolución genérica:
  - `IMAGE_REANIM_X` → `textures/<Entidad>/X.png` (case-insensitive).
  - fallback a `textures/Common/X.png`, otras carpetas de `textures/` y `images/`.
- [x] **P0** Sustituir `image_to_path()` hardcodeado por la resolución genérica.
- [x] **P0** Eliminar `get_allowlist()`: mostrar todos los tracks con imagen.
- [x] **P0** Test: cargar los 146 reanims y comprobar que todos los tracks con imagen resuelven.
- [x] **P1** Manejar colisiones de nombre entre entidades (preferencia entidad → Common → otras texturas).
- [x] **P1** Cachear el índice de texturas para no recorrer el disco por frame.

**Criterio de aceptación:** `cargo test` carga el catálogo completo y el validador reporta 0 referencias sin resolver (o solo las justificadas con alias).

### Fase 2 — Parser de reanim robusto (P0/P1)

- [ ] **P0** Separar `reanim.rs` en `model.rs`, `parser.rs`, `player.rs`, `resolve.rs`.
- [ ] **P1** Validar estructura y devolver errores con archivo/línea/track.
- [ ] **P1** Soportar `<fps>`, `<track>`, `<name>`, `<t>`, `x/y/kx/sx/sy/f/i` completos.
- [ ] **P1** Soportar imágenes por frame y cambios de imagen (parpadeos) en todos los tracks.
- [ ] **P1** Soportar `AnimState` combinando rangos cuerpo/cabeza de forma genérica (no solo PeaShooter).
- [ ] **P1** Soportar eventos/loops y animaciones que no son `anim_*`. El visor ya reproduce esos archivos como un estado `full`; faltan eventos y loops con nombre.
- [ ] **P2** Tests unitarios con `.reanim` reales de varias plantas y zombies.

**Criterio de aceptación:** el visor reproduce cualquier `.reanim` del catálogo sin código específico.

### Fase 3 — Visor/catálogo explorable (P1, hecho; queda pulido P2)

- [x] **P1** Pantalla de selección de entidad (lista de plantas/zombies/UI).
- [x] **P1** Mostrar nombre, fps, número de tracks y animaciones.
- [x] **P1** Cambiar animación, pausa, reinicio, zoom y movimiento.
- [x] **P1** Mostrar el mapeo de imágenes y avisar de assets faltantes (consola al crear las piezas, y los que faltan en el HUD).
- [x] **P1** Estilos del `Zombie.reanim` base: normal, cono, cubo, puerta, bandera y flotador. Sin esto el archivo dibuja todos los accesorios a la vez.
- [ ] **P2** Captura/export de frames para depurar.
- [ ] **P2** Modo “comparar” dos reanims.

**Criterio de aceptación:** se puede navegar por las 74 entidades y reproducir sus animaciones.

### Fase 4 — Tablero y reglas base (P1)

- [x] **P0** Definir el tablero: 5 carriles × 9 columnas, coordenadas y escala. Se dibuja en `Jugar`.
- [ ] **P0** Fondo, césped, casa, carretilla y límites. Hechos el fondo de día (`background1.jpg`, incluye casa y césped) y el límite de la casa. Falta la carretilla: la simulación aún no la tiene.
- [x] **P1** Colocación de plantas con rejilla y snapping.
- [ ] **P1** Shovel/recogida.
- [x] **P1** Estado de nivel: soles, progreso, oleada, victoria/derrota.
- [x] **P1** Pausa y reinicio.
- [ ] **P2** Soportar varios tipos de terreno (día, noche, piscina, tejado).

**Criterio de aceptación:** se puede plantar y ver una oleada básica en un tablero.

### Fase 5 — Plantas data-driven (P1)

- [ ] **P0** Definir `PlantDef`: id, nombre, coste, vida, recarga, animación, proyectil, comportamiento.
- [ ] **P0** Implementar las plantas base: PeaShooter, SunFlower, Wallnut, PotatoMine, CherryBomb, SnowPea, Chomper. El lanzaguisantes ya se planta y dispara en la partida de prueba.
- [ ] **P1** Implementar el resto por familias:
  - Tirador: PeaShooter, SnowPea, Repeater/Gatling, SplitPea, Threepeater.
  - Productor: SunFlower, SunShroom, TwinSunflower.
  - Muro: Wallnut, Tallnut, Pumpkin, Garlic.
  - Explosivo: CherryBomb, Jalapeno, DoomShroom, PotatoMine.
  - Área: FumeShroom, GloomShroom, Melonpult, WinterMelon, Cabbagepult, Cornpult, CobCannon.
  - Control: IceShroom, HypnoShroom, Squash, Tanglekelp, MagnetShroom.
  - Especiales: LilyPad, Flower Pot, CoffeeBean, UmbrellaLeaf, Blover, Plantern, Gravebuster, Marigold, GoldMagnet, Cattail, Spikerock, Torchwood, Imitater, Cactus, SeaShroom, ScaredyShroom, PuffShroom.
- [ ] **P1** Animaciones por estado: idle, shoot, eat, die, etc.
- [ ] **P1** Coste/recarga y seed packets.
- [ ] **P2** Almanac con datos y arte.

**Criterio de aceptación:** cada planta tiene definición, animación y comportamiento básico.

### Fase 6 — Zombies y oleadas (P1)

- [x] **P0** Definir `ZombieDef`: id, vida, velocidad, daño, animación, comportamiento. Hoy solo el zombi básico.
- [x] **P0** Implementar zombie base: caminar, comer, morir. Se ve en la partida de prueba.
- [ ] **P1** Variantes: cono, cubo, bandera, carrerilla, bailarín, futbolista, pogo, pole vault, digger, ladder, paper, balloon, snorkel, dolphin, bobsled, catapult, gargantuar, imp, jefe, yeti, zamboni, disco, jackbox, dancer, charred.
- [ ] **P1** Oleadas: composición, tiempos, flag wave, final wave.
- [ ] **P1** Spawn por carril y progresión de dificultad.
- [ ] **P1** Muerte, partes desmembradas y efectos.
- [ ] **P2** Niveles especiales (supervivencia, minijuegos, puzzle).

**Criterio de aceptación:** una oleada completa se juega de principio a fin.

### Fase 7 — Combate, proyectiles y soles (P1)

- [ ] **P0** Proyectiles: pea, snow pea, spike, star, melon, cabbage, kernel, cob. El guisante ya vuela y golpea.
- [x] **P0** Colisiones proyectil↔zombie y planta↔zombie.
- [x] **P0** Daño, vida, muerte y feedback visual. La muerte usa `anim_death`; todavía no hay sangre ni partículas.
- [x] **P1** Soles: caída, recogida, contador y gasto.
- [ ] **P1** Efectos: explosiones, hielo, fuego, veneno, ralentización.
- [ ] **P1** Partículas (`assets/particles/`) y sonidos de impacto.
- [ ] **P2** Balance de daños/costes.

**Criterio de aceptación:** el combate se siente como PvZ y los recursos se gastan/recogen.

### Fase 8 — UI, audio y progresión (P2)

- [ ] **P1** Seed bank, selección de plantas, contador de soles.
- [ ] **P1** HUD de nivel, oleada y pausa.
- [ ] **P2** Menú principal con arte de `assets/images/`.
- [ ] **P2** Almanac, tienda, logros y zombatar (si se quiere fidelidad).
- [ ] **P2** Audio: música (`mainmusic.mo3`), efectos `.ogg`, voces de Crazy Dave.
- [ ] **P2** Guardado de progreso (serde/serde_json ya están en Cargo).
- [ ] **P2** Opciones: volumen, pantalla, idioma.

**Criterio de aceptación:** experiencia completa de menú → nivel → victoria/derrota.

### Fase 9 — Pruebas, rendimiento y empaquetado (P2)

- [ ] **P1** Tests de parser, catálogo, resolución de imágenes y simulación.
- [ ] **P1** Test de humo: cargar los 146 reanims y las 1817 texturas.
- [ ] **P2** Perfilar render de muchos sprites (batching, atlas).
- [ ] **P2** Reducir memoria de texturas (atlas opcional).
- [ ] **P2** `cargo fmt`, `cargo clippy`, CI.
- [ ] **P2** Empaquetado (release, assets junto al binario, `BEVY_ASSET_ROOT`).
- [ ] **P2** Documentar controles y créditos.

**Criterio de aceptación:** build reproducible, tests verdes y binario distribuible.

---

## 4. Prioridad recomendada (siguiente iteración)

Orden concreto para no bloquearse:

1. **Catálogo + resolución genérica** (Fase 1). ✅ Hecho: desbloquea todo lo demás.
2. **Visor de cualquier entidad** (Fase 3). ✅ Hecho el P1. Quedan captura de frames y el modo comparar (P2).
3. **Parser robusto + tests** (Fase 2). Evita deuda al crecer. Los `.reanim` sin `anim_*` ya se reproducen enteros.
4. **Tablero mínimo + PeaShooter vs zombie base** (Fases 4–7). ✅ Se juega el nivel de prueba. Faltan la pala, la carretilla y el resto de plantas y zombies.
5. **Resto de plantas/zombies data-driven** (Fases 5–6).
6. **UI/audio/progresión** (Fase 8).
7. **Empaquetado y CI** (Fase 9).

---

## 5. Convenciones y comandos

### Rutas

- `assets/animations/<Entidad>/<archivo>.reanim`
- `assets/textures/<Entidad>/<archivo>.png`
- `assets/images/**`, `assets/sounds/**`, `assets/particles/**`, `assets/data/**`, `assets/properties/**`, `assets/compiled/**`

### Comandos

```bash
# Extraer/organizar desde el .pak (no sobrescribe lo existente)
python3 tools/extract_assets.py

# Forzar re-extracción (¡sobrescribe cambios manuales!)
python3 tools/extract_assets.py --force

# Validar referencias .reanim -> texturas
python3 tools/validate_assets.py

# Fallar si hay referencias sin resolver (para CI)
python3 tools/validate_assets.py --strict

# Compilar y probar
cargo test
cargo run
```

### Variables de entorno

- `BEVY_ASSET_ROOT`: raíz alternativa para assets (ya soportada por `utils::assets_dir`).
- `CARGO_MANIFEST_DIR`: usada por defecto con `cargo run`.

---

## 6. Registro de verificación (2026-11-02)

| Comprobación | Resultado |
|---|---|
| Entradas del `.pak` | 3198 |
| Archivos extraídos/organizados | 3198 |
| Hash origen vs manifiesto | 3198/3198 OK |
| Hash destino vs manifiesto | 3197/3198 OK (1 modificado a propósito) |
| Archivos preservados sin sobrescribir | 57 |
| Archivo modificado preservado | `animations/Zombie/Zombie.reanim` |
| Categorías | 8 (`animations`, `textures`, `images`, `sounds`, `particles`, `compiled`, `data`, `properties`) |
| Entidades (carpetas) | 74 en `animations/`, 74 en `textures/` |
| Referencias `.reanim` → imagen | 3013 |
| Resueltas (textures/) | 2928 |
| Resueltas (images/) | 85 |
| Sin resolver | 0 |
| Entidades / reanims en el catálogo | 74 / 146 |
| `cargo test` | 36/36 OK |
| `validate_assets.py --strict` | OK (0 sin resolver) |
| Tamaño de `assets/` | ~72 MB |

---

## 7. Decisiones abiertas / riesgos

1. **¿Confirmar el borrado de `assets/assets/`?** El árbol nuevo lo reemplaza, pero hay 3175 eliminaciones sin commitear.
2. **¿Incluir `compiled/`?** Son datos compilados del juego original; útiles para comparar, no necesarios para el clon. Se incluyeron para tener “todos los assets”.
3. **¿Licencia/distribución?** Los assets son de PopCap/EA. Este proyecto es de uso personal/educativo; no distribuir el arte original sin permiso.
4. **¿Fidelidad 1:1 o reinterpretación?** El alcance de UI/minijuegos/zombatar define cuánto arte usar.
5. **¿Atlas o archivos sueltos?** Hoy se cargan PNG sueltos; para rendimiento conviene un atlas opcional.
6. **¿Un solo crate o workspace?** Si el juego crece, separar `reanim`, `game` y `tools`.

---

## 8. Referencias internas

- `tools/extract_assets.py` — extractor y reglas de clasificación.
- `tools/validate_assets.py` — validador de referencias.
- `assets/ASSET_MANIFEST.tsv` — trazabilidad completa.
- `src/reanim.rs` — parser y modelo actual.
- `src/peashooter.rs` — ejemplo de estados combinados.
- `src/utils.rs` — rutas y mapeo actual.
