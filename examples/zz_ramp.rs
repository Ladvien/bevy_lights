use bevy_lights::model::*;
fn main() {
    let t = bed_ramp();
    for row in [23usize, 22, 20, 18, 16, 14, 12, 10, 8, 4, 0] {
        let h = row as f32 / 23.0;
        let mut s = format!("h={h:.2}");
        for col in [0usize, 5, 9, 13, 18, 24, 31] {
            let e = t[row * 32 + col];
            s += &format!("  f{:.2}=({:.4},{:.4},{:.4})", col as f32 / 31.0, e[0], e[1], e[2]);
        }
        println!("{s}");
    }
    println!("visible heat {}", bed_visible_heat());
}
