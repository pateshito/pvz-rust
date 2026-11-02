//! Escaneo de `assets/animations/` para descubrir entidades y sus `.reanim`.
//!
//! El catálogo es la fuente para cargar cualquier personaje sin tocar código:
//! una entidad es una carpeta bajo `animations/` y su carpeta de texturas es la
//! del mismo nombre bajo `textures/` (con alias para los pocos casos cuyo nombre
//! no coincide, como `TreeOfWisdom` → `Tree`).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Un `.reanim` concreto dentro de una entidad.
#[derive(Clone, Debug)]
pub struct ReanimEntry {
    /// Nombre del archivo sin `.reanim`.
    pub name: String,
    /// Ruta absoluta al `.reanim`.
    pub path: PathBuf,
}

/// Una entidad (planta, zombie, prop, pantalla de UI...).
#[derive(Clone, Debug)]
#[allow(dead_code)] // Campos para el visor de catálogo (Fase 3).
pub struct EntityInfo {
    pub name: String,
    /// `assets/animations/<Entidad>`.
    pub animations_dir: PathBuf,
    /// `assets/textures/<Entidad>` si existe.
    pub textures_dir: Option<PathBuf>,
    /// `.reanim` de la entidad, ordenados por nombre.
    pub reanims: Vec<ReanimEntry>,
}

impl EntityInfo {
    pub fn reanim(&self, name: &str) -> Option<&ReanimEntry> {
        self.reanims.iter().find(|r| r.name == name)
    }
}

/// Entidades descubiertas en `assets/animations/`, ordenadas por nombre.
#[derive(Clone, Debug, Default)]
pub struct EntityCatalog {
    entities: BTreeMap<String, EntityInfo>,
}

impl EntityCatalog {
    /// Escanea `animations/` y asocia cada entidad con `textures/`.
    pub fn discover(assets: &Path) -> Result<Self, String> {
        let animations = assets.join("animations");
        let textures = assets.join("textures");

        let mut entity_dirs: Vec<PathBuf> = fs::read_dir(&animations)
            .map_err(|e| format!("❌ No se pudo leer {:?}: {e}", animations))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        entity_dirs.sort();

        let mut entities = BTreeMap::new();
        for dir in entity_dirs {
            let name = dir
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| format!("❌ Nombre de entidad no UTF-8: {dir:?}"))?
                .to_string();

            let mut reanim_paths = Vec::new();
            collect_reanims(&dir, &mut reanim_paths)?;
            if reanim_paths.is_empty() {
                continue;
            }
            reanim_paths.sort();

            let reanims = reanim_paths
                .into_iter()
                .filter_map(|path| {
                    let stem = path.file_stem()?.to_str()?.to_string();
                    Some(ReanimEntry { name: stem, path })
                })
                .collect::<Vec<_>>();

            let textures_dir = find_textures_dir(&textures, &name);
            entities.insert(
                name.clone(),
                EntityInfo {
                    name,
                    animations_dir: dir,
                    textures_dir,
                    reanims,
                },
            );
        }

        if entities.is_empty() {
            return Err(format!(
                "❌ No se encontraron entidades en {:?}",
                animations
            ));
        }
        Ok(Self { entities })
    }

    #[allow(dead_code)] // API del visor de catálogo (Fase 3).
    pub fn entities(&self) -> impl Iterator<Item = &EntityInfo> {
        self.entities.values()
    }
    pub fn entity(&self, name: &str) -> Option<&EntityInfo> {
        self.entities.get(name)
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    #[allow(dead_code)] // API del visor de catálogo (Fase 3).
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub fn reanim_count(&self) -> usize {
        self.entities.values().map(|e| e.reanims.len()).sum()
    }

    /// Todos los `.reanim`, entidad por entidad y ordenados por nombre.
    #[allow(dead_code)] // API del visor de catálogo (Fase 3).
    pub fn reanims(&self) -> impl Iterator<Item = &ReanimEntry> {
        self.entities.values().flat_map(|e| e.reanims.iter())
    }
}

fn collect_reanims(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("❌ No se pudo leer {dir:?}: {e}"))?;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_reanims(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("reanim") {
            out.push(path);
        }
    }
    Ok(())
}

/// Busca `textures/<entidad>` sin distinguir mayúsculas; si no existe, prueba
/// los alias conocidos del árbol de assets.
fn find_textures_dir(textures: &Path, entity: &str) -> Option<PathBuf> {
    let exact = textures.join(entity);
    if exact.is_dir() {
        return Some(exact);
    }

    let entries = fs::read_dir(textures).ok()?;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir()
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.eq_ignore_ascii_case(entity))
                .unwrap_or(false)
        {
            return Some(path);
        }
    }

    // La animación del árbol del jardín usa la carpeta genérica `Tree`.
    if entity.eq_ignore_ascii_case("TreeOfWisdom") {
        let tree = textures.join("Tree");
        if tree.is_dir() {
            return Some(tree);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assets() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
    }

    #[test]
    fn descubre_todas_las_entidades_y_reanims() {
        let catalog = EntityCatalog::discover(&assets()).expect("catálogo");
        assert_eq!(catalog.len(), 74, "entidades esperadas");
        assert_eq!(catalog.reanim_count(), 146, "reanims esperados");
        assert!(catalog.entity("Zombie").is_some());
        assert!(catalog.entity("PeaShooter").is_some());
    }

    #[test]
    fn asocia_cada_entidad_con_sus_texturas() {
        let catalog = EntityCatalog::discover(&assets()).expect("catálogo");
        let zombie = catalog.entity("Zombie").unwrap();
        assert_eq!(zombie.reanims.len(), 40);
        assert!(zombie
            .textures_dir
            .as_ref()
            .unwrap()
            .ends_with("textures/Zombie"));
        assert!(zombie.reanim("Zombie").is_some());

        let tree = catalog.entity("TreeOfWisdom").unwrap();
        assert!(tree
            .textures_dir
            .as_ref()
            .unwrap()
            .ends_with("textures/Tree"));
    }

    #[test]
    fn los_nombres_de_reanim_son_unicos_por_entidad() {
        let catalog = EntityCatalog::discover(&assets()).expect("catálogo");
        for entity in catalog.entities() {
            let mut names: Vec<&str> = entity.reanims.iter().map(|r| r.name.as_str()).collect();
            names.sort_unstable();
            let before = names.len();
            names.dedup();
            assert_eq!(before, names.len(), "reanim duplicado en {}", entity.name);
        }
    }
}
