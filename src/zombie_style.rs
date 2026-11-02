//! Estilos del zombi normal (`Zombie.reanim`).
//!
//! Ese archivo trae, en los mismos frames, el cuerpo y todos los accesorios
//! (cono, cubo, puerta, bandera, bigote, lengua, flotador). El juego original
//! enseña un grupo de tracks por tipo y esconde el resto. Aquí se hace lo mismo
//! para que el visor no dibuje una mezcla de todas las texturas.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlainStyle {
    Normal,
    Cone,
    Bucket,
    Door,
    Flag,
    Ducky,
}

impl PlainStyle {
    pub const ALL: [PlainStyle; 6] = [
        PlainStyle::Normal,
        PlainStyle::Cone,
        PlainStyle::Bucket,
        PlainStyle::Door,
        PlainStyle::Flag,
        PlainStyle::Ducky,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PlainStyle::Normal => "Normal",
            PlainStyle::Cone => "Cono",
            PlainStyle::Bucket => "Cubo",
            PlainStyle::Door => "Puerta",
            PlainStyle::Flag => "Bandera",
            PlainStyle::Ducky => "Flotador",
        }
    }

    /// `false` = el track no pertenece a este estilo y hay que esconderlo,
    /// aunque el frame del `.reanim` lo marque activo.
    pub fn allows(self, track: &str) -> bool {
        // Accesorios que el archivo deja activos y el juego apaga por defecto.
        let mut hidden = matches!(
            track,
            "anim_cone" | "anim_bucket" | "anim_screendoor" | "Zombie_flaghand" | "anim_tongue"
        ) || track.starts_with("Zombie_duckytube")
            || track.starts_with("Zombie_mustache")
            || track.starts_with("Zombie_outerarm_screendoor")
            || track.starts_with("Zombie_innerarm_screendoor");

        match self {
            PlainStyle::Normal => {}
            PlainStyle::Cone => {
                if track == "anim_cone" {
                    hidden = false;
                }
            }
            PlainStyle::Bucket => {
                if track == "anim_bucket" {
                    hidden = false;
                }
            }
            PlainStyle::Door => {
                if track == "anim_screendoor"
                    || track.starts_with("Zombie_outerarm_screendoor")
                    || track.starts_with("Zombie_innerarm_screendoor")
                {
                    hidden = false;
                }
            }
            PlainStyle::Flag => {
                if track == "Zombie_flaghand" || track.starts_with("Zombie_innerarm_screendoor") {
                    hidden = false;
                }
            }
            PlainStyle::Ducky => {
                if track.starts_with("Zombie_duckytube") {
                    hidden = false;
                }
            }
        }

        // Cono y cubo tapan el pelo. Puerta y bandera cambian los brazos.
        if matches!(self, PlainStyle::Cone | PlainStyle::Bucket) && track == "anim_hair" {
            return false;
        }
        if matches!(self, PlainStyle::Door | PlainStyle::Flag) && track.starts_with("anim_innerarm")
        {
            return false;
        }
        if self == PlainStyle::Door
            && matches!(
                track,
                "Zombie_outerarm_hand" | "Zombie_outerarm_lower" | "Zombie_outerarm_upper"
            )
        {
            return false;
        }

        !hidden
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXTRAS: [&str; 10] = [
        "anim_cone",
        "anim_bucket",
        "anim_screendoor",
        "Zombie_flaghand",
        "anim_tongue",
        "Zombie_mustache",
        "Zombie_duckytube",
        "Zombie_outerarm_screendoor",
        "Zombie_innerarm_screendoor",
        "Zombie_innerarm_screendoor_hand",
    ];

    const BODY: [&str; 9] = [
        "Zombie_body",
        "anim_head1",
        "anim_hair",
        "Zombie_tie",
        "Zombie_outerarm_upper",
        "anim_innerarm1",
        "anim_innerarm2",
        "anim_innerarm3",
        "Zombie_neck",
    ];

    #[test]
    fn el_normal_es_solo_el_cuerpo() {
        for track in EXTRAS {
            assert!(!PlainStyle::Normal.allows(track), "{track}");
        }
        for track in BODY {
            assert!(PlainStyle::Normal.allows(track), "{track}");
        }
        // El agua la controla el propio frame (`f`), no el estilo.
        assert!(PlainStyle::Normal.allows("Zombie_whitewater"));
        assert!(PlainStyle::Normal.allows("Zombie_whitewater2"));
    }

    #[test]
    fn cada_estilo_ensenha_solo_lo_suyo() {
        assert!(PlainStyle::Cone.allows("anim_cone"));
        assert!(!PlainStyle::Cone.allows("anim_hair"));
        assert!(!PlainStyle::Cone.allows("anim_bucket"));

        assert!(PlainStyle::Bucket.allows("anim_bucket"));
        assert!(!PlainStyle::Bucket.allows("anim_hair"));
        assert!(!PlainStyle::Bucket.allows("anim_cone"));

        assert!(PlainStyle::Door.allows("anim_screendoor"));
        assert!(PlainStyle::Door.allows("Zombie_outerarm_screendoor"));
        assert!(PlainStyle::Door.allows("Zombie_innerarm_screendoor_hand"));
        assert!(!PlainStyle::Door.allows("Zombie_outerarm_upper"));
        assert!(!PlainStyle::Door.allows("anim_innerarm1"));
        assert!(!PlainStyle::Door.allows("anim_cone"));

        assert!(PlainStyle::Flag.allows("Zombie_flaghand"));
        assert!(PlainStyle::Flag.allows("Zombie_innerarm_screendoor"));
        assert!(PlainStyle::Flag.allows("Zombie_outerarm_upper"));
        assert!(!PlainStyle::Flag.allows("anim_innerarm3"));
        assert!(!PlainStyle::Flag.allows("anim_screendoor"));

        assert!(PlainStyle::Ducky.allows("Zombie_duckytube"));
        assert!(PlainStyle::Ducky.allows("anim_hair"));
        assert!(!PlainStyle::Ducky.allows("anim_cone"));
    }
}
