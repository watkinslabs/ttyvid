pub mod layers;

use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,

    #[serde(default)]
    pub background: u8,

    #[serde(default = "default_foreground")]
    pub foreground: u8,

    #[serde(default)]
    pub default_background: u8,

    #[serde(default = "default_foreground")]
    pub default_foreground: u8,

    #[serde(default)]
    pub transparent: u8,

    #[serde(default)]
    pub title: Option<TitleConfig>,

    #[serde(default)]
    pub padding: Option<Padding>,

    #[serde(default)]
    pub layers: Vec<Layer>,

    #[serde(default)]
    pub palette: Option<Palette>,
}

fn default_foreground() -> u8 {
    7
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleConfig {
    pub foreground: u8,
    pub background: u8,
    pub x: i32,
    pub y: i32,

    #[serde(default = "default_font_size")]
    pub font_size: f32,
}

fn default_font_size() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Padding {
    #[serde(default)]
    pub left: i32,

    #[serde(default)]
    pub top: i32,

    #[serde(default)]
    pub right: i32,

    #[serde(default)]
    pub bottom: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    /// Layer depth: negative for underlay, positive for overlay
    pub depth: i32,

    /// Path to the image file
    pub file: String,

    /// Rendering mode
    pub mode: LayerMode,

    #[serde(default)]
    pub nineslice: Option<NineSliceConfig>,

    #[serde(default)]
    pub bounds: Option<Bounds>,

    #[serde(default)]
    pub dst_bounds: Option<Bounds>,

    #[serde(default = "default_copy_mode")]
    pub copy_mode: CopyMode,

    /// Animation settings
    #[serde(default)]
    pub animation: Option<AnimationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationConfig {
    /// Animation speed multiplier (1.0 = normal speed, 2.0 = twice as fast)
    #[serde(default = "default_animation_speed")]
    pub speed: f64,

    /// Whether to loop the animation (default: true)
    #[serde(default = "default_animation_loop")]
    pub r#loop: bool,

    /// Frame to start on (0-indexed, default: 0)
    #[serde(default)]
    pub start_frame: usize,
}

fn default_animation_speed() -> f64 {
    1.0
}

fn default_animation_loop() -> bool {
    true
}

fn default_copy_mode() -> CopyMode {
    CopyMode::Copy
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayerMode {
    Center,
    Stretch,
    Tile,
    None,
    Scale,
    #[serde(rename = "9slice")]
    NineSlice,
    #[serde(rename = "3slice")]
    ThreeSlice,
    Copy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CopyMode {
    Copy,
    Tile,
}

#[derive(Debug, Clone)]
pub enum NineSliceValue {
    Auto,
    Value(i32),
}

impl<'de> Deserialize<'de> for NineSliceValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        struct NineSliceValueVisitor;

        impl<'de> serde::de::Visitor<'de> for NineSliceValueVisitor {
            type Value = NineSliceValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an integer or the string 'auto'")
            }

            fn visit_i64<E>(self, value: i64) -> Result<NineSliceValue, E>
            where
                E: Error,
            {
                Ok(NineSliceValue::Value(value as i32))
            }

            fn visit_u64<E>(self, value: u64) -> Result<NineSliceValue, E>
            where
                E: Error,
            {
                Ok(NineSliceValue::Value(value as i32))
            }

            fn visit_str<E>(self, value: &str) -> Result<NineSliceValue, E>
            where
                E: Error,
            {
                if value == "auto" {
                    Ok(NineSliceValue::Auto)
                } else {
                    Err(E::custom(format!("expected 'auto', got '{}'", value)))
                }
            }
        }

        deserializer.deserialize_any(NineSliceValueVisitor)
    }
}

impl Serialize for NineSliceValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            NineSliceValue::Auto => serializer.serialize_str("auto"),
            NineSliceValue::Value(v) => serializer.serialize_i32(*v),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NineSliceConfig {
    pub outer_left: NineSliceValue,
    pub outer_top: NineSliceValue,
    pub outer_right: NineSliceValue,
    pub outer_bottom: NineSliceValue,
    pub inner_left: NineSliceValue,
    pub inner_top: NineSliceValue,
    pub inner_right: NineSliceValue,
    pub inner_bottom: NineSliceValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bounds {
    #[serde(default)]
    pub left: BoundValue,

    #[serde(default)]
    pub top: BoundValue,

    #[serde(default)]
    pub right: BoundValue,

    #[serde(default)]
    pub bottom: BoundValue,
}

#[derive(Debug, Clone)]
pub enum BoundValue {
    Auto,
    Value(i32),
}

impl<'de> Deserialize<'de> for BoundValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        struct BoundValueVisitor;

        impl<'de> serde::de::Visitor<'de> for BoundValueVisitor {
            type Value = BoundValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an integer or the string 'auto'")
            }

            fn visit_i64<E>(self, value: i64) -> Result<BoundValue, E>
            where
                E: Error,
            {
                Ok(BoundValue::Value(value as i32))
            }

            fn visit_u64<E>(self, value: u64) -> Result<BoundValue, E>
            where
                E: Error,
            {
                Ok(BoundValue::Value(value as i32))
            }

            fn visit_str<E>(self, value: &str) -> Result<BoundValue, E>
            where
                E: Error,
            {
                if value == "auto" {
                    Ok(BoundValue::Auto)
                } else {
                    Err(E::custom(format!("expected 'auto', got '{}'", value)))
                }
            }
        }

        deserializer.deserialize_any(BoundValueVisitor)
    }
}

impl Serialize for BoundValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            BoundValue::Auto => serializer.serialize_str("auto"),
            BoundValue::Value(v) => serializer.serialize_i32(*v),
        }
    }
}

impl Default for BoundValue {
    fn default() -> Self {
        BoundValue::Auto
    }
}

impl BoundValue {
    pub fn resolve(&self, default: i32) -> i32 {
        match self {
            BoundValue::Auto => default,
            BoundValue::Value(v) => *v,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Palette {
    pub colors: usize,
    pub rgb: Vec<[u8; 3]>,
}

impl Theme {
    /// Load theme from filesystem path
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read theme file: {}", path.display()))?;

        let theme: Theme = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse theme YAML: {}", path.display()))?;

        Ok(theme)
    }

    /// Load theme by name, searching embedded themes first, then filesystem
    pub fn load_by_name(name: &str) -> Result<Self> {
        // Try embedded themes first (compiled into binary)
        if let Ok(theme) = Self::load_builtin(name) {
            return Ok(theme);
        }

        // Fall back to filesystem locations if not embedded
        for base_path in Self::theme_search_paths() {
            let theme_path = base_path.join(format!("{}.yaml", name));
            if theme_path.exists() {
                return Self::load(&theme_path);
            }
        }

        anyhow::bail!("Theme '{}' not found in embedded themes or filesystem", name)
    }

    /// Get theme search paths in order of priority
    fn theme_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Current directory (for development)
        paths.push(PathBuf::from("themes"));

        // User data directory
        if let Some(proj_dirs) = directories::ProjectDirs::from("", "", "ttyvid") {
            paths.push(proj_dirs.data_dir().join("themes"));
            paths.push(proj_dirs.config_dir().join("themes"));
        }

        // System-wide directories
        paths.push(PathBuf::from("/usr/share/ttyvid/themes"));
        paths.push(PathBuf::from("/usr/local/share/ttyvid/themes"));

        paths
    }

    /// Load embedded builtin theme
    pub fn load_builtin(name: &str) -> Result<Self> {
        let yaml = match name {
            "default" => include_str!("../../themes/default.yaml"),
            "windows7" => include_str!("../../themes/windows7.yaml"),
            "fdwm" => include_str!("../../themes/fdwm.yaml"),
            "fdwm-x" => include_str!("../../themes/fdwm-x.yaml"),
            "simple" => include_str!("../../themes/simple.yaml"),
            "bar" => include_str!("../../themes/bar.yaml"),
            "default-2bit" => include_str!("../../themes/default-2bit.yaml"),
            "default-4bit" => include_str!("../../themes/default-4bit.yaml"),
            "game" => include_str!("../../themes/game.yaml"),
            "mac" => include_str!("../../themes/mac.yaml"),
            "opensource" => include_str!("../../themes/opensource.yaml"),
            "scripted" => include_str!("../../themes/scripted.yaml"),
            _ => anyhow::bail!("Unknown builtin theme: {}. Available themes: default, windows7, fdwm, fdwm-x, simple, bar, game, mac, opensource, scripted", name),
        };

        let theme: Theme = serde_yaml::from_str(yaml)
            .with_context(|| format!("Failed to parse embedded theme: {}", name))?;

        Ok(theme)
    }

    pub fn find_layer_file(&self, layer_file: &str, theme_dir: &Path) -> PathBuf {
        // Try relative to theme file first
        let relative_path = theme_dir.join(layer_file);
        if relative_path.exists() {
            return relative_path;
        }

        // Try absolute path
        let absolute_path = PathBuf::from(layer_file);
        if absolute_path.exists() {
            return absolute_path;
        }

        // Fall back to relative path (will fail later if not found)
        relative_path
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            name: "default".to_string(),
            background: 0,
            foreground: 7,
            default_background: 0,
            default_foreground: 7,
            transparent: 0,
            title: None,
            padding: None,
            layers: Vec::new(),
            palette: None,
        }
    }
}
