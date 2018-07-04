use std::collections::HashMap;

use webrender::api;


fn default_backface_visible() -> bool { true }
fn default_radius_component() -> f32 { 0.0 }

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
    pub perspective: Perspective,
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
    None,
    Matrix(api::LayoutTransform),
    Simple {
        distance: f32,
        origin: Option<api::LayoutPoint>,
    }
}

impl Default for Perspective {
    fn default() -> Self {
        Perspective::None
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
    pub clip_and_scroll: ClipAndScroll,
    #[serde(default)]
    pub complex_clip: Option<ComplexClip>,
    #[serde(default)]
    pub clip_rect: Option<api::LayoutRect>,
    #[serde(default = "default_backface_visible")]
    pub backface_visible: bool,
    #[serde(default)]
    pub tag: Option<(i64, i64)>,
}

#[derive(Serialize, Deserialize)]
pub enum ClipAndScroll {
    None,
    Same(ClipId),
    Both {
        clip: ClipId,
        scroll: ClipId,
    },
}

impl Default for ClipAndScroll {
    fn default() -> Self {
        ClipAndScroll::None
    }
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
    Zero,
    Uniform(f32),
    Custom {
        #[serde(default = "default_radius_component")]
        top_left: f32,
        #[serde(default = "default_radius_component")]
        top_right: f32,
        #[serde(default = "default_radius_component")]
        bottom_left: f32,
        #[serde(default = "default_radius_component")]
        bottom_right: f32,
    },
}

impl Default for BorderRadius {
    fn default() -> Self {
        BorderRadius::Zero
    }
}

#[derive(Serialize, Deserialize)]
pub enum ItemKind {
    Rect,
    ClearRect {
        bounds: api::LayoutRect,
    },
    Line,
    Image,
    YuvImage,
    Text,
    ScrollFrame,
    StickyFrame,
    Clip,
    ClipChain,
    Border,
    Gradient,
    RadialGradient,
    BoxShadow,
    Iframe,
    StackingContext(StackingContext),
    PopAllShadows,
}
