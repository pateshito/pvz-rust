use regex::Regex;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Resolved {
    pub x: f32,
    pub y: f32,
    pub kx: f32,
    pub sx: f32,
    pub sy: f32,
    pub image: Option<String>,
    pub active: bool,
}

#[derive(Clone, Debug)]
pub struct Track {
    pub name: String,
    pub frames: Vec<Option<Resolved>>,
    pub raw_f: Vec<Option<i32>>,
    pub has_image: bool,
}

#[derive(Clone, Debug)]
pub struct AnimRange {
    pub name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone)]
pub struct ReanimData {
    pub fps: f32,
    pub tracks: Vec<Track>,
    pub animations: Vec<AnimRange>,
}

pub fn parse_reanim(path: &Path) -> Result<ReanimData, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("❌ No se pudo leer {:?}: {}", path, e))?;

    let fps = Regex::new(r"<fps>(\d+)</fps>")
        .unwrap()
        .captures(&content)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<f32>().ok())
        .unwrap_or(12.0);

    let track_re = Regex::new(r"(?s)<track>(.*?)</track>").unwrap();
    let name_re = Regex::new(r"(?s)<name>(.*?)</name>").unwrap();
    let frame_re = Regex::new(r"(?s)<t>(.*?)</t>").unwrap();
    let val_x = Regex::new(r"<x>(-?[\d.]+)</x>").unwrap();
    let val_y = Regex::new(r"<y>(-?[\d.]+)</y>").unwrap();
    let val_kx = Regex::new(r"<kx>(-?[\d.]+)</kx>").unwrap();
    let val_sx = Regex::new(r"<sx>(-?[\d.]+)</sx>").unwrap();
    let val_sy = Regex::new(r"<sy>(-?[\d.]+)</sy>").unwrap();
    let val_f = Regex::new(r"<f>(-?\d+)</f>").unwrap();
    let val_i = Regex::new(r"<i>(.*?)</i>").unwrap();

    let mut tracks: Vec<Track> = Vec::new();

    for track_cap in track_re.captures_iter(&content) {
        let tc = track_cap.get(1).unwrap().as_str();
        let name = name_re
            .captures(tc)
            .map(|c| c.get(1).unwrap().as_str().trim().to_string())
            .unwrap_or_else(|| "unknown".into());

        let mut frames: Vec<Option<Resolved>> = Vec::new();
        let mut raw_f: Vec<Option<i32>> = Vec::new();
        let mut has_image = false;

        let mut cur = Resolved {
            x: 0.0,
            y: 0.0,
            kx: 0.0,
            sx: 1.0,
            sy: 1.0,
            image: None,
            active: true,
        };

        for frame_cap in frame_re.captures_iter(tc) {
            let fc = frame_cap.get(1).unwrap().as_str();
            let get_f = |re: &Regex| -> Option<f32> {
                re.captures(fc)
                    .and_then(|c| c.get(1))
                    .and_then(|m| m.as_str().parse().ok())
            };

            if let Some(v) = get_f(&val_x) {
                cur.x = v;
            }
            if let Some(v) = get_f(&val_y) {
                cur.y = v;
            }
            if let Some(v) = get_f(&val_kx) {
                cur.kx = v;
            }
            if let Some(v) = get_f(&val_sx) {
                cur.sx = v;
            }
            if let Some(v) = get_f(&val_sy) {
                cur.sy = v;
            }

            if let Some(v) = val_i
                .captures(fc)
                .map(|c| c.get(1).unwrap().as_str().trim().to_string())
            {
                cur.image = Some(v);
                has_image = true;
            }

            let f = val_f
                .captures(fc)
                .and_then(|c| c.get(1))
                .and_then(|m| m.as_str().parse::<i32>().ok());
            if let Some(v) = f {
                cur.active = v >= 0;
            }

            raw_f.push(f);
            frames.push(Some(cur.clone()));
        }

        tracks.push(Track {
            name,
            frames,
            raw_f,
            has_image,
        });
    }

    let marker_range = |t: &Track| -> Option<(usize, usize)> {
        let mut active = true;
        let mut start: Option<usize> = None;
        let mut end = 0;
        for (i, f) in t.raw_f.iter().enumerate() {
            if let Some(v) = f {
                active = *v >= 0;
            }
            if active {
                if start.is_none() {
                    start = Some(i);
                }
                end = i;
            }
        }
        start.map(|s| (s, end))
    };

    let mut animations: Vec<AnimRange> = Vec::new();
    for t in &tracks {
        if t.has_image || !t.name.starts_with("anim_") {
            continue;
        }
        if let Some((s, e)) = marker_range(t) {
            animations.push(AnimRange {
                name: t.name.clone(),
                start: s,
                end: e,
            });
        }
    }
    animations.sort_by_key(|a| a.start);

    println!(
        "✅ {:?}: {} tracks | fps={} | {} animaciones",
        path.file_name(),
        tracks.len(),
        fps,
        animations.len()
    );
    for (i, a) in animations.iter().enumerate() {
        println!("   🎬 [{}] {} -> frames {}..{}", i, a.name, a.start, a.end);
    }

    Ok(ReanimData {
        fps,
        tracks,
        animations,
    })
}
