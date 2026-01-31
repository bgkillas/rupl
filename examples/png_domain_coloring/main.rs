use rupl::types::*;
use std::io::Write;
const WIDTH: usize = 4096;
fn main() -> Result<(), std::io::Error> {
    let (start, end) = (-0.5, 0.5);
    let pts = points(start, end);
    let graph = GraphData::Width3D(pts, start, start, end, end);
    let name = Name::new("sin(1/z)".to_string());
    let mut plot = Graph::new(vec![graph], vec![name], true, start, end);
    plot.set_mode(GraphMode::DomainColoring);
    plot.disable_axis = true;
    plot.disable_lines = true;
    plot.mult = 1.0;
    plot.anti_alias = false;
    let mut stdin = std::io::stdout().lock();
    stdin.write_all(plot.get_png(WIDTH as u32, WIDTH as u32).as_bytes())?;
    stdin.flush()?;
    Ok(())
}
fn points(start: f64, end: f64) -> Vec<Complex> {
    let delta = (end - start) / WIDTH as f64;
    (0..WIDTH)
        .flat_map(|i| {
            let y = start + i as f64 * delta;
            (0..WIDTH).map(move |j| {
                let x = start + j as f64 * delta;
                Complex::from(f(x, y))
            })
        })
        .collect()
}
pub fn f(x: f64, y: f64) -> (f64, f64) {
    let (x, y) = recip(x, y);
    sin(x, y)
}
pub fn recip(x: f64, y: f64) -> (f64, f64) {
    let r = x * x + y * y;
    (x / r, -y / r)
}
pub fn sin(x: f64, y: f64) -> (f64, f64) {
    let (a, b) = x.sin_cos();
    let (c, d) = (y.sinh(), y.cosh());
    (a * d, b * c)
}
