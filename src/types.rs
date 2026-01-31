#[cfg(feature = "serde")]
use base64::Engine;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};
#[derive(PartialEq, Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum GraphMode {
    ///given a 3d data set maps in 3d, given a 2d data set maps in 2d
    #[default]
    Normal,
    ///takes a slice of the 3d data set and displays it in 2d,
    ///what slice is depended on Graph.view_x and Graph.slice
    Slice,
    ///graphs the 3d data set as a domain coloring plot, explained more in Graph.domain_alternate
    DomainColoring,
    ///maps the real part to the x axis and imaginary part to the y axis
    ///in 3d takes a slice and applys the above logic
    Flatten,
    ///maps the real part to the x axis and imaginary part to the y axis
    ///and the input variable to the z axis
    ///in 3d takes a slice and applys the above logic
    Depth,
    ///turns a 2d function into a polar graph by mapping the x axis to angle of rotation and the y axis to radius,
    ///given a 3d function it maps the z to radius, x to the polar angle, y to the azimuthal angle
    Polar,
    ///takes a slice of a 3d function and applys polar logic
    SlicePolar,
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub enum GraphData {
    ///2d data set where the first element in the vector maps to the first float on the x axis,
    ///and the last element in the vector maps to the last float on the x axis, with even spacing
    Width(Vec<Complex>, f64, f64),
    ///each complex number is mapped to the first element in the tuple on the x axis
    Coord(Vec<(f64, Complex)>),
    ///3d data set where the first 2 floats are the starting x/y positions and the last 2 floats are
    ///the ending x/y positions,
    ///
    ///the ith element in the vector corrosponds to the (i % len)th element down the x axis
    ///and the (i / len)th element down the y axis
    ///
    ///expects square vector size
    Width3D(Vec<Complex>, f64, f64, f64, f64),
    ///each complex number is mapped to the first element in the tuple on the x axis
    ///and the second element in the tuple on the y axis
    Coord3D(Vec<(f64, f64, Complex)>),
    ///a constant value, in 2d second value determines weather its on the x or y axis
    Constant(Complex, bool),
    ///a point, 2d only
    Point(Vec2),
    ///a list of graphs, so that all graphs will be the same color
    List(Vec<GraphData>),
    None,
}
impl GraphData {
    pub fn set_type(&mut self, ty: GraphType, cap: usize) {
        match (self, ty) {
            (GraphData::Width(v, _, _), GraphType::Width) => v.reserve(cap.saturating_sub(v.len())),
            (GraphData::Coord(v), GraphType::Coord) => v.reserve(cap.saturating_sub(v.len())),
            (GraphData::Width3D(v, _, _, _, _), GraphType::Width3D) => {
                v.reserve(cap.saturating_sub(v.len()))
            }
            (GraphData::Coord3D(v), GraphType::Coord3D) => v.reserve(cap.saturating_sub(v.len())),
            (GraphData::Constant(_, _), GraphType::Constant) => {}
            (GraphData::Point(_), GraphType::Point) => {}
            (GraphData::List(_), GraphType::List) => {}
            (GraphData::None, GraphType::None) => {}
            (s, ty) => {
                *s = match ty {
                    GraphType::Width => GraphData::Width(Vec::with_capacity(cap), 0.0, 0.0),
                    GraphType::Coord => GraphData::Coord(Vec::with_capacity(cap)),
                    GraphType::Width3D => {
                        GraphData::Width3D(Vec::with_capacity(cap), 0.0, 0.0, 0.0, 0.0)
                    }
                    GraphType::Coord3D => GraphData::Coord3D(Vec::with_capacity(cap)),
                    GraphType::Constant => GraphData::Constant(Complex::Real(0.0), false),
                    GraphType::Point => GraphData::Point(Vec2::splat(0.0)),
                    GraphType::List => GraphData::List(Vec::with_capacity(cap)),
                    GraphType::None => GraphData::None,
                }
            }
        }
    }
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub enum GraphType {
    Width,
    Coord,
    Width3D,
    Coord3D,
    Constant,
    Point,
    List,
    None,
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Name {
    pub vars: Vec<String>,
    ///name of the function
    pub name: String,
    ///if the function has an imaginary part or not
    pub show: Show,
}
impl Name {
    pub fn new(name: String) -> Self {
        Name {
            vars: Vec::new(),
            name,
            show: Show::Real,
        }
    }
}
#[derive(Copy, Clone)]
pub(crate) enum Draw {
    Line(Pos, Pos, f32),
    Point(Pos),
}
pub enum Prec {
    ///a multiplier on the precision of the graph to update data on, potentially note Graph.prec
    Mult(f64),
    ///a multiplier on the precision of the graph to update data on, potentially note Graph.prec
    ///
    ///expecting you to only get the slice data
    Slice(f64),
    ///the amount of x/y data is requested for domain coloring
    Dimension(usize, usize),
}
pub enum Bound {
    ///a 2d data set is requested
    Width(f64, f64, Prec),
    ///a 3d data set is requested
    Width3D(f64, f64, f64, f64, Prec),
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default)]
pub enum Show {
    Real,
    Imag,
    #[default]
    Complex,
    None,
}
impl Show {
    pub fn real(&self) -> bool {
        matches!(self, Self::Complex | Self::Real)
    }
    pub fn imag(&self) -> bool {
        matches!(self, Self::Complex | Self::Imag)
    }
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Copy, Default)]
pub enum Lines {
    Points,
    LinesPoints,
    #[default]
    Lines,
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Copy, Default)]
pub enum DepthColor {
    ///colors based off of how far on the z axis the value is
    Vertical,
    ///colors based off of how close to the camera it is
    Depth,
    #[default]
    None,
}
#[cfg(feature = "egui")]
pub(crate) struct Image(pub egui::TextureHandle);
#[cfg(feature = "egui")]
impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "eguiimage")
    }
}
#[cfg(feature = "skia")]
#[derive(Debug)]
pub(crate) struct Image(pub skia_safe::Image);
#[cfg(feature = "skia")]
impl AsRef<skia_safe::Image> for Image {
    fn as_ref(&self) -> &skia_safe::Image {
        &self.0
    }
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Copy, Default)]
pub enum Angle {
    #[default]
    Radian,
    Degree,
    Gradian,
}
#[derive(Clone, Debug, Copy)]
pub(crate) enum Dragable {
    Point(Pos),
    Points((usize, Pos)),
    X(f32),
    Y(f32),
}
impl Angle {
    pub(crate) fn to_val(self, t: f64) -> f64 {
        match self {
            Angle::Radian => t,
            Angle::Degree => 180.0 * t / PI,
            Angle::Gradian => 200.0 * t / PI,
        }
    }
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub(crate) enum Change {
    Char((usize, usize), char, bool),
    Str((usize, usize), String, bool),
    Line(usize, bool, bool),
    None,
}
#[cfg(feature = "arboard")]
pub(crate) struct Clipboard(pub arboard::Clipboard);
#[cfg(feature = "arboard")]
impl std::fmt::Debug for Clipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "arboard")
    }
}
#[cfg(not(feature = "arboard"))]
#[derive(Debug)]
pub(crate) struct Clipboard(pub(crate) String);
impl Clipboard {
    #[cfg(feature = "arboard")]
    pub(crate) fn set_text(&mut self, text: &str) {
        self.0.set_text(text).unwrap_or_default()
    }
    #[cfg(feature = "arboard")]
    pub(crate) fn set_image(&mut self, width: usize, height: usize, bytes: &[u8]) {
        self.0
            .set_image(arboard::ImageData {
                width,
                height,
                bytes: bytes.into(),
            })
            .unwrap()
    }
    #[cfg(not(feature = "arboard"))]
    pub(crate) fn set_text(&mut self, text: &str) {
        self.0 = text.to_string();
    }
    #[cfg(feature = "arboard")]
    pub(crate) fn get_text(&mut self) -> String {
        self.0.get_text().unwrap_or_default()
    }
    #[cfg(not(feature = "arboard"))]
    pub(crate) fn get_text(&mut self) -> String {
        self.0.clone()
    }
}
#[cfg(feature = "tiny-skia")]
pub(crate) struct Image(pub tiny_skia::Pixmap);
#[cfg(feature = "wasm-draw")]
pub(crate) struct Image<'a>(pub &'a [u8], pub usize, pub usize);
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Graph {
    #[cfg(feature = "skia-vulkan")]
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) render_ctx: crate::skia_vulkan::context::VulkanRenderContext,
    ///vulkan surface
    #[cfg(feature = "skia-vulkan")]
    #[cfg_attr(feature = "serde", serde(skip))]
    pub renderer: Option<crate::skia_vulkan::renderer::VulkanRenderer>,
    ///current data sets
    #[cfg_attr(feature = "serde", serde(skip))]
    pub data: Vec<GraphData>,
    ///current data sets names for labeling, ordered by data
    #[cfg_attr(feature = "serde", serde(default))]
    pub names: Vec<Name>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg(feature = "wasm-draw")]
    pub(crate) cache: Option<Image<'static>>,
    #[cfg(not(feature = "wasm-draw"))]
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) cache: Option<Image>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) name_updated: Option<usize>,
    #[cfg(feature = "skia")]
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) font: Option<skia_safe::Font>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg(feature = "tiny-skia-text")]
    pub(crate) font: Option<bdf2::Font>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) font_size: f32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) font_width: f32,
    #[allow(clippy::type_complexity)]
    #[cfg_attr(feature = "serde", serde(skip))]
    ///for tab completion,will tab complete upto a "(" if it exists,
    /// so you may give info about which variables will be accepted
    pub tab_complete: Option<Box<dyn Fn(&str) -> Vec<String>>>,
    ///width of function lines
    #[cfg_attr(feature = "serde", serde(default))]
    pub line_width: f32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub point_size: f32,
    #[cfg(feature = "skia")]
    ///if Some, then returns bytes of an image format from update
    #[cfg_attr(feature = "serde", serde(default))]
    pub image_format: crate::ui::ImageFormat,
    #[cfg_attr(feature = "serde", serde(default))]
    pub fast_3d: bool,
    ///enable fast 3d only when moving with a mouse
    #[cfg_attr(feature = "serde", serde(default))]
    pub fast_3d_move: bool,
    ///request less data when moving with a mouse
    #[cfg_attr(feature = "serde", serde(default))]
    pub reduced_move: bool,
    ///current initial bound of window
    #[cfg_attr(feature = "serde", serde(default))]
    pub bound: Vec2,
    ///weather data is complex or not, changes graph mode options from keybinds
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_complex: bool,
    ///offset in 3d mode
    #[cfg_attr(feature = "serde", serde(default))]
    pub offset3d: Vec3,
    ///offset in 2d mode
    #[cfg_attr(feature = "serde", serde(default))]
    pub offset: Vec2,
    ///view angle in 3d mode
    #[cfg_attr(feature = "serde", serde(default))]
    pub angle: Vec2,
    ///weather bounds should be ignored in 3d mode
    #[cfg_attr(feature = "serde", serde(default))]
    pub ignore_bounds: bool,
    ///current zoom
    #[cfg_attr(feature = "serde", serde(default))]
    pub zoom: Vec2,
    ///current zoom for 3d
    #[cfg_attr(feature = "serde", serde(default))]
    pub zoom_3d: Vec3,
    ///what slice we are currently at in any slice mode
    #[cfg_attr(feature = "serde", serde(default))]
    pub slice: isize,
    ///var range used for flatten or depth
    #[cfg_attr(feature = "serde", serde(default))]
    pub var: Vec2,
    ///log scale for domain coloring
    #[cfg_attr(feature = "serde", serde(default))]
    pub log_scale: bool,
    ///how large the box should be in 3d
    #[cfg_attr(feature = "serde", serde(default))]
    pub box_size: f64,
    ///alternate domain coloring mode
    #[cfg_attr(feature = "serde", serde(default))]
    pub domain_alternate: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) screen: Vec2,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) screen_offset: Vec2,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) delta: f64,
    ///if real/imag should be displayed
    #[cfg_attr(feature = "serde", serde(default))]
    pub show: Show,
    ///weather some elements should be anti aliased or not
    #[cfg_attr(feature = "serde", serde(default))]
    pub anti_alias: bool,
    ///what color depth mode is currently enabled for 3d
    #[cfg_attr(feature = "serde", serde(default))]
    pub color_depth: DepthColor,
    ///weather all box lines should be displayed
    #[cfg_attr(feature = "serde", serde(default))]
    pub show_box: bool,
    ///colors of data sets for real part, ordered by data
    #[cfg_attr(feature = "serde", serde(default))]
    pub main_colors: Vec<Color>,
    ///colors of data sets for imag part, ordered by data
    #[cfg_attr(feature = "serde", serde(default))]
    pub alt_colors: Vec<Color>,
    ///major ticks axis color
    #[cfg_attr(feature = "serde", serde(default))]
    pub axis_color: Color,
    ///do not show graph with these indices
    #[cfg_attr(feature = "serde", serde(default))]
    pub blacklist_graphs: Vec<usize>,
    ///minor ticks axis color
    #[cfg_attr(feature = "serde", serde(default))]
    pub axis_color_light: Color,
    ///background color
    #[cfg_attr(feature = "serde", serde(default))]
    pub background_color: Color,
    ///text color
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) text_color: Color,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) mouse_position: Option<Vec2>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) mouse_moved: bool,
    ///weather non origin lines are disabled or not
    #[cfg_attr(feature = "serde", serde(default))]
    pub disable_lines: bool,
    ///weather axis text is disabled or not
    #[cfg_attr(feature = "serde", serde(default))]
    pub disable_axis: bool,
    ///weather mouse position is disabled or not
    #[cfg_attr(feature = "serde", serde(default))]
    pub disable_coord: bool,
    ///is slice viewing the x part or y part
    #[cfg_attr(feature = "serde", serde(default))]
    pub view_x: bool,
    ///current graph mode
    #[cfg_attr(feature = "serde", serde(default))]
    pub graph_mode: GraphMode,
    ///weather we are displaying a 3d plot or 2d
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_3d: bool,
    ///weather the data type supplied is naturally 3d or not
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_3d_data: bool,
    ///what angle type will be displayed
    #[cfg_attr(feature = "serde", serde(default))]
    pub angle_type: Angle,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) last_interact: Option<Vec2>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) last_right_interact: Option<Vec2>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) recalculate: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) name_modified: bool,
    ///current line style
    #[cfg_attr(feature = "serde", serde(default))]
    pub lines: Lines,
    ///current ruler position
    #[cfg_attr(feature = "serde", serde(default))]
    pub ruler_pos: Option<Vec2>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) prec: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) mouse_held: bool,
    ///how much extra reduced precision domain coloring should have
    #[cfg_attr(feature = "serde", serde(default))]
    pub mult: f64,
    ///how many major lines to display
    #[cfg_attr(feature = "serde", serde(default))]
    pub line_major: usize,
    ///how many minor lines inbetween major lines to display
    #[cfg_attr(feature = "serde", serde(default))]
    pub line_minor: usize,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) draw_offset: Pos,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) cos_phi: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) sin_phi: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) cos_theta: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) sin_theta: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) select: Option<(usize, usize, Option<bool>)>,
    ///where in side panel the cursor is at
    #[cfg_attr(feature = "serde", serde(default))]
    pub text_box: Option<(usize, usize)>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) side_slider: Option<usize>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) side_drag: Option<(usize, Option<usize>)>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) last_multi: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) side_bar_width: f64,
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) clipboard: Option<Clipboard>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) history: Vec<Change>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) history_pos: usize,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) text_scroll_pos: (usize, usize),
    ///do not show anything if it contains an imaginary part
    #[cfg_attr(feature = "serde", serde(default))]
    pub only_real: bool,
    ///what menu should be drawn
    #[cfg_attr(feature = "serde", serde(default))]
    pub menu: Menu,
    ///current keybinds, always some, besides during deserialization
    #[cfg_attr(feature = "serde", serde(skip))]
    pub keybinds: Option<Keybinds>,
    ///side bar height per line
    #[cfg_attr(feature = "serde", serde(default))]
    pub side_height: f32,
    ///in horizontal view, minimum width side bar will be in pixels
    #[cfg_attr(feature = "serde", serde(default))]
    pub min_side_width: f64,
    ///in horizontal view, maximum width main graph will be in pixels in side bar view
    #[cfg_attr(feature = "serde", serde(default))]
    pub min_screen_width: f64,
    ///in horizontal view, minimum ratio of the main screen will be targeted
    #[cfg_attr(feature = "serde", serde(default))]
    pub target_side_ratio: f64,
    /// bracket color based on depth of brackets
    #[cfg_attr(feature = "serde", serde(default))]
    pub bracket_color: Vec<Color>,
    /// what color the text drag select will be
    #[cfg_attr(feature = "serde", serde(default))]
    pub select_color: Color,
    #[cfg(feature = "arboard")]
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) wait_frame: bool,
    #[cfg(feature = "serde")]
    /// which file will save the serialization data
    #[cfg_attr(feature = "serde", serde(default))]
    pub save_file: String,
    #[cfg(feature = "serde")]
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) save_num: Option<usize>,
    #[cfg(feature = "serde")]
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) file_data: Option<Vec<(String, usize, String)>>,
    #[cfg(feature = "serde")]
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) file_data_raw: Option<Vec<String>>,
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) constant_eval: Vec<(usize, String)>,
    #[cfg(any(feature = "skia", feature = "tiny-skia", feature = "wasm-draw"))]
    #[cfg_attr(feature = "serde", serde(skip))]
    pub request_redraw: bool,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg(feature = "tiny-skia-text")]
    pub(crate) font_cache: std::collections::HashMap<char, tiny_skia::Pixmap>,
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) image_buffer: Vec<u8>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg(feature = "tiny-skia")]
    pub canvas: Option<tiny_skia::Pixmap>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg(all(feature = "skia", not(feature = "skia-vulkan")))]
    pub canvas: Option<skia_safe::Surface>,
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Copy, PartialEq, Default)]
pub enum Menu {
    #[default]
    Normal,
    Side,
    Settings,
    #[cfg(feature = "serde")]
    Load,
}
impl Default for Graph {
    fn default() -> Self {
        #[cfg(all(any(feature = "skia", feature = "tiny-skia-text"), feature = "serde"))]
        let terminus = &{
            #[cfg(any(target_os = "linux", feature = "tiny-skia"))]
            {
                let b = include_bytes!("../terminus.zstd");
                zstd::bulk::decompress(b, 240681).unwrap()
            }
            #[cfg(not(any(target_os = "linux", feature = "tiny-skia")))]
            {
                let b = include_bytes!("../terminus-ttf.zstd");
                zstd::bulk::decompress(b, 500668).unwrap()
            }
        };
        #[cfg(all(
            any(feature = "skia", feature = "tiny-skia-text"),
            not(feature = "serde"),
            any(target_os = "linux", feature = "tiny-skia-text")
        ))]
        let terminus = include_bytes!("../terminus.bdf");
        #[cfg(all(
            any(feature = "skia", feature = "tiny-skia-text"),
            not(feature = "serde"),
            not(any(target_os = "linux", feature = "tiny-skia-text"))
        ))]
        let terminus = include_bytes!("../terminus.ttf");
        #[cfg(feature = "skia")]
        let typeface = skia_safe::FontMgr::default()
            .new_from_data(terminus, None)
            .unwrap();
        let text_color = Color::splat(0);
        let font_size = 18.0;
        #[cfg(feature = "tiny-skia-text")]
        let font = bdf2::read(&terminus[..]).ok();
        #[cfg(feature = "skia")]
        let font = Some(skia_safe::Font::new(typeface, font_size));
        #[cfg(feature = "arboard")]
        let clipboard = None;
        #[cfg(not(feature = "arboard"))]
        let clipboard = Some(Clipboard(String::new()));
        Self {
            #[cfg(feature = "tiny-skia-text")]
            font_cache: crate::build_cache(&font, text_color),
            #[cfg(feature = "skia-vulkan")]
            render_ctx: Default::default(),
            #[cfg(feature = "skia-vulkan")]
            renderer: None,
            name_updated: None,
            is_3d: false,
            clipboard,
            #[cfg(feature = "arboard")]
            wait_frame: true,
            #[cfg(feature = "serde")]
            file_data: None,
            #[cfg(feature = "serde")]
            file_data_raw: None,
            image_buffer: Vec::new(),
            point_size: 5.0,
            history: Vec::new(),
            tab_complete: None,
            history_pos: 0,
            is_3d_data: false,
            constant_eval: Vec::new(),
            zoom_3d: Vec3::splat(1.0),
            names: Vec::new(),
            fast_3d: false,
            text_scroll_pos: (0, 0),
            data: Vec::new(),
            #[cfg(feature = "serde")]
            save_file: String::new(),
            #[cfg(feature = "serde")]
            save_num: None,
            cache: None,
            blacklist_graphs: Vec::new(),
            line_width: 3.0,
            #[cfg(any(feature = "skia", feature = "tiny-skia-text"))]
            font,
            font_size,
            font_width: 0.0,
            #[cfg(feature = "skia")]
            image_format: crate::ui::ImageFormat::Png,
            fast_3d_move: false,
            reduced_move: false,
            bound: Vec2::new(-2.0, 2.0),
            offset3d: Vec3::splat(0.0),
            offset: Vec2::splat(0.0),
            angle: Vec2::splat(PI / 6.0),
            slice: 0,
            mult: 1.0,
            text_box: None,
            line_major: 8,
            line_minor: 4,
            is_complex: false,
            show: Show::Complex,
            ignore_bounds: false,
            zoom: Vec2::splat(1.0),
            name_modified: false,
            draw_offset: Pos::new(0.0, 0.0),
            angle_type: Angle::Radian,
            mouse_held: false,
            menu: Menu::Normal,
            screen: Vec2::splat(0.0),
            screen_offset: Vec2::splat(0.0),
            delta: 0.0,
            show_box: true,
            select: None,
            log_scale: false,
            view_x: true,
            color_depth: DepthColor::None,
            box_size: 3.0f64.sqrt(),
            anti_alias: true,
            lines: Lines::Lines,
            domain_alternate: true,
            var: Vec2::new(-2.0, 2.0),
            #[cfg(any(feature = "skia", feature = "tiny-skia", feature = "wasm-draw"))]
            request_redraw: false,
            last_interact: None,
            last_right_interact: None,
            min_screen_width: 256.0,
            main_colors: vec![
                Color::new(255, 85, 85),
                Color::new(85, 85, 255),
                Color::new(255, 85, 255),
                Color::new(85, 255, 85),
                Color::new(85, 255, 255),
                Color::new(255, 255, 85),
            ],
            alt_colors: vec![
                Color::new(170, 0, 0),
                Color::new(0, 0, 170),
                Color::new(170, 0, 170),
                Color::new(0, 170, 0),
                Color::new(0, 170, 170),
                Color::new(170, 170, 0),
            ],
            axis_color: Color::splat(0),
            axis_color_light: Color::splat(220),
            text_color,
            background_color: Color::splat(255),
            mouse_position: None,
            mouse_moved: false,
            disable_lines: false,
            disable_axis: false,
            disable_coord: true,
            side_slider: None,
            side_drag: None,
            graph_mode: GraphMode::Normal,
            last_multi: false,
            prec: 1.0,
            side_bar_width: 0.0,
            side_height: 1.875,
            recalculate: false,
            ruler_pos: None,
            bracket_color: vec![
                Color::new(255, 85, 85),
                //Color::new(85, 255, 85),
                //Color::new(255, 255, 85),
                Color::new(85, 85, 255),
                Color::new(255, 85, 255),
                //Color::new(85, 255, 255),
            ],
            cos_phi: 0.0,
            sin_phi: 0.0,
            cos_theta: 0.0,
            sin_theta: 0.0,
            only_real: false,
            keybinds: Some(Keybinds::default()),
            target_side_ratio: 3.0 / 2.0,
            min_side_width: 256.0,
            select_color: Color::new(191, 191, 255),
            #[cfg(any(
                all(feature = "skia", not(feature = "skia-vulkan")),
                feature = "tiny-skia"
            ))]
            canvas: None,
        }
    }
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Debug, Clone, PartialEq)]
pub struct Keybinds {
    ///moves left on the x axis in 2d, rotates left in 3d
    pub left: Option<Keys>,
    ///moves right on the x axis in 2d, rotates right in 3d
    pub right: Option<Keys>,
    ///moves up on the y axis in 2d, rotates up in 3d
    pub up: Option<Keys>,
    ///moves down on the y axis in 2d, rotates down in 3d
    pub down: Option<Keys>,
    ///moves viewport left in 3d
    pub left_3d: Option<Keys>,
    ///moves viewport right in 3d
    pub right_3d: Option<Keys>,
    ///moves viewport up in 3d
    pub up_3d: Option<Keys>,
    ///moves viewport down in 3d
    pub down_3d: Option<Keys>,
    ///in 3d, moves up on the z axis
    pub in_3d: Option<Keys>,
    ///in 3d, moves down on the z axis
    pub out_3d: Option<Keys>,
    ///zooms the into the data set, in 2d, towards the cursor if moved since last reset,
    ///otherwise towards center of screen
    pub zoom_in: Option<Keys>,
    ///zooms the out of the data set, in 2d, away from cursor if moved since last reset,
    ///otherwise towards center of screen
    pub zoom_out: Option<Keys>,
    pub zoom_in_x: Option<Keys>,
    pub zoom_out_x: Option<Keys>,
    pub zoom_in_y: Option<Keys>,
    pub zoom_out_y: Option<Keys>,
    pub zoom_in_z: Option<Keys>,
    pub zoom_out_z: Option<Keys>,
    ///toggles non center lines in 2d, or all lines with axis aditionally disabled
    pub lines: Option<Keys>,
    ///toggles display of axis numbers, or all lines with axis aditionally disabled
    pub axis: Option<Keys>,
    ///toggles current coordonate of mouse in bottom left, or angle in 3d
    pub coord: Option<Keys>,
    ///toggles anti alias for some things
    pub anti_alias: Option<Keys>,
    ///in 3d, ignores the bounds of the box and displays all data points
    pub ignore_bounds: Option<Keys>,
    ///in 3d, toggles the color depth enum
    pub color_depth: Option<Keys>,
    ///makes viewport larger in 3d
    pub zoom_in_3d: Option<Keys>,
    ///makes viewport smaller in 3d
    pub zoom_out_3d: Option<Keys>,
    ///in 3d, shows the full box instead of just the axis lines,
    ///or none if additionally axis is disabled
    pub show_box: Option<Keys>,
    ///toggles domain alternate mode, see Graph.domain_alternate for more info
    pub domain_alternate: Option<Keys>,
    ///iterates Graph.slice foward
    pub slice_up: Option<Keys>,
    ///iterates Graph.slice backward
    pub slice_down: Option<Keys>,
    ///toggles Graph.view_x
    pub slice_view: Option<Keys>,
    ///log scale, currently only for domain coloring
    pub log_scale: Option<Keys>,
    ///toggles line style enum
    pub line_style: Option<Keys>,
    ///for flatten or depth graph modes, move the input variables range foward
    pub var_up: Option<Keys>,
    ///for flatten or depth graph modes, move the input variables range backward
    pub var_down: Option<Keys>,
    ///for flatten or depth graph modes, decrease range of input variables range
    pub var_in: Option<Keys>,
    ///for flatten or depth graph modes, incrase range of input variables range
    pub var_out: Option<Keys>,
    ///increases amount of data asked for
    pub prec_up: Option<Keys>,
    ///decreases amount of data asked for
    pub prec_down: Option<Keys>,
    ///toggles a ruler at current mouse position, in bottom right will have the following info,
    ///delta x of ruler
    ///delta y of ruler
    ///norm of ruler
    ///angle of ruler in degrees
    pub ruler: Option<Keys>,
    ///toggles showing real/imag parts of graphs
    pub view: Option<Keys>,
    ///toggles the current graph mode enum foward
    pub mode_up: Option<Keys>,
    ///toggles the current graph mode enum backward
    pub mode_down: Option<Keys>,
    ///resets most settings to default
    pub reset: Option<Keys>,
    ///toggles up the side menu
    pub side: Option<Keys>,
    ///toggles using faster logic in 2d/3d
    pub fast: Option<Keys>,
    #[cfg(feature = "serde")]
    ///copys tiny serialized data to clipboard
    pub save: Option<Keys>,
    #[cfg(feature = "serde")]
    ///full saves the graph into the Graph.save_file directory
    pub full_save: Option<Keys>,
    #[cfg(feature = "serde")]
    ///applys tiny serialized data from clipboard
    pub paste: Option<Keys>,
    ///settings menu
    pub settings: Option<Keys>,
    #[cfg(feature = "serde")]
    ///load from full saves
    pub load: Option<Keys>,
    #[cfg(any(feature = "skia", feature = "tiny-skia"))]
    ///save screen to clipboard
    pub save_png: Option<Keys>,
    ///only shows real values, and ignores real values if they have an imaginary part
    pub only_real: Option<Keys>,
    ///toggles dark mode
    pub toggle_dark_mode: Option<Keys>,
}
#[cfg(feature = "serde")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
#[derive(Clone, Debug, Default)]
pub struct GraphTiny {
    pub names: Vec<Name>,
    pub bound: (f32, f32),
    pub prec: f32,
    pub is_complex: bool,
    pub offset3d: Option<(f32, f32, f32)>,
    pub offset: Option<(f32, f32)>,
    pub zoom: Option<(f32, f32)>,
    pub zoom_3d: Option<(f32, f32, f32)>,
    pub slice: i8,
    pub var: (f32, f32),
    pub log_scale: bool,
    pub domain_alternate: bool,
    pub color_depth: DepthColor,
    pub blacklist_graphs: Vec<u8>,
    pub view_x: bool,
    pub graph_mode: GraphMode,
    pub only_real: bool,
}
#[cfg(feature = "serde")]
impl Graph {
    pub fn to_tiny(&self) -> GraphTiny {
        let (a, b) = self.to_coord((self.screen / 2.0).to_pos());
        GraphTiny {
            names: self
                .names
                .iter()
                .filter_map(|n| {
                    if n.vars.is_empty() && n.name.is_empty() {
                        None
                    } else {
                        Some(n.clone())
                    }
                })
                .collect(),
            bound: self.bound.to_tuple(),
            prec: self.prec as f32,
            is_complex: self.is_complex,
            offset3d: self.is_3d.then_some(self.offset3d.to_tuple()),
            offset: (!self.is_3d).then_some((a as f32, b as f32)),
            zoom: (!self.is_3d).then_some(self.zoom.to_tuple()),
            zoom_3d: self.is_3d.then_some(self.zoom_3d.to_tuple()),
            slice: self.slice as i8,
            var: self.var.to_tuple(),
            log_scale: self.log_scale,
            domain_alternate: self.domain_alternate,
            color_depth: self.color_depth,
            blacklist_graphs: self.blacklist_graphs.iter().map(|i| *i as u8).collect(),
            view_x: self.view_x,
            graph_mode: self.graph_mode,
            only_real: self.only_real,
        }
    }
    pub fn apply_tiny(&mut self, tiny: GraphTiny) {
        self.names = tiny.names;
        self.bound = tiny.bound.into();
        self.is_complex = tiny.is_complex;
        self.prec = tiny.prec as f64;
        self.offset3d = tiny.offset3d.unwrap_or_default().into();
        self.zoom = tiny.zoom.unwrap_or((1.0, 1.0)).into();
        self.zoom_3d = tiny.zoom_3d.unwrap_or((1.0, 1.0, 1.0)).into();
        let o: Vec2 = tiny.offset.unwrap_or_default().into();
        self.offset = self.get_new_offset(o);
        self.slice = tiny.slice as isize;
        self.var = tiny.var.into();
        self.log_scale = tiny.log_scale;
        self.domain_alternate = tiny.domain_alternate;
        self.color_depth = tiny.color_depth;
        self.blacklist_graphs = tiny.blacklist_graphs.iter().map(|i| *i as usize).collect();
        self.view_x = tiny.view_x;
        self.graph_mode = tiny.graph_mode;
        self.only_real = tiny.only_real;
        self.recalculate(None);
        self.name_modified(None);
        self.text_box = Some((0, 0));
    }
}
#[cfg(feature = "serde")]
impl TryFrom<&String> for GraphTiny {
    type Error = ();
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        let (a, b) = value.rsplit_once('@').ok_or(())?;
        let l = base64::prelude::BASE64_URL_SAFE_NO_PAD
            .decode(a)
            .map_err(|_| ())?;
        let l = String::from_utf8(l)
            .map_err(|_| ())?
            .parse::<usize>()
            .map_err(|_| ())?;
        let comp = base64::prelude::BASE64_URL_SAFE_NO_PAD
            .decode(b)
            .map_err(|_| ())?;
        let seri = zstd::bulk::decompress(&comp, l).map_err(|_| ())?;
        bitcode::deserialize(&seri).map_err(|_| ())
    }
}
impl Default for Keybinds {
    fn default() -> Self {
        Self {
            left: Some(Keys::new(Key::ArrowLeft)),
            right: Some(Keys::new(Key::ArrowRight)),
            up: Some(Keys::new(Key::ArrowUp)),
            down: Some(Keys::new(Key::ArrowDown)),
            zoom_in: Some(Keys::new(Key::Equals)),
            zoom_out: Some(Keys::new(Key::Minus)),
            zoom_in_x: Some(Keys::new_with_modifier(
                Key::Equals,
                Modifiers::default().ctrl(),
            )),
            zoom_out_x: Some(Keys::new_with_modifier(
                Key::Minus,
                Modifiers::default().ctrl(),
            )),
            zoom_in_y: Some(Keys::new_with_modifier(
                Key::Plus,
                Modifiers::default().shift(),
            )),
            zoom_out_y: Some(Keys::new_with_modifier(
                Key::Underscore,
                Modifiers::default().shift(),
            )),
            zoom_in_z: Some(Keys::new_with_modifier(
                Key::Plus,
                Modifiers::default().shift().ctrl(),
            )),
            zoom_out_z: Some(Keys::new_with_modifier(
                Key::Underscore,
                Modifiers::default().shift().ctrl(),
            )),
            lines: Some(Keys::new(Key::Z)),
            axis: Some(Keys::new(Key::X)),
            left_3d: Some(Keys::new_with_modifier(
                Key::ArrowLeft,
                Modifiers::default().ctrl(),
            )),
            right_3d: Some(Keys::new_with_modifier(
                Key::ArrowRight,
                Modifiers::default().ctrl(),
            )),
            up_3d: Some(Keys::new_with_modifier(
                Key::ArrowUp,
                Modifiers::default().ctrl(),
            )),
            down_3d: Some(Keys::new_with_modifier(
                Key::ArrowDown,
                Modifiers::default().ctrl(),
            )),
            in_3d: Some(Keys::new_with_modifier(
                Key::ArrowDown,
                Modifiers::default().ctrl().alt(),
            )),
            out_3d: Some(Keys::new_with_modifier(
                Key::ArrowUp,
                Modifiers::default().ctrl().alt(),
            )),
            #[cfg(feature = "serde")]
            save: Some(Keys::new_with_modifier(Key::S, Modifiers::default().ctrl())),
            #[cfg(feature = "serde")]
            full_save: Some(Keys::new_with_modifier(
                Key::S,
                Modifiers::default().ctrl().shift(),
            )),
            #[cfg(feature = "serde")]
            paste: Some(Keys::new_with_modifier(Key::P, Modifiers::default().ctrl())),
            coord: Some(Keys::new(Key::C)),
            anti_alias: Some(Keys::new(Key::R)),
            ignore_bounds: Some(Keys::new(Key::P)),
            color_depth: Some(Keys::new(Key::O)),
            zoom_in_3d: Some(Keys::new(Key::Semicolon)),
            zoom_out_3d: Some(Keys::new(Key::Quote)),
            show_box: Some(Keys::new(Key::U)),
            domain_alternate: Some(Keys::new(Key::Y)),
            slice_up: Some(Keys::new(Key::Period)),
            slice_down: Some(Keys::new(Key::Comma)),
            slice_view: Some(Keys::new(Key::Slash)),
            log_scale: Some(Keys::new_with_modifier(Key::L, Modifiers::default().ctrl())),
            line_style: Some(Keys::new(Key::L)),
            var_up: Some(Keys::new_with_modifier(
                Key::ArrowRight,
                Modifiers::default().shift(),
            )),
            var_down: Some(Keys::new_with_modifier(
                Key::ArrowLeft,
                Modifiers::default().shift(),
            )),
            var_in: Some(Keys::new_with_modifier(
                Key::ArrowUp,
                Modifiers::default().shift(),
            )),
            var_out: Some(Keys::new_with_modifier(
                Key::ArrowDown,
                Modifiers::default().shift(),
            )),
            prec_up: Some(Keys::new(Key::OpenBracket)),
            prec_down: Some(Keys::new(Key::CloseBracket)),
            ruler: Some(Keys::new(Key::N)),
            view: Some(Keys::new(Key::I)),
            mode_up: Some(Keys::new(Key::B)),
            mode_down: Some(Keys::new_with_modifier(
                Key::B,
                Modifiers::default().shift(),
            )),
            reset: Some(Keys::new(Key::T)),
            side: Some(Keys::new(Key::Escape)),
            fast: Some(Keys::new(Key::F)),
            settings: None, /*Some(Keys::new_with_modifier( TODO
                                Key::Escape,
                                Modifiers::default().ctrl(),
                            ))*/
            #[cfg(feature = "serde")]
            load: Some(Keys::new_with_modifier(
                Key::Escape,
                Modifiers::default().shift(),
            )),
            #[cfg(any(feature = "skia", feature = "tiny-skia"))]
            save_png: Some(Keys::new_with_modifier(Key::S, Modifiers::default().alt())),
            toggle_dark_mode: Some(Keys::new_with_modifier(
                Key::D,
                Modifiers::default().shift().ctrl(),
            )),
            only_real: Some(Keys::new_with_modifier(
                Key::O,
                Modifiers::default().shift().ctrl(),
            )),
        }
    }
}
pub struct Multi {
    ///how much touch input has zoomed in this frame
    pub zoom_delta: f64,
    ///how much touch input translated in this frame
    pub translation_delta: Vec2,
}
pub struct InputState {
    ///which keys have been pressed this frame
    pub keys_pressed: Vec<Key>,
    ///which modifiers are pressed
    pub modifiers: Modifiers,
    ///how much scroll wheel has scrolled
    pub raw_scroll_delta: Vec2,
    ///where the pointer is currently
    pub pointer_pos: Option<Vec2>,
    ///some if pointer is down, true if this frame pointer was pressed
    pub pointer: Option<bool>,
    ///some if pointer is down, true if this frame pointer was pressed
    pub pointer_right: Option<bool>,
    ///Some if multiple touch inputs have been detected
    pub multi: Option<Multi>,
    #[cfg(any(feature = "egui", target_arch = "wasm32"))]
    pub clipboard_override: Option<String>,
}
impl Default for InputState {
    fn default() -> Self {
        Self {
            keys_pressed: Vec::new(),
            modifiers: Modifiers::default(),
            raw_scroll_delta: Vec2::splat(0.0),
            pointer_pos: None,
            pointer: None,
            pointer_right: None,
            multi: None,
            #[cfg(any(feature = "egui", target_arch = "wasm32"))]
            clipboard_override: None,
        }
    }
}
impl InputState {
    ///resets raw_scroll_delta, keys_pressed, pointer_just_down, multi,
    ///expected to happen after update()
    pub fn reset(&mut self) {
        self.raw_scroll_delta = Vec2::splat(0.0);
        self.keys_pressed = Vec::new();
        if self.pointer.is_some() {
            self.pointer = Some(false);
        }
        if self.pointer_right.is_some() {
            self.pointer_right = Some(false);
        }
        self.multi = None;
    }
}
#[cfg(feature = "egui")]
impl From<&egui::InputState> for InputState {
    fn from(val: &egui::InputState) -> Self {
        let pointer = if val.pointer.primary_down() {
            Some(val.pointer.primary_pressed())
        } else {
            None
        };
        let pointer_right = if val.pointer.secondary_down() {
            Some(val.pointer.secondary_pressed())
        } else {
            None
        };
        let mut clipboard_override = None;
        let keys_pressed = val
            .events
            .iter()
            .filter_map(|event| match event {
                egui::Event::Copy => Some(Key::C),
                egui::Event::Cut => Some(Key::X),
                egui::Event::Paste(s) => {
                    clipboard_override = Some(s.clone());
                    Some(Key::V)
                }
                egui::Event::Key {
                    key, pressed: true, ..
                } => Some(key.into()),
                egui::Event::Text(s) => match s.as_str() {
                    "^" => Some(Key::Caret),
                    "#" => Some(Key::HashTag),
                    "(" => Some(Key::OpenParentheses),
                    ")" => Some(Key::CloseParentheses),
                    "&" => Some(Key::And),
                    "%" => Some(Key::Percent),
                    "_" => Some(Key::Underscore),
                    "<" => Some(Key::LessThen),
                    ">" => Some(Key::GreaterThen),
                    "±" => Some(Key::PlusMinus),
                    "\"" => Some(Key::DoubleQuote),
                    "$" => Some(Key::Dollar),
                    "¢" => Some(Key::Cent),
                    "~" => Some(Key::Tilde),
                    "*" => Some(Key::Mult),
                    _ => None,
                },
                _ => None,
            })
            .collect::<Vec<Key>>();
        InputState {
            keys_pressed,
            modifiers: val.modifiers.into(),
            raw_scroll_delta: Vec2 {
                x: val.raw_scroll_delta.x as f64,
                y: val.raw_scroll_delta.y as f64,
            },
            pointer_pos: val
                .pointer
                .latest_pos()
                .map(|a| Vec2::new(a.x as f64, a.y as f64)),
            pointer,
            pointer_right,
            multi: val.multi_touch().map(|i| Multi {
                translation_delta: Vec2::new(
                    i.translation_delta.x as f64,
                    i.translation_delta.y as f64,
                ),
                zoom_delta: i.zoom_delta as f64,
            }),
            clipboard_override,
        }
    }
}
impl InputState {
    pub(crate) fn keys_pressed(&self, keys: Option<Keys>) -> bool {
        if let Some(keys) = keys {
            keys.modifiers
                .map(|m| self.modifiers == m)
                .unwrap_or(self.modifiers.is_false())
                && self.keys_pressed.contains(&keys.key)
        } else {
            false
        }
    }
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Debug, Clone, PartialEq)]
pub struct Keys {
    ///None is equivalent to a set of false Modifiers
    modifiers: Option<Modifiers>,
    key: Key,
}
impl Keys {
    pub fn new(key: Key) -> Self {
        Self {
            key,
            modifiers: None,
        }
    }
    pub fn new_with_modifier(key: Key, modifiers: Modifiers) -> Self {
        Self {
            key,
            modifiers: Some(modifiers),
        }
    }
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Debug, Clone, PartialEq, Default)]
pub struct Modifiers {
    pub alt: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub mac_cmd: bool,
    pub command: bool,
}
#[cfg(feature = "egui")]
impl From<Modifiers> for egui::Modifiers {
    fn from(val: Modifiers) -> Self {
        egui::Modifiers {
            alt: val.alt,
            ctrl: val.ctrl,
            shift: val.shift,
            mac_cmd: val.mac_cmd,
            command: val.command,
        }
    }
}
#[cfg(feature = "egui")]
impl From<egui::Modifiers> for Modifiers {
    fn from(val: egui::Modifiers) -> Self {
        Modifiers {
            alt: val.alt,
            ctrl: val.ctrl,
            shift: val.shift,
            mac_cmd: val.mac_cmd,
            command: val.command,
        }
    }
}
impl Modifiers {
    pub(crate) fn is_false(&self) -> bool {
        !self.mac_cmd && !self.alt && !self.command && !self.ctrl && !self.shift
    }
    pub fn alt(mut self) -> Self {
        self.alt = true;
        self
    }
    pub fn ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }
    pub fn shift(mut self) -> Self {
        self.shift = true;
        self
    }
    pub fn mac_cmd(mut self) -> Self {
        self.mac_cmd = true;
        self
    }
    pub fn command(mut self) -> Self {
        self.command = true;
        self
    }
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
impl Color {
    pub(crate) fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
    pub(crate) fn splat(c: u8) -> Self {
        Self { r: c, g: c, b: c }
    }
    #[cfg(feature = "wasm-draw")]
    pub(crate) fn to_col(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
    #[cfg(feature = "egui")]
    pub(crate) fn to_col(self) -> egui::Color32 {
        egui::Color32::from_rgb(self.r, self.g, self.b)
    }
    #[cfg(feature = "skia")]
    pub(crate) fn to_col(self) -> skia_safe::Color4f {
        skia_safe::Color4f::new(
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            1.0,
        )
    }
    #[cfg(feature = "tiny-skia")]
    pub(crate) fn to_col(self) -> tiny_skia::Color {
        #[cfg(target_arch = "wasm32")]
        let c = tiny_skia::Color::from_rgba8(self.r, self.g, self.b, 255);
        #[cfg(not(target_arch = "wasm32"))]
        let c = tiny_skia::Color::from_rgba8(self.b, self.g, self.r, 255);
        c
    }
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Debug, Clone, PartialEq, Default)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}
impl Pos {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn to_vec(&self) -> Vec2 {
        Vec2 {
            x: self.x as f64,
            y: self.y as f64,
        }
    }
    #[cfg(feature = "egui")]
    pub(crate) fn to_pos2(self) -> egui::Pos2 {
        egui::Pos2 {
            x: self.x,
            y: self.y,
        }
    }
    #[cfg(feature = "skia")]
    pub(crate) fn to_pos2(self) -> skia_safe::Point {
        skia_safe::Point::new(self.x, self.y)
    }
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Debug, Clone)]
pub enum Complex {
    Real(f64),
    Imag(f64),
    Complex(f64, f64),
}
impl Complex {
    pub fn to_options(self) -> (Option<f64>, Option<f64>) {
        match self {
            Complex::Real(y) => (Some(y), None),
            Complex::Imag(z) => (None, Some(z)),
            Complex::Complex(y, z) => (Some(y), Some(z)),
        }
    }
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Debug, Clone, PartialEq, Default)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}
impl Vec2 {
    pub fn norm(&self) -> f64 {
        self.y.hypot(self.x)
    }
    pub fn splat(v: f64) -> Self {
        Self { x: v, y: v }
    }
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub(crate) fn to_pos(self) -> Pos {
        Pos {
            x: self.x as f32,
            y: self.y as f32,
        }
    }
    pub fn to_tuple(self) -> (f32, f32) {
        (self.x as f32, self.y as f32)
    }
}
impl From<(f32, f32)> for Vec2 {
    fn from(value: (f32, f32)) -> Self {
        Self {
            x: value.0 as f64,
            y: value.1 as f64,
        }
    }
}
impl From<(f64, f64)> for Vec2 {
    fn from(value: (f64, f64)) -> Self {
        Self {
            x: value.0,
            y: value.1,
        }
    }
}
impl From<(f32, f32, f32)> for Vec3 {
    fn from(value: (f32, f32, f32)) -> Self {
        Self {
            x: value.0 as f64,
            y: value.1 as f64,
            z: value.2 as f64,
        }
    }
}
impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Self) -> Self::Output {
        Vec2::new(self.x - rhs.x, self.y - rhs.y)
    }
}
impl Sub for &Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Self) -> Self::Output {
        Vec2::new(self.x - rhs.x, self.y - rhs.y)
    }
}
impl Add for &Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Self) -> Self::Output {
        Vec2::new(self.x + rhs.x, self.y + rhs.y)
    }
}
impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Self) -> Self::Output {
        Vec2::new(self.x + rhs.x, self.y + rhs.y)
    }
}
impl DivAssign<f64> for Vec2 {
    fn div_assign(&mut self, rhs: f64) {
        self.x /= rhs;
        self.y /= rhs;
    }
}
impl Sum for Vec2 {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Vec2::splat(0.0), |a, b| a + b)
    }
}
impl MulAssign<f64> for Vec2 {
    fn mul_assign(&mut self, rhs: f64) {
        self.x *= rhs;
        self.y *= rhs;
    }
}
impl From<(f64, f64)> for Complex {
    fn from(value: (f64, f64)) -> Self {
        Complex::Complex(value.0, value.1)
    }
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Debug, Clone, Default)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
impl Vec3 {
    pub fn splat(v: f64) -> Self {
        Self { x: v, y: v, z: v }
    }
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    pub fn to_tuple(self) -> (f32, f32, f32) {
        (self.x as f32, self.y as f32, self.z as f32)
    }
}
impl AddAssign<Vec2> for Vec2 {
    fn add_assign(&mut self, rhs: Vec2) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}
impl SubAssign<Vec2> for Vec2 {
    fn sub_assign(&mut self, rhs: Vec2) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}
impl MulAssign<f64> for Vec3 {
    fn mul_assign(&mut self, rhs: f64) {
        self.x *= rhs;
        self.y *= rhs;
        self.z *= rhs;
    }
}
impl DivAssign<f64> for Vec3 {
    fn div_assign(&mut self, rhs: f64) {
        self.x /= rhs;
        self.y /= rhs;
        self.z /= rhs;
    }
}
impl Div<Vec3> for Vec3 {
    type Output = Vec3;
    fn div(mut self, rhs: Vec3) -> Self::Output {
        self.x /= rhs.x;
        self.y /= rhs.y;
        self.z /= rhs.z;
        self
    }
}
impl DivAssign<Vec3> for Vec3 {
    fn div_assign(&mut self, rhs: Vec3) {
        self.x /= rhs.x;
        self.y /= rhs.y;
        self.z /= rhs.z;
    }
}
impl MulAssign<Vec3> for Vec3 {
    fn mul_assign(&mut self, rhs: Vec3) {
        self.x *= rhs.x;
        self.y *= rhs.y;
        self.z *= rhs.z;
    }
}
impl Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, rhs: f64) -> Self::Output {
        Vec3::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}
impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: Self) -> Self::Output {
        Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}
impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: Self) -> Self::Output {
        Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}
impl Add for Pos {
    type Output = Pos;
    fn add(self, rhs: Self) -> Self::Output {
        Pos::new(self.x + rhs.x, self.y + rhs.y)
    }
}
impl Sub for Pos {
    type Output = Pos;
    fn sub(self, rhs: Self) -> Self::Output {
        Pos::new(self.x - rhs.x, self.y - rhs.y)
    }
}
impl Mul<f32> for Pos {
    type Output = Pos;
    fn mul(self, rhs: f32) -> Self::Output {
        Pos::new(self.x * rhs, self.y * rhs)
    }
}
impl Div<f32> for Pos {
    type Output = Pos;
    fn div(self, rhs: f32) -> Self::Output {
        Pos::new(self.x / rhs, self.y / rhs)
    }
}
impl Div<f64> for Vec2 {
    type Output = Vec2;
    fn div(self, rhs: f64) -> Self::Output {
        Vec2::new(self.x / rhs, self.y / rhs)
    }
}
#[derive(Copy, Clone)]
pub(crate) enum Align {
    LeftBottom,
    LeftCenter,
    LeftTop,
    #[allow(dead_code)]
    CenterBottom,
    #[allow(dead_code)]
    CenterCenter,
    CenterTop,
    RightBottom,
    RightCenter,
    RightTop,
}
#[cfg(feature = "egui")]
impl From<Align> for egui::Align2 {
    fn from(val: Align) -> Self {
        match val {
            Align::LeftBottom => egui::Align2::LEFT_BOTTOM,
            Align::LeftCenter => egui::Align2::LEFT_CENTER,
            Align::LeftTop => egui::Align2::LEFT_TOP,
            Align::CenterBottom => egui::Align2::CENTER_BOTTOM,
            Align::CenterCenter => egui::Align2::CENTER_CENTER,
            Align::CenterTop => egui::Align2::CENTER_TOP,
            Align::RightBottom => egui::Align2::RIGHT_BOTTOM,
            Align::RightCenter => egui::Align2::RIGHT_CENTER,
            Align::RightTop => egui::Align2::RIGHT_TOP,
        }
    }
}
#[cfg(feature = "skia")]
impl From<Align> for skia_safe::utils::text_utils::Align {
    fn from(val: Align) -> Self {
        match val {
            Align::LeftBottom => skia_safe::utils::text_utils::Align::Left,
            Align::LeftCenter => skia_safe::utils::text_utils::Align::Left,
            Align::LeftTop => skia_safe::utils::text_utils::Align::Left,
            Align::CenterBottom => skia_safe::utils::text_utils::Align::Center,
            Align::CenterCenter => skia_safe::utils::text_utils::Align::Center,
            Align::CenterTop => skia_safe::utils::text_utils::Align::Center,
            Align::RightBottom => skia_safe::utils::text_utils::Align::Right,
            Align::RightCenter => skia_safe::utils::text_utils::Align::Right,
            Align::RightTop => skia_safe::utils::text_utils::Align::Right,
        }
    }
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Debug, Clone, PartialEq)]
pub enum Key {
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    Escape,
    Tab,
    Backspace,
    Enter,
    Space,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Copy,
    Cut,
    Paste,
    Colon,
    Comma,
    Backslash,
    Slash,
    Pipe,
    Questionmark,
    Exclamationmark,
    OpenBracket,
    CloseBracket,
    OpenCurlyBracket,
    CloseCurlyBracket,
    Backtick,
    Minus,
    Period,
    Plus,
    Equals,
    Semicolon,
    Quote,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Caret,
    HashTag,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    F25,
    F26,
    F27,
    F28,
    F29,
    F30,
    F31,
    F32,
    F33,
    F34,
    F35,
    OpenParentheses,
    CloseParentheses,
    And,
    Percent,
    Underscore,
    LessThen,
    GreaterThen,
    PlusMinus,
    DoubleQuote,
    Dollar,
    Cent,
    Tilde,
    Mult,
    Undefined,
}
#[cfg(feature = "egui")]
impl From<Key> for egui::Key {
    fn from(val: Key) -> Self {
        match val {
            Key::ArrowDown => egui::Key::ArrowDown,
            Key::ArrowLeft => egui::Key::ArrowLeft,
            Key::ArrowRight => egui::Key::ArrowRight,
            Key::ArrowUp => egui::Key::ArrowUp,
            Key::Escape => egui::Key::Escape,
            Key::Tab => egui::Key::Tab,
            Key::Backspace => egui::Key::Backspace,
            Key::Enter => egui::Key::Enter,
            Key::Space => egui::Key::Space,
            Key::Insert => egui::Key::Insert,
            Key::Delete => egui::Key::Delete,
            Key::Home => egui::Key::Home,
            Key::End => egui::Key::End,
            Key::PageUp => egui::Key::PageUp,
            Key::PageDown => egui::Key::PageDown,
            Key::Copy => egui::Key::Copy,
            Key::Cut => egui::Key::Cut,
            Key::Paste => egui::Key::Paste,
            Key::Colon => egui::Key::Colon,
            Key::Comma => egui::Key::Comma,
            Key::Backslash => egui::Key::Backslash,
            Key::Slash => egui::Key::Slash,
            Key::Pipe => egui::Key::Pipe,
            Key::Questionmark => egui::Key::Questionmark,
            Key::Exclamationmark => egui::Key::Exclamationmark,
            Key::OpenBracket => egui::Key::OpenBracket,
            Key::CloseBracket => egui::Key::CloseBracket,
            Key::OpenCurlyBracket => egui::Key::OpenCurlyBracket,
            Key::CloseCurlyBracket => egui::Key::CloseCurlyBracket,
            Key::Backtick => egui::Key::Backtick,
            Key::Minus => egui::Key::Minus,
            Key::Period => egui::Key::Period,
            Key::Plus => egui::Key::Plus,
            Key::Equals => egui::Key::Equals,
            Key::Semicolon => egui::Key::Semicolon,
            Key::Quote => egui::Key::Quote,
            Key::Num0 => egui::Key::Num0,
            Key::Num1 => egui::Key::Num1,
            Key::Num2 => egui::Key::Num2,
            Key::Num3 => egui::Key::Num3,
            Key::Num4 => egui::Key::Num4,
            Key::Num5 => egui::Key::Num5,
            Key::Num6 => egui::Key::Num6,
            Key::Num7 => egui::Key::Num7,
            Key::Num8 => egui::Key::Num8,
            Key::Num9 => egui::Key::Num9,
            Key::A => egui::Key::A,
            Key::B => egui::Key::B,
            Key::C => egui::Key::C,
            Key::D => egui::Key::D,
            Key::E => egui::Key::E,
            Key::F => egui::Key::F,
            Key::G => egui::Key::G,
            Key::H => egui::Key::H,
            Key::I => egui::Key::I,
            Key::J => egui::Key::J,
            Key::K => egui::Key::K,
            Key::L => egui::Key::L,
            Key::M => egui::Key::M,
            Key::N => egui::Key::N,
            Key::O => egui::Key::O,
            Key::P => egui::Key::P,
            Key::Q => egui::Key::Q,
            Key::R => egui::Key::R,
            Key::S => egui::Key::S,
            Key::T => egui::Key::T,
            Key::U => egui::Key::U,
            Key::V => egui::Key::V,
            Key::W => egui::Key::W,
            Key::X => egui::Key::X,
            Key::Y => egui::Key::Y,
            Key::Z => egui::Key::Z,
            Key::F1 => egui::Key::F1,
            Key::F2 => egui::Key::F2,
            Key::F3 => egui::Key::F3,
            Key::F4 => egui::Key::F4,
            Key::F5 => egui::Key::F5,
            Key::F6 => egui::Key::F6,
            Key::F7 => egui::Key::F7,
            Key::F8 => egui::Key::F8,
            Key::F9 => egui::Key::F9,
            Key::F10 => egui::Key::F10,
            Key::F11 => egui::Key::F11,
            Key::F12 => egui::Key::F12,
            Key::F13 => egui::Key::F13,
            Key::F14 => egui::Key::F14,
            Key::F15 => egui::Key::F15,
            Key::F16 => egui::Key::F16,
            Key::F17 => egui::Key::F17,
            Key::F18 => egui::Key::F18,
            Key::F19 => egui::Key::F19,
            Key::F20 => egui::Key::F20,
            Key::F21 => egui::Key::F21,
            Key::F22 => egui::Key::F22,
            Key::F23 => egui::Key::F23,
            Key::F24 => egui::Key::F24,
            Key::F25 => egui::Key::F25,
            Key::F26 => egui::Key::F26,
            Key::F27 => egui::Key::F27,
            Key::F28 => egui::Key::F28,
            Key::F29 => egui::Key::F29,
            Key::F30 => egui::Key::F30,
            Key::F31 => egui::Key::F31,
            Key::F32 => egui::Key::F32,
            Key::F33 => egui::Key::F33,
            Key::F34 => egui::Key::F34,
            Key::F35 => egui::Key::F35,
            _ => egui::Key::F35,
        }
    }
}
#[cfg(feature = "egui")]
impl From<&egui::Key> for Key {
    fn from(val: &egui::Key) -> Self {
        match val {
            egui::Key::ArrowDown => Key::ArrowDown,
            egui::Key::ArrowLeft => Key::ArrowLeft,
            egui::Key::ArrowRight => Key::ArrowRight,
            egui::Key::ArrowUp => Key::ArrowUp,
            egui::Key::Escape => Key::Escape,
            egui::Key::Tab => Key::Tab,
            egui::Key::Backspace => Key::Backspace,
            egui::Key::Enter => Key::Enter,
            egui::Key::Space => Key::Space,
            egui::Key::Insert => Key::Insert,
            egui::Key::Delete => Key::Delete,
            egui::Key::Home => Key::Home,
            egui::Key::End => Key::End,
            egui::Key::PageUp => Key::PageUp,
            egui::Key::PageDown => Key::PageDown,
            egui::Key::Copy => Key::Copy,
            egui::Key::Cut => Key::Cut,
            egui::Key::Paste => Key::Paste,
            egui::Key::Colon => Key::Colon,
            egui::Key::Comma => Key::Comma,
            egui::Key::Backslash => Key::Backslash,
            egui::Key::Slash => Key::Slash,
            egui::Key::Pipe => Key::Pipe,
            egui::Key::Questionmark => Key::Questionmark,
            egui::Key::Exclamationmark => Key::Exclamationmark,
            egui::Key::OpenBracket => Key::OpenBracket,
            egui::Key::CloseBracket => Key::CloseBracket,
            egui::Key::OpenCurlyBracket => Key::OpenCurlyBracket,
            egui::Key::CloseCurlyBracket => Key::CloseCurlyBracket,
            egui::Key::Backtick => Key::Backtick,
            egui::Key::Minus => Key::Minus,
            egui::Key::Period => Key::Period,
            egui::Key::Plus => Key::Plus,
            egui::Key::Equals => Key::Equals,
            egui::Key::Semicolon => Key::Semicolon,
            egui::Key::Quote => Key::Quote,
            egui::Key::Num0 => Key::Num0,
            egui::Key::Num1 => Key::Num1,
            egui::Key::Num2 => Key::Num2,
            egui::Key::Num3 => Key::Num3,
            egui::Key::Num4 => Key::Num4,
            egui::Key::Num5 => Key::Num5,
            egui::Key::Num6 => Key::Num6,
            egui::Key::Num7 => Key::Num7,
            egui::Key::Num8 => Key::Num8,
            egui::Key::Num9 => Key::Num9,
            egui::Key::A => Key::A,
            egui::Key::B => Key::B,
            egui::Key::C => Key::C,
            egui::Key::D => Key::D,
            egui::Key::E => Key::E,
            egui::Key::F => Key::F,
            egui::Key::G => Key::G,
            egui::Key::H => Key::H,
            egui::Key::I => Key::I,
            egui::Key::J => Key::J,
            egui::Key::K => Key::K,
            egui::Key::L => Key::L,
            egui::Key::M => Key::M,
            egui::Key::N => Key::N,
            egui::Key::O => Key::O,
            egui::Key::P => Key::P,
            egui::Key::Q => Key::Q,
            egui::Key::R => Key::R,
            egui::Key::S => Key::S,
            egui::Key::T => Key::T,
            egui::Key::U => Key::U,
            egui::Key::V => Key::V,
            egui::Key::W => Key::W,
            egui::Key::X => Key::X,
            egui::Key::Y => Key::Y,
            egui::Key::Z => Key::Z,
            egui::Key::F1 => Key::F1,
            egui::Key::F2 => Key::F2,
            egui::Key::F3 => Key::F3,
            egui::Key::F4 => Key::F4,
            egui::Key::F5 => Key::F5,
            egui::Key::F6 => Key::F6,
            egui::Key::F7 => Key::F7,
            egui::Key::F8 => Key::F8,
            egui::Key::F9 => Key::F9,
            egui::Key::F10 => Key::F10,
            egui::Key::F11 => Key::F11,
            egui::Key::F12 => Key::F12,
            egui::Key::F13 => Key::F13,
            egui::Key::F14 => Key::F14,
            egui::Key::F15 => Key::F15,
            egui::Key::F16 => Key::F16,
            egui::Key::F17 => Key::F17,
            egui::Key::F18 => Key::F18,
            egui::Key::F19 => Key::F19,
            egui::Key::F20 => Key::F20,
            egui::Key::F21 => Key::F21,
            egui::Key::F22 => Key::F22,
            egui::Key::F23 => Key::F23,
            egui::Key::F24 => Key::F24,
            egui::Key::F25 => Key::F25,
            egui::Key::F26 => Key::F26,
            egui::Key::F27 => Key::F27,
            egui::Key::F28 => Key::F28,
            egui::Key::F29 => Key::F29,
            egui::Key::F30 => Key::F30,
            egui::Key::F31 => Key::F31,
            egui::Key::F32 => Key::F32,
            egui::Key::F33 => Key::F33,
            egui::Key::F34 => Key::F34,
            egui::Key::F35 => Key::F35,
            egui::Key::BrowserBack => Key::F35,
        }
    }
}
#[cfg(feature = "winit")]
impl From<Key> for winit::keyboard::Key {
    fn from(val: Key) -> Self {
        match val {
            Key::ArrowDown => winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowDown),
            Key::ArrowLeft => winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowLeft),
            Key::ArrowRight => winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowRight),
            Key::ArrowUp => winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowUp),
            Key::Escape => winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
            Key::Tab => winit::keyboard::Key::Named(winit::keyboard::NamedKey::Tab),
            Key::Backspace => winit::keyboard::Key::Named(winit::keyboard::NamedKey::Backspace),
            Key::Enter => winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter),
            Key::Space => winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space),
            Key::Insert => winit::keyboard::Key::Named(winit::keyboard::NamedKey::Insert),
            Key::Delete => winit::keyboard::Key::Named(winit::keyboard::NamedKey::Delete),
            Key::Home => winit::keyboard::Key::Named(winit::keyboard::NamedKey::Home),
            Key::End => winit::keyboard::Key::Named(winit::keyboard::NamedKey::End),
            Key::PageUp => winit::keyboard::Key::Named(winit::keyboard::NamedKey::PageUp),
            Key::PageDown => winit::keyboard::Key::Named(winit::keyboard::NamedKey::PageDown),
            Key::Copy => winit::keyboard::Key::Named(winit::keyboard::NamedKey::Copy),
            Key::Cut => winit::keyboard::Key::Named(winit::keyboard::NamedKey::Cut),
            Key::Paste => winit::keyboard::Key::Named(winit::keyboard::NamedKey::Paste),
            Key::Colon => winit::keyboard::Key::Character(":".into()),
            Key::Comma => winit::keyboard::Key::Character(",".into()),
            Key::Backslash => winit::keyboard::Key::Character("\\".into()),
            Key::Slash => winit::keyboard::Key::Character("/".into()),
            Key::Pipe => winit::keyboard::Key::Character("|".into()),
            Key::Questionmark => winit::keyboard::Key::Character("?".into()),
            Key::Exclamationmark => winit::keyboard::Key::Character("!".into()),
            Key::OpenBracket => winit::keyboard::Key::Character("[".into()),
            Key::CloseBracket => winit::keyboard::Key::Character("]".into()),
            Key::OpenCurlyBracket => winit::keyboard::Key::Character("{".into()),
            Key::CloseCurlyBracket => winit::keyboard::Key::Character("}".into()),
            Key::Backtick => winit::keyboard::Key::Character("`".into()),
            Key::Minus => winit::keyboard::Key::Character("-".into()),
            Key::Period => winit::keyboard::Key::Character(".".into()),
            Key::Plus => winit::keyboard::Key::Character("+".into()),
            Key::Equals => winit::keyboard::Key::Character("=".into()),
            Key::Semicolon => winit::keyboard::Key::Character(";".into()),
            Key::Quote => winit::keyboard::Key::Character("\'".into()),
            Key::Num0 => winit::keyboard::Key::Character("0".into()),
            Key::Num1 => winit::keyboard::Key::Character("1".into()),
            Key::Num2 => winit::keyboard::Key::Character("2".into()),
            Key::Num3 => winit::keyboard::Key::Character("3".into()),
            Key::Num4 => winit::keyboard::Key::Character("4".into()),
            Key::Num5 => winit::keyboard::Key::Character("5".into()),
            Key::Num6 => winit::keyboard::Key::Character("6".into()),
            Key::Num7 => winit::keyboard::Key::Character("7".into()),
            Key::Num8 => winit::keyboard::Key::Character("8".into()),
            Key::Num9 => winit::keyboard::Key::Character("9".into()),
            Key::A => winit::keyboard::Key::Character("a".into()),
            Key::B => winit::keyboard::Key::Character("b".into()),
            Key::C => winit::keyboard::Key::Character("c".into()),
            Key::D => winit::keyboard::Key::Character("d".into()),
            Key::E => winit::keyboard::Key::Character("e".into()),
            Key::F => winit::keyboard::Key::Character("f".into()),
            Key::G => winit::keyboard::Key::Character("g".into()),
            Key::H => winit::keyboard::Key::Character("h".into()),
            Key::I => winit::keyboard::Key::Character("i".into()),
            Key::J => winit::keyboard::Key::Character("j".into()),
            Key::K => winit::keyboard::Key::Character("k".into()),
            Key::L => winit::keyboard::Key::Character("l".into()),
            Key::M => winit::keyboard::Key::Character("m".into()),
            Key::N => winit::keyboard::Key::Character("n".into()),
            Key::O => winit::keyboard::Key::Character("o".into()),
            Key::P => winit::keyboard::Key::Character("p".into()),
            Key::Q => winit::keyboard::Key::Character("q".into()),
            Key::R => winit::keyboard::Key::Character("r".into()),
            Key::S => winit::keyboard::Key::Character("s".into()),
            Key::T => winit::keyboard::Key::Character("t".into()),
            Key::U => winit::keyboard::Key::Character("u".into()),
            Key::V => winit::keyboard::Key::Character("v".into()),
            Key::W => winit::keyboard::Key::Character("w".into()),
            Key::X => winit::keyboard::Key::Character("x".into()),
            Key::Y => winit::keyboard::Key::Character("y".into()),
            Key::Z => winit::keyboard::Key::Character("z".into()),
            Key::Mult => winit::keyboard::Key::Character("*".into()),
            Key::Caret => winit::keyboard::Key::Character("^".into()),
            Key::HashTag => winit::keyboard::Key::Character("#".into()),
            Key::OpenParentheses => winit::keyboard::Key::Character("(".into()),
            Key::CloseParentheses => winit::keyboard::Key::Character(")".into()),
            Key::And => winit::keyboard::Key::Character("&".into()),
            Key::Percent => winit::keyboard::Key::Character("%".into()),
            Key::Underscore => winit::keyboard::Key::Character("_".into()),
            Key::LessThen => winit::keyboard::Key::Character("<".into()),
            Key::GreaterThen => winit::keyboard::Key::Character(">".into()),
            Key::PlusMinus => winit::keyboard::Key::Character("±".into()),
            Key::DoubleQuote => winit::keyboard::Key::Character("\"".into()),
            Key::Dollar => winit::keyboard::Key::Character("$".into()),
            Key::Cent => winit::keyboard::Key::Character("¢".into()),
            Key::Tilde => winit::keyboard::Key::Character("~".into()),
            Key::F1 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F1),
            Key::F2 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F2),
            Key::F3 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F3),
            Key::F4 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F4),
            Key::F5 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F5),
            Key::F6 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F6),
            Key::F7 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F7),
            Key::F8 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F8),
            Key::F9 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F9),
            Key::F10 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F10),
            Key::F11 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F11),
            Key::F12 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F12),
            Key::F13 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F13),
            Key::F14 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F14),
            Key::F15 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F15),
            Key::F16 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F16),
            Key::F17 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F17),
            Key::F18 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F18),
            Key::F19 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F19),
            Key::F20 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F20),
            Key::F21 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F21),
            Key::F22 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F22),
            Key::F23 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F23),
            Key::F24 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F24),
            Key::F25 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F25),
            Key::F26 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F26),
            Key::F27 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F27),
            Key::F28 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F28),
            Key::F29 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F29),
            Key::F30 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F30),
            Key::F31 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F31),
            Key::F32 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F32),
            Key::F33 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F33),
            Key::F34 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F34),
            Key::F35 => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F35),
            Key::Undefined => winit::keyboard::Key::Named(winit::keyboard::NamedKey::F35),
        }
    }
}
#[cfg(feature = "winit")]
impl From<winit::keyboard::Key> for Key {
    fn from(val: winit::keyboard::Key) -> Self {
        match val {
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowDown) => Key::ArrowDown,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowLeft) => Key::ArrowLeft,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowRight) => Key::ArrowRight,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowUp) => Key::ArrowUp,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) => Key::Escape,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Tab) => Key::Tab,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Backspace) => Key::Backspace,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter) => Key::Enter,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space) => Key::Space,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Insert) => Key::Insert,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Delete) => Key::Delete,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Home) => Key::Home,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::End) => Key::End,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::PageUp) => Key::PageUp,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::PageDown) => Key::PageDown,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Copy) => Key::Copy,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Cut) => Key::Cut,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Paste) => Key::Paste,
            winit::keyboard::Key::Character(val) => {
                match val.to_string().to_ascii_lowercase().as_str() {
                    ":" => Key::Colon,
                    "," => Key::Comma,
                    "\\" => Key::Backslash,
                    "/" => Key::Slash,
                    "|" => Key::Pipe,
                    "?" => Key::Questionmark,
                    "!" => Key::Exclamationmark,
                    "[" => Key::OpenBracket,
                    "]" => Key::CloseBracket,
                    "{" => Key::OpenCurlyBracket,
                    "}" => Key::CloseCurlyBracket,
                    "`" => Key::Backtick,
                    "-" => Key::Minus,
                    "." => Key::Period,
                    "+" => Key::Plus,
                    "=" => Key::Equals,
                    ";" => Key::Semicolon,
                    "\'" => Key::Quote,
                    "0" => Key::Num0,
                    "1" => Key::Num1,
                    "2" => Key::Num2,
                    "3" => Key::Num3,
                    "4" => Key::Num4,
                    "5" => Key::Num5,
                    "6" => Key::Num6,
                    "7" => Key::Num7,
                    "8" => Key::Num8,
                    "9" => Key::Num9,
                    "a" => Key::A,
                    "b" => Key::B,
                    "c" => Key::C,
                    "d" => Key::D,
                    "e" => Key::E,
                    "f" => Key::F,
                    "g" => Key::G,
                    "h" => Key::H,
                    "i" => Key::I,
                    "j" => Key::J,
                    "k" => Key::K,
                    "l" => Key::L,
                    "m" => Key::M,
                    "n" => Key::N,
                    "o" => Key::O,
                    "p" => Key::P,
                    "q" => Key::Q,
                    "r" => Key::R,
                    "s" => Key::S,
                    "t" => Key::T,
                    "u" => Key::U,
                    "v" => Key::V,
                    "w" => Key::W,
                    "x" => Key::X,
                    "y" => Key::Y,
                    "z" => Key::Z,
                    "^" => Key::Caret,
                    "#" => Key::HashTag,
                    "(" => Key::OpenParentheses,
                    ")" => Key::CloseParentheses,
                    "&" => Key::And,
                    "%" => Key::Percent,
                    "_" => Key::Underscore,
                    "<" => Key::LessThen,
                    ">" => Key::GreaterThen,
                    "±" => Key::PlusMinus,
                    "\"" => Key::DoubleQuote,
                    "$" => Key::Dollar,
                    "¢" => Key::Cent,
                    "~" => Key::Tilde,
                    "*" => Key::Mult,
                    _ => Key::Undefined,
                }
            }
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F1) => Key::F1,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F2) => Key::F2,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F3) => Key::F3,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F4) => Key::F4,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F5) => Key::F5,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F6) => Key::F6,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F7) => Key::F7,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F8) => Key::F8,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F9) => Key::F9,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F10) => Key::F10,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F11) => Key::F11,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F12) => Key::F12,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F13) => Key::F13,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F14) => Key::F14,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F15) => Key::F15,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F16) => Key::F16,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F17) => Key::F17,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F18) => Key::F18,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F19) => Key::F19,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F20) => Key::F20,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F21) => Key::F21,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F22) => Key::F22,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F23) => Key::F23,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F24) => Key::F24,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F25) => Key::F25,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F26) => Key::F26,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F27) => Key::F27,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F28) => Key::F28,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F29) => Key::F29,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F30) => Key::F30,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F31) => Key::F31,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F32) => Key::F32,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F33) => Key::F33,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F34) => Key::F34,
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::F35) => Key::F35,
            _ => Key::Undefined,
        }
    }
}
pub enum NamedKey {
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    Escape,
    Tab,
    Backspace,
    Enter,
    Space,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Copy,
    Cut,
    Paste,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    F25,
    F26,
    F27,
    F28,
    F29,
    F30,
    F31,
    F32,
    F33,
    F34,
    F35,
}
pub enum KeyStr {
    Named(NamedKey),
    Character(char),
}
impl From<&Key> for KeyStr {
    fn from(key: &Key) -> Self {
        match key {
            Key::ArrowDown => KeyStr::Named(NamedKey::ArrowDown),
            Key::ArrowLeft => KeyStr::Named(NamedKey::ArrowLeft),
            Key::ArrowRight => KeyStr::Named(NamedKey::ArrowRight),
            Key::ArrowUp => KeyStr::Named(NamedKey::ArrowUp),
            Key::Escape => KeyStr::Named(NamedKey::Escape),
            Key::Tab => KeyStr::Named(NamedKey::Tab),
            Key::Backspace => KeyStr::Named(NamedKey::Backspace),
            Key::Enter => KeyStr::Named(NamedKey::Enter),
            Key::Space => KeyStr::Named(NamedKey::Space),
            Key::Insert => KeyStr::Named(NamedKey::Insert),
            Key::Delete => KeyStr::Named(NamedKey::Delete),
            Key::Home => KeyStr::Named(NamedKey::Home),
            Key::End => KeyStr::Named(NamedKey::End),
            Key::PageUp => KeyStr::Named(NamedKey::PageUp),
            Key::PageDown => KeyStr::Named(NamedKey::PageDown),
            Key::Copy => KeyStr::Named(NamedKey::Copy),
            Key::Cut => KeyStr::Named(NamedKey::Cut),
            Key::Paste => KeyStr::Named(NamedKey::Paste),
            Key::F1 => KeyStr::Named(NamedKey::F1),
            Key::F2 => KeyStr::Named(NamedKey::F2),
            Key::F3 => KeyStr::Named(NamedKey::F3),
            Key::F4 => KeyStr::Named(NamedKey::F4),
            Key::F5 => KeyStr::Named(NamedKey::F5),
            Key::F6 => KeyStr::Named(NamedKey::F6),
            Key::F7 => KeyStr::Named(NamedKey::F7),
            Key::F8 => KeyStr::Named(NamedKey::F8),
            Key::F9 => KeyStr::Named(NamedKey::F9),
            Key::F10 => KeyStr::Named(NamedKey::F10),
            Key::F11 => KeyStr::Named(NamedKey::F11),
            Key::F12 => KeyStr::Named(NamedKey::F12),
            Key::F13 => KeyStr::Named(NamedKey::F13),
            Key::F14 => KeyStr::Named(NamedKey::F14),
            Key::F15 => KeyStr::Named(NamedKey::F15),
            Key::F16 => KeyStr::Named(NamedKey::F16),
            Key::F17 => KeyStr::Named(NamedKey::F17),
            Key::F18 => KeyStr::Named(NamedKey::F18),
            Key::F19 => KeyStr::Named(NamedKey::F19),
            Key::F20 => KeyStr::Named(NamedKey::F20),
            Key::F21 => KeyStr::Named(NamedKey::F21),
            Key::F22 => KeyStr::Named(NamedKey::F22),
            Key::F23 => KeyStr::Named(NamedKey::F23),
            Key::F24 => KeyStr::Named(NamedKey::F24),
            Key::F25 => KeyStr::Named(NamedKey::F25),
            Key::F26 => KeyStr::Named(NamedKey::F26),
            Key::F27 => KeyStr::Named(NamedKey::F27),
            Key::F28 => KeyStr::Named(NamedKey::F28),
            Key::F29 => KeyStr::Named(NamedKey::F29),
            Key::F30 => KeyStr::Named(NamedKey::F30),
            Key::F31 => KeyStr::Named(NamedKey::F31),
            Key::F32 => KeyStr::Named(NamedKey::F32),
            Key::F33 => KeyStr::Named(NamedKey::F33),
            Key::F34 => KeyStr::Named(NamedKey::F34),
            Key::F35 => KeyStr::Named(NamedKey::F35),
            Key::Colon => KeyStr::Character(':'),
            Key::Comma => KeyStr::Character(','),
            Key::Backslash => KeyStr::Character('\\'),
            Key::Slash => KeyStr::Character('/'),
            Key::Pipe => KeyStr::Character('|'),
            Key::Questionmark => KeyStr::Character('?'),
            Key::Exclamationmark => KeyStr::Character('!'),
            Key::OpenBracket => KeyStr::Character('['),
            Key::CloseBracket => KeyStr::Character(']'),
            Key::OpenCurlyBracket => KeyStr::Character('{'),
            Key::CloseCurlyBracket => KeyStr::Character('}'),
            Key::Backtick => KeyStr::Character('`'),
            Key::Minus => KeyStr::Character('-'),
            Key::Period => KeyStr::Character('.'),
            Key::Plus => KeyStr::Character('+'),
            Key::Equals => KeyStr::Character('='),
            Key::Semicolon => KeyStr::Character(';'),
            Key::Quote => KeyStr::Character('\''),
            Key::Num0 => KeyStr::Character('0'),
            Key::Num1 => KeyStr::Character('1'),
            Key::Num2 => KeyStr::Character('2'),
            Key::Num3 => KeyStr::Character('3'),
            Key::Num4 => KeyStr::Character('4'),
            Key::Num5 => KeyStr::Character('5'),
            Key::Num6 => KeyStr::Character('6'),
            Key::Num7 => KeyStr::Character('7'),
            Key::Num8 => KeyStr::Character('8'),
            Key::Num9 => KeyStr::Character('9'),
            Key::A => KeyStr::Character('a'),
            Key::B => KeyStr::Character('b'),
            Key::C => KeyStr::Character('c'),
            Key::D => KeyStr::Character('d'),
            Key::E => KeyStr::Character('e'),
            Key::F => KeyStr::Character('f'),
            Key::G => KeyStr::Character('g'),
            Key::H => KeyStr::Character('h'),
            Key::I => KeyStr::Character('i'),
            Key::J => KeyStr::Character('j'),
            Key::K => KeyStr::Character('k'),
            Key::L => KeyStr::Character('l'),
            Key::M => KeyStr::Character('m'),
            Key::N => KeyStr::Character('n'),
            Key::O => KeyStr::Character('o'),
            Key::P => KeyStr::Character('p'),
            Key::Q => KeyStr::Character('q'),
            Key::R => KeyStr::Character('r'),
            Key::S => KeyStr::Character('s'),
            Key::T => KeyStr::Character('t'),
            Key::U => KeyStr::Character('u'),
            Key::V => KeyStr::Character('v'),
            Key::W => KeyStr::Character('w'),
            Key::X => KeyStr::Character('x'),
            Key::Y => KeyStr::Character('y'),
            Key::Z => KeyStr::Character('z'),
            Key::Mult => KeyStr::Character('*'),
            Key::Caret => KeyStr::Character('^'),
            Key::HashTag => KeyStr::Character('#'),
            Key::OpenParentheses => KeyStr::Character('('),
            Key::CloseParentheses => KeyStr::Character(')'),
            Key::And => KeyStr::Character('&'),
            Key::Percent => KeyStr::Character('%'),
            Key::Underscore => KeyStr::Character('_'),
            Key::LessThen => KeyStr::Character('<'),
            Key::GreaterThen => KeyStr::Character('>'),
            Key::PlusMinus => KeyStr::Character('±'),
            Key::DoubleQuote => KeyStr::Character('\''),
            Key::Dollar => KeyStr::Character('$'),
            Key::Cent => KeyStr::Character('¢'),
            Key::Tilde => KeyStr::Character('~'),
            Key::Undefined => KeyStr::Named(NamedKey::F35),
        }
    }
}
