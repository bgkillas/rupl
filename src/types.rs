use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};
#[derive(PartialEq)]
pub enum GraphMode {
    Normal,
    Slice,
    SliceFlatten,
    SliceDepth,
    DomainColoring,
    Flatten,
    Depth,
}
pub enum GraphType {
    Width(Vec<Complex>, f64, f64),
    Coord(Vec<(f64, Complex)>),
    Width3D(Vec<Complex>, f64, f64, f64, f64),
    Coord3D(Vec<(f64, f64, Complex)>),
}
#[derive(Copy, Clone)]
pub enum Draw {
    Line(Pos, Pos, f32),
    Point(Pos),
}
pub enum Prec {
    Mult(f64),
    Slice(f64, bool, isize),
    Dimension(usize, usize),
}
pub enum UpdateResult {
    Width(f64, f64, Prec),
    Width3D(f64, f64, f64, f64, Prec),
    None,
}
pub enum Show {
    Real,
    Imag,
    Complex,
}
impl Show {
    pub fn real(&self) -> bool {
        matches!(self, Self::Complex | Self::Real)
    }
    pub fn imag(&self) -> bool {
        matches!(self, Self::Complex | Self::Imag)
    }
}
pub enum Lines {
    Points,
    LinesPoints,
    Lines,
}
pub enum DepthColor {
    Vertical,
    Depth,
    None,
}
pub struct Graph {
    pub data: Vec<GraphType>,
    #[cfg(feature = "egui")]
    pub cache: Option<egui::TextureHandle>,
    pub bound: Vec2,
    pub is_complex: bool,
    pub offset3d: Vec3,
    pub offset: Vec2,
    pub angle: Vec2,
    pub ignore_bounds: bool,
    pub zoom: f64,
    pub slice: isize,
    pub switch: bool,
    pub var: Vec2,
    pub log_scale: bool,
    pub box_size: f64,
    pub domain_alternate: bool,
    pub screen: Vec2,
    pub screen_offset: Vec2,
    pub delta: f64,
    pub show: Show,
    pub anti_alias: bool,
    pub color_depth: DepthColor,
    pub show_box: bool,
    pub zoom3d: f64,
    pub main_colors: Vec<Color>,
    pub alt_colors: Vec<Color>,
    pub axis_color: Color,
    pub axis_color_light: Color,
    pub background_color: Color,
    pub text_color: Color,
    pub mouse_position: Option<Pos>,
    pub mouse_moved: bool,
    pub scale_axis: bool,
    pub disable_lines: bool,
    pub disable_axis: bool,
    pub disable_coord: bool,
    pub view_x: bool,
    pub graph_mode: GraphMode,
    pub is_3d: bool,
    pub last_interact: Option<Pos>,
    pub recalculate: bool,
    pub lines: Lines,
    pub ruler_pos: Option<Vec2>,
    pub prec: f64,
    pub mouse_held: bool,
    pub buffer: Vec<(f32, Draw, Color)>,
    pub mult: f64,
}
#[derive(Copy, Clone)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
impl Color {
    pub fn new(r: u8, g:u8,b:u8)-> Self {
    Self {r,g,b}
    }
    pub fn splat(c: u8)-> Self {
    Self {r:c,g:c,b:c}
    }
    pub fn to_col(&self)->egui::Color32 {
        egui::Color32::from_rgb(self.r,self.g,self.b)
    }
}
#[derive(Copy, Clone,PartialEq)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}
impl Pos {
    pub fn new(x: f32, y:f32)-> Self{
        Self {
            x,y
        }
    }
    pub fn to_pos2(&self) -> egui::Pos2 {
        egui::Pos2 {
            x: self.x,
            y: self.y,
        }
    }
}
#[derive(Copy, Clone)]
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
    pub fn from(y: Option<f64>, z: Option<f64>) -> Self {
        match (y, z) {
            (Some(y), Some(z)) => Self::Complex(y, z),
            (Some(y), None) => Self::Real(y),
            (None, Some(z)) => Self::Imag(z),
            (None, None) => Self::Complex(f64::NAN, f64::NAN),
        }
    }
}
#[derive(Copy, Clone, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}
impl Vec2 {
    pub fn splat(v: f64) -> Self {
        Self { x: v, y: v }
    }
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn to_pos(&self) -> Pos {
        Pos {
            x: self.x as f32,
            y: self.y as f32,
        }
    }
    pub fn to_pos2(&self) -> egui::Pos2 {
        egui::Pos2 {
            x: self.x as f32,
            y: self.y as f32,
        }
    }
}
impl DivAssign<f64> for Vec2 {
    fn div_assign(&mut self, rhs: f64) {
        self.x /= rhs;
        self.y /= rhs;
    }
}
impl MulAssign<f64> for Vec2 {
    fn mul_assign(&mut self, rhs: f64) {
        self.x *= rhs;
        self.y *= rhs;
    }
}
#[derive(Copy, Clone)]
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
        Pos::new(self.x *rhs, self.y *rhs)
    }
}
impl Div<f32> for Pos {
    type Output = Pos;
    fn div(self, rhs: f32) -> Self::Output {
        Pos::new(self.x /rhs, self.y /rhs)
    }
}