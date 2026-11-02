//! Resolución genérica de ids de imagen de los `.reanim` a rutas de asset.
//!
//! `IMAGE_REANIM_PEASHOOTER_HEAD` se busca como `PeaShooter_Head` sin distinguir
//! mayúsculas, en este orden:
//!
//! 1. `textures/<Entidad>/`
//! 2. `textures/Common/`
//! 3. cualquier otra carpeta de `textures/` (hay nombres cruzados, p. ej.
//!    `TreeOfWisdom` usa `textures/Tree/`)
//! 4. `images/`, `particles/` y `data/` (sombras, fondos, bocadillos...)
//!
//! El índice se construye una sola vez y se consulta en memoria; no se toca el
//! disco por frame.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

const PREFIXES: [&str; 2] = ["IMAGE_REANIM_", "IMAGE_"];
const SEARCH_ROOTS: [&str; 4] = ["textures", "images", "particles", "data"];

#[derive(Clone, Debug)]
struct Candidate {
    /// Carpeta que contiene el archivo (`Zombie`, `Common`, `reloaded`...).
    owner: String,
    /// Raíz bajo `assets/` (`textures`, `images`...).
    top: &'static str,
    /// Ruta relativa a `assets/`, con `/`.
    rel: String,
    /// `0` = png, `1` = jpg, `2` = jpeg, `3` = otra.
    ext_rank: u8,
}

/// Índice `stem en minúsculas → archivos candidatos`.
#[derive(Clone, Debug, Default)]
pub struct TextureIndex {
    by_stem: BTreeMap<String, Vec<Candidate>>,
    files: usize,
}

impl TextureIndex {
    /// Recorre `textures/`, `images/`, `particles/` y `data/`.
    pub fn scan(assets: &Path) -> Result<Self, String> {
        let mut index = Self::default();
        for top in SEARCH_ROOTS {
            let root = assets.join(top);
            if root.is_dir() {
                index.walk(assets, &root, top)?;
            }
        }
        Ok(index)
    }

    fn walk(&mut self, assets: &Path, dir: &Path, top: &'static str) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| format!("❌ No se pudo leer {dir:?}: {e}"))?;
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                self.walk(assets, &path, top)?;
                continue;
            }
            let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            let ext_rank = match ext.to_ascii_lowercase().as_str() {
                "png" => 0,
                "jpg" => 1,
                "jpeg" => 2,
                _ => continue,
            };
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let Some(rel) = relative_path(assets, &path) else {
                continue;
            };
            let owner = path
                .parent()
                .and_then(Path::file_name)
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            self.by_stem
                .entry(stem.to_ascii_lowercase())
                .or_default()
                .push(Candidate {
                    owner,
                    top,
                    rel,
                    ext_rank,
                });
            self.files += 1;
        }
        Ok(())
    }

    pub fn files(&self) -> usize {
        self.files
    }

    /// Resuelve un id interno (`IMAGE_REANIM_*` o ya sin prefijo).
    pub fn resolve(&self, internal: &str, entity: &str) -> Option<&str> {
        let stem = normalize_id(internal)?;
        let candidates = self.by_stem.get(&stem.to_ascii_lowercase())?;
        candidates
            .iter()
            .min_by_key(|c| (score(c, entity), c.ext_rank, c.rel.as_str()))
            .map(|c| c.rel.as_str())
    }

    /// Candidatos para depurar un id (ordenados por preferencia).
    #[allow(dead_code)] // Se usará en el visor de catálogo para explicar fallos.
    pub fn candidates(&self, internal: &str, entity: &str) -> Vec<&str> {
        let Some(stem) = normalize_id(internal) else {
            return Vec::new();
        };
        let Some(candidates) = self.by_stem.get(&stem.to_ascii_lowercase()) else {
            return Vec::new();
        };
        let mut ordered: Vec<&Candidate> = candidates.iter().collect();
        ordered.sort_by_key(|c| (score(c, entity), c.ext_rank, c.rel.as_str()));
        ordered.into_iter().map(|c| c.rel.as_str()).collect()
    }
}

/// `IMAGE_REANIM_ZOMBIE_HEAD` → `ZOMBIE_HEAD`; `IMAGE_X` → `X`.
fn normalize_id(internal: &str) -> Option<&str> {
    let id = internal.trim();
    if id.is_empty() {
        return None;
    }
    let stem = PREFIXES
        .iter()
        .find_map(|prefix| id.strip_prefix(prefix))
        .unwrap_or(id)
        .trim();
    (!stem.is_empty()).then_some(stem)
}

/// Menor es mejor: entidad propia → Common → otra textura → imágenes/UI.
fn score(candidate: &Candidate, entity: &str) -> u8 {
    if candidate.top != "textures" {
        return 3;
    }
    if candidate.owner.eq_ignore_ascii_case(entity) {
        0
    } else if candidate.owner.eq_ignore_ascii_case("Common") {
        1
    } else {
        2
    }
}

fn relative_path(base: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(base).ok()?;
    Some(
        rel.components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
    )
}

fn global_index() -> Option<&'static TextureIndex> {
    static INDEX: OnceLock<Option<TextureIndex>> = OnceLock::new();
    INDEX
        .get_or_init(|| match TextureIndex::scan(crate::utils::assets_dir()) {
            Ok(index) => {
                println!("🗂️  Índice de imágenes: {} archivos", index.files());
                Some(index)
            }
            Err(e) => {
                eprintln!("⚠️  No se pudo indexar assets: {e}");
                None
            }
        })
        .as_ref()
}

/// Resuelve un id de imagen contra el índice global de `assets/`.
pub fn resolve_image(internal: &str, entity: &str) -> Option<String> {
    global_index()?
        .resolve(internal, entity)
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn assets() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
    }

    fn index() -> TextureIndex {
        TextureIndex::scan(&assets()).expect("índice de texturas")
    }

    #[test]
    fn indexa_texturas_e_imagenes() {
        let index = index();
        assert!(
            index.files() > 2400,
            "archivos indexados: {}",
            index.files()
        );
    }

    #[test]
    fn resuelve_casos_conocidos() {
        let index = index();
        assert_eq!(
            index.resolve("IMAGE_REANIM_PEASHOOTER_HEAD", "PeaShooter"),
            Some("textures/PeaShooter/PeaShooter_Head.png")
        );
        assert_eq!(
            index.resolve("IMAGE_REANIM_ZOMBIE_HEAD", "Zombie"),
            Some("textures/Zombie/Zombie_head.png")
        );
        // `2RDFARTHEST` en el reanim, `2rdfarthest` en el archivo.
        assert_eq!(
            index.resolve("IMAGE_REANIM_PEASHOOTER_HEADLEAF_2RDFARTHEST", "PeaShooter"),
            Some("textures/PeaShooter/PeaShooter_headleaf_2rdfarthest.png")
        );
    }

    #[test]
    fn usa_common_y_luego_images() {
        let index = index();
        assert_eq!(
            index.resolve("IMAGE_REANIM_ANIM_SPROUT", "PeaShooter"),
            Some("textures/Common/anim_sprout.png")
        );
        assert_eq!(
            index.resolve("IMAGE_REANIM_PLANTSHADOW", "SunFlower"),
            Some("images/plantshadow.png")
        );
        assert_eq!(
            index.resolve("IMAGE_REANIM_BACKGROUND1", "Credits"),
            Some("images/background1.jpg")
        );
    }

    #[test]
    fn prefiere_la_textura_de_la_entidad() {
        let index = index();
        let candidates = index.candidates("IMAGE_REANIM_ZOMBIE_BODY", "Zombie");
        assert_eq!(
            candidates.first().copied(),
            Some("textures/Zombie/Zombie_body.png")
        );
    }

    #[test]
    fn id_desconocido_no_resuelve() {
        let index = index();
        assert_eq!(index.resolve("IMAGE_REANIM_NO_EXISTE", "Zombie"), None);
        assert_eq!(index.resolve("", "Zombie"), None);
    }

    /// Carga los 146 `.reanim` del árbol y comprueba que cada id de imagen
    /// distinto resuelve a un archivo real (sin alias pendientes).
    #[test]
    fn todos_los_reanims_resuelven_sus_imagenes() {
        use crate::assets::catalog::EntityCatalog;
        use std::collections::BTreeSet;

        let assets = assets();
        let catalog = EntityCatalog::discover(&assets).expect("catálogo");
        let index = index();

        let mut ids: BTreeSet<(String, String)> = BTreeSet::new();
        for entity in catalog.entities() {
            for entry in &entity.reanims {
                let data = crate::reanim::parse_reanim(&entry.path)
                    .unwrap_or_else(|e| panic!("{}: {e}", entry.path.display()));
                for track in &data.tracks {
                    for frame in track.frames.iter().flatten() {
                        if let Some(image) = &frame.image {
                            ids.insert((entity.name.clone(), image.clone()));
                        }
                    }
                }
            }
        }

        let missing: Vec<_> = ids
            .iter()
            .filter(|(entity, image)| index.resolve(image, entity).is_none())
            .map(|(entity, image)| format!("{entity}: {image}"))
            .collect();

        assert!(ids.len() > 1500, "ids de imagen distintos: {}", ids.len());
        assert!(missing.is_empty(), "ids sin resolver: {missing:#?}");
    }
}
