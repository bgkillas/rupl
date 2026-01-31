use rupl::types::*;
use std::io::Write;
fn main() -> Result<(), std::io::Error> {
    let (start, end) = (-2.0, 2.0);
    let (width, height) = (1920, 1080);
    let pts = points(start, end);
    let graph = GraphData::Width(pts, start, end);
    let name = Name::new("x".to_string());
    let mut plot = Graph::new(vec![graph], vec![name], false, start, end);
    let mut stdin = std::io::stdout().lock();
    stdin.write_all(plot.get_png(width, height).as_bytes())?;
    stdin.flush()?;
    Ok(())
}
fn points(start: f64, end: f64) -> Vec<Complex> {
    let len = 256;
    let delta = (end - start) / len as f64;
    (0..=len)
        .map(|i| {
            let x = start + i as f64 * delta;
            Complex::Real(f(x))
        })
        .collect()
}
fn f(x: f64) -> f64 {
    x * x * x - x
}
