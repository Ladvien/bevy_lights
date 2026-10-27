fn main() {
    for seed in [0u32, 7] {
        let row: Vec<f32> = (0..64).map(|i| bevy_lights::char_field(i as f32 / 63.0, 0.37, seed)).collect();
        let bright = row.iter().filter(|v| **v > 0.5).count();
        let dark = row.iter().filter(|v| **v < 0.25).count();
        println!("seed {seed}: bright {bright} dark {dark} min {:.3} max {:.3}",
            row.iter().cloned().fold(f32::MAX, f32::min), row.iter().cloned().fold(f32::MIN, f32::max));
        let all: Vec<f32> = (0..64).flat_map(|i| (0..64).map(move |j| (i, j)))
            .map(|(i,j)| bevy_lights::char_field(i as f32/64.0, j as f32/64.0, seed)).collect();
        println!("   grid mean {:.3} frac<.25 {:.3} frac>.5 {:.3}",
            all.iter().sum::<f32>()/all.len() as f32,
            all.iter().filter(|v| **v < 0.25).count() as f32/all.len() as f32,
            all.iter().filter(|v| **v > 0.5).count() as f32/all.len() as f32);
    }
}
