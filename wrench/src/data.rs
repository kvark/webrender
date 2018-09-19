use std::collections::HashMap;
use std::ops::Range;

use euclid::SideOffsets2D;
use webrender::api;


fn bool_true() -> bool { true }
fn bool_false() -> bool { false }
fn usize_16() -> usize { 16 }
fn yuv_color_space_709() -> api::YuvColorSpace { api::YuvColorSpace::Rec709 }

pub type Document = HashMap<PipelineId, StackingContext>;

#[derive(Serialize, Deserialize)]
pub enum PipelineId {
    Root,
    Other(api::PipelineSourceId, u32),
}

#[derive(Serialize, Deserialize)]
pub struct StackingContext {
    #[serde(default)]
    pub bounds: Option<api::LayoutRect>,
    #[serde(default)]
    pub transform: Option<ComplexTransform>,
    #[serde(default)]
    pub perspective: Option<Perspective>,
    #[serde(default)]
    pub clip_node: Option<ClipId>,
    #[serde(default)]
    pub reference_frame_id: Option<u64>,
    #[serde(default)]
    pub glyph_raster_space: api::GlyphRasterSpace,
    #[serde(default)]
    pub scroll_offset: Option<api::LayoutPoint>,
    #[serde(default)]
    pub mix_blend_mode: api::MixBlendMode,
    #[serde(default)]
    pub filters: Vec<api::FilterOp>,
    pub items: Vec<Item>,
}

#[derive(Serialize, Deserialize)]
pub struct ComplexTransform {
    pub style: api::TransformStyle,
    pub modifiers: Vec<Transform>,
}

#[derive(Serialize, Deserialize)]
pub enum Transform {
    Matrix(api::LayoutTransform),
    Translate(api::LayoutVector2D),
    Rotate {
        axis: Axis,
        degrees: f32,
        origin: api::LayoutPoint,
    },
    Scale {
        axis: Option<Axis>,
        value: f32,
    },
    Skew {
        axis: Axis,
        value: f32,
    },
    Perspective {
        distance: f32,
    },
}

#[derive(Serialize, Deserialize)]
pub enum Axis {
    X,
    Y,
    Z,
}

#[derive(Serialize, Deserialize)]
pub enum Perspective {
    Matrix(api::LayoutTransform),
    Simple {
        distance: f32,
        origin: Option<api::LayoutPoint>,
    }
}

#[derive(Serialize, Deserialize)]
pub enum ClipId {
    Specific(u64),
    RootReferenceFrame,
    RootScrollNode,
}

#[derive(Serialize, Deserialize)]
pub struct Item {
    pub kind: ItemKind,
    #[serde(default)]
    pub clip_and_scroll: Option<ClipAndScroll>,
    #[serde(default)]
    pub complex_clip: Option<ComplexClip>,
    #[serde(default)]
    pub clip_rect: Option<api::LayoutRect>,
    #[serde(default = "bool_true")]
    pub backface_visible: bool,
    #[serde(default)]
    pub tag: Option<(i64, i64)>,
}

#[derive(Serialize, Deserialize)]
pub enum ClipAndScroll {
    Single(ClipId),
    Custom {
        clip: ClipId,
        scroll: ClipId,
    },
}

#[derive(Serialize, Deserialize)]
pub struct ComplexClip {
    pub rect: api::LayoutRect,
    #[serde(default)]
    pub radius: BorderRadius,
    #[serde(default)]
    pub clip_mode: api::ClipMode,
}

#[derive(Serialize, Deserialize)]
pub enum BorderRadius {
    Uniform(f32),
    Custom {
        #[serde(default)]
        top_left: f32,
        #[serde(default)]
        top_right: f32,
        #[serde(default)]
        bottom_left: f32,
        #[serde(default)]
        bottom_right: f32,
    },
}

impl Default for BorderRadius {
    fn default() -> Self {
        BorderRadius::Uniform(0.0)
    }
}

#[derive(Serialize, Deserialize)]
pub enum ItemKind {
    Rect {
        bounds: api::LayoutRect,
        #[serde(default = "Color::white")]
        color: Color,
    },
    ClearRect {
        bounds: api::LayoutRect,
    },
    Line {
        style: api::LineStyle,
        orientation: api::LineOrientation,
        #[serde(default = "Color::black")]
        color: Color,
        bounds: LineBounds,
    },
    Image {
        src: String,
        bounds: ImageRect,
        tile_size: Option<u16>,
        stretch_size: Option<api::LayoutSize>,
        #[serde(default = "api::LayoutSize::zero")]
        tile_spacing: api::LayoutSize,
        #[serde(default)]
        rendering: api::ImageRendering,
        #[serde(default)]
        alpha_type: api::AlphaType,
    },
    YuvImage {
        bounds: ImageRect,
        #[serde(default = "yuv_color_space_709")]
        color_space: api::YuvColorSpace,
        kind: YuvKind,
    },
    Text {
        #[serde(default = "usize_16")]
        size: usize,
        #[serde(default = "Color::black")]
        color: Color,
        #[serde(default)]
        bg_color: Option<Color>,
        #[serde(default = "bool_false")]
        synthetic_italics: bool,
        #[serde(default = "bool_false")]
        synthetic_bold: bool,
        #[serde(default = "bool_false")]
        embedded_bitmaps: bool,
        #[serde(default = "bool_false")]
        transpose: bool,
        #[serde(default = "bool_false")]
        flip_x: bool,
        #[serde(default = "bool_false")]
        flip_y: bool,
    },
    ScrollFrame,
    StickyFrame,
    Clip,
    ClipChain,
    Border {
        bounds: api::LayoutRect,
        widths: api::BorderWidths,
        kind: BorderKind,
    },
    Gradient {
        kind: GradientKind,
        tiling: Option<GradientTile>,
        stops: Vec<(f32, Color)>,
        #[serde(default)]
        extend: api::ExtendMode,
    },
    BoxShadow {
        bounds: api::LayoutRect,
        #[serde(default)]
        box_bounds: Option<api::LayoutRect>,
        #[serde(default = "api::LayoutVector2D::zero")]
        offset: api::LayoutVector2D,
        #[serde(default = "Color::black")]
        color: Color,
        #[serde(default)]
        blur_radius: f32,
        #[serde(default)]
        spread_radius: f32,
        #[serde(default)]
        border_radius: BorderRadius,
        #[serde(default)]
        clip_mode: api::BoxShadowClipMode,
    },
    Iframe,
    StackingContext(StackingContext),
    PopAllShadows,
}

#[derive(Serialize, Deserialize)]
pub enum Color {
    Custom {
        r: u8,
        g: u8,
        b: u8,
        a: f32,
    },
    Black,
    Blue,
    Green,
    Red,
    White,
    Yellow,
}

impl Color {
    fn black() -> Self {
        Color::Black
    }
    fn white() -> Self {
        Color::White
    }
}

#[derive(Serialize, Deserialize)]
pub enum LineBounds {
    Rect(api::LayoutRect),
    Baseline {
        level: f32,
        range: Range<f32>,
        width: f32,
    },
}

#[derive(Serialize, Deserialize)]
pub struct GradientTile {
    size: api::LayoutSize,
    #[serde(default = "api::LayoutSize::zero")]
    spacing: api::LayoutSize,
}

#[derive(Serialize, Deserialize)]
pub enum GradientKind {
    Linear {
        bounds: api::LayoutRect,
        range: Range<api::LayoutPoint>,
    },
    Radial {
        center: api::LayoutPoint,
        radius: api::LayoutSize,
    },
}

#[derive(Serialize, Deserialize)]
pub enum BorderKind {
    Normal {
        radius: BorderRadius,
        top: BorderSide,
        bottom: BorderSide,
        left: BorderSide,
        right: BorderSide,
    },
    Image {
        src: String,
        size: (i64, i64),
        #[serde(default = "bool_false")]
        fill: bool,
        slice: SideOffsets2D<u32>,
        outset: SideOffsets2D<f32>,
        repeat_horizontal: api::RepeatMode,
        repeat_vertical: api::RepeatMode,
    },
    Gradient {
        kind: GradientKind,
        outset: SideOffsets2D<f32>,
    },
}

pub type BorderSide = (api::BorderStyle, Color);

#[derive(Serialize, Deserialize)]
pub struct ImageRect {
    pub origin: api::LayoutPoint,
    #[serde(default)]
    pub size: Option<api::LayoutSize>,
}

#[derive(Serialize, Deserialize)]
pub enum YuvKind {
    Planar {
        src_y: String,
        src_u: String,
        src_v: String,
    },
    Nv12 {
        src_y: String,
        src_uv: String,
    },
    Interleaved {
        src: String,
    },
}
