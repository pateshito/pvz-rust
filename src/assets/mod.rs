//! Catálogo de assets y resolución genérica de imágenes.
//!
//! La convención del proyecto es:
//!
//! ```text
//! assets/animations/<Entidad>/<archivo>.reanim
//! assets/textures/<Entidad>/<archivo>.png|jpg
//! assets/images/**                 (UI, fondos, sombras...)
//! ```
//!
//! Un `.reanim` referencia sus sprites con ids como `IMAGE_REANIM_ZOMBIE_HEAD`.
//! [`paths`] traduce ese id a una ruta real sin tablas hardcodeadas por personaje.

pub mod catalog;
pub mod paths;
