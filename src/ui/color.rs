use {
    std::fs,
    eframe::egui::*,
    lazy_static::lazy_static,
    serde::Deserialize,
};

/*
 * NOTE:
 * ~ (egui v0.18) Noninteractive and interactive (but inactive) text colors are
 * 0x8C and 0xB4 respectively, except for default `LayoutJob` text, which is
 * 0xA0. The "weak" text color is computed at runtime
 * (`Visuals::weak_text_color`), and it's 0x53.
 */

const DIM_FACTOR: f32 = 0.55;

macro_rules! hex_color {
    ($hex:expr) => {
        Color32::from_rgb((($hex & 0xFF0000) >> 16) as u8, (($hex & 0xFF00) >> 8) as u8, ($hex & 0xFF) as u8)
    };

    ($hex:expr, % 'gray) => {
        Color32::from_gray($hex)
    };
}

pub struct Color {
    dim_stroke: Stroke,
    stroke: Stroke,
}

impl Color {
    #[inline(always)]
    pub fn dim(&self) -> Color32 {
        self.dim_stroke.color
    }

    #[inline(always)]
    pub fn dim_stroke(&self) -> Stroke {
        self.dim_stroke
    }

    fn from_gray(gray: u8) -> Self {
        let gray_u32 = gray as u32;

        Self::from((gray_u32 << 16) | (gray_u32 << 8) | (gray_u32))
    }

    #[inline(always)]
    pub fn normal(&self) -> Color32 {
        self.stroke.color
    }

    #[inline(always)]
    pub fn normal_stroke(&self) -> Stroke {
        self.stroke
    }
}

#[derive(Deserialize)]
#[serde(from = "TomlColors")]
pub struct Colors {
    pub common: CommonColors,
    pub editor: EditorColors,
    pub menu: MenuColors,
    pub tree: TreeColors,
}

pub struct CommonColors {
    pub gray: Color,
    pub light_gray: Color,
    pub near_weak_gray: Color,
    pub weak_gray: Color,
}

#[derive(Deserialize)]
#[serde(from = "TomlEditorColors")]
pub struct EditorColors {
    pub error: Color,
    pub modified: Color,
}

#[derive(Deserialize)]
#[serde(from = "TomlMenuColors")]
pub struct MenuColors {
    pub selected_tab: Color,
}

#[derive(Deserialize)]
#[serde(from = "TomlTreeColors")]
pub struct TreeColors {
    pub copied: Color,
    pub copy_pasted: Color,
    pub cut_pasted: Color,
    pub filtered: Color,
    pub modified: Color,
    pub pinned: Color,
    pub selected: Color,
    pub spotlighted: Color,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct TomlColors {
    editor: TomlEditorColors,
    menu: TomlMenuColors,
    tree: TomlTreeColors,
}

#[derive(Deserialize)]
#[serde(default)]
struct TomlEditorColors {
    error: u32,
    modified: u32,
}

#[derive(Deserialize)]
#[serde(default)]
struct TomlMenuColors {
    selected_tab: u32,
}

#[derive(Deserialize)]
#[serde(default)]
struct TomlTreeColors {
    copied: u32,
    copy_pasted: u32,
    cut_pasted: u32,
    filtered: u32,
    modified: u32,
    pinned: u32,
    selected: u32,
    spotlighted: u32,
}

impl From<u32> for Color {
    fn from(hex: u32) -> Self {
        let color = hex_color!(hex);

        Self {
            dim_stroke: Stroke::new(1.0, color.linear_multiply(DIM_FACTOR)),
            stroke: Stroke::new(1.0, color),
        }
    }
}

impl From<TomlColors> for Colors {
    fn from(toml: TomlColors) -> Self {
        Self {
            common: CommonColors::default(),
            editor: EditorColors::from(toml.editor),
            menu: MenuColors::from(toml.menu),
            tree: TreeColors::from(toml.tree),
        }
    }
}

impl From<TomlEditorColors> for EditorColors {
    fn from(toml: TomlEditorColors) -> Self {
        Self {
            error: Color::from(toml.error),
            modified: Color::from(toml.modified),
        }
    }
}

impl From<TomlMenuColors> for MenuColors {
    fn from(toml: TomlMenuColors) -> Self {
        Self {
            selected_tab: Color::from(toml.selected_tab),
        }
    }
}

impl From<TomlTreeColors> for TreeColors {
    fn from(toml: TomlTreeColors) -> Self {
        Self {
            copied: Color::from(toml.copied),
            copy_pasted: Color::from(toml.copy_pasted),
            cut_pasted: Color::from(toml.cut_pasted),
            filtered: Color::from(toml.filtered),
            modified: Color::from(toml.modified),
            pinned: Color::from(toml.pinned),
            selected: Color::from(toml.selected),
            spotlighted: Color::from(toml.spotlighted),
        }
    }
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            common: CommonColors::default(),
            editor: EditorColors::default(),
            menu: MenuColors::default(),
            tree: TreeColors::default(),
        }
    }
}

impl Default for CommonColors {
    fn default() -> Self {
        Self {
            gray: Color::from_gray(0x8C),
            light_gray: Color::from_gray(0xB4),
            near_weak_gray: Color::from_gray(0x5D),
            weak_gray: Color::from_gray(0x53),
        }
    }
}

impl Default for EditorColors {
    fn default() -> Self {
        Self::from(TomlEditorColors::default())
    }
}

impl Default for MenuColors {
    fn default() -> Self {
        Self::from(TomlMenuColors::default())
    }
}

impl Default for TreeColors {
    fn default() -> Self {
        Self::from(TomlTreeColors::default())
    }
}

impl Default for TomlEditorColors {
    fn default() -> Self {
        Self {
            error: 0xE87979,
            modified: 0xB3835E,
        }
    }
}

impl Default for TomlMenuColors {
    fn default() -> Self {
        Self {
            selected_tab: 0x1A4646,
        }
    }
}

impl Default for TomlTreeColors {
    fn default() -> Self {
        Self {
            copied: 0x79ADE8,
            copy_pasted: 0xBD79E8,
            cut_pasted: 0xE8C179,
            filtered: 0x8879E8,
            modified: 0xE8A979,
            pinned: 0x21345C,
            selected: 0x1F2842,
            spotlighted: 0xA7D4E7,
        }
    }
}

lazy_static! {
    pub static ref UI_COLORS: Colors = {
        match fs::read("colors.toml") {
            Ok(toml) => {
                match toml::from_str(&String::from_utf8_lossy(&toml)) {
                    Ok(colors) => return colors,
                    Err(error) => { dbg!(error); }
                }
            }
            Err(error) => { dbg!(error); }
        }

        Colors::default()
    };
}

