use rupl::types::*;
use std::io::Write;
const WIDTH: usize = 1080;
fn main() -> Result<(), std::io::Error> {
    let (start, end) = (-0.5, 0.5);
    let pts = points(start, end);
    let graph = GraphData::Width3D(pts, start, start, end, end);
    let name = Name::new("x+y".to_string());
    let mut plot = Graph::new(vec![graph], vec![name], true, start, end);
    let mut stdin = std::io::stdout().lock();
    stdin.write_all(plot.get_png(WIDTH as u32, WIDTH as u32).as_bytes())?;
    stdin.flush()?;
    Ok(())
}
fn points(start: f64, end: f64) -> Vec<Complex> {
    let count = WIDTH / 16;
    let delta = (end - start) / count as f64;
    (0..count)
        .flat_map(|i| {
            let y = start + i as f64 * delta;
            (0..count).map(move |j| {
                let x = start + j as f64 * delta;
                Complex::Real(f(x, y))
            })
        })
        .collect()
}
pub fn f(x: f64, y: f64) -> f64 {
    x + y
}
