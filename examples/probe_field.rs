fn main() {
    for seed in [0u32, 7, 1234] {
        let vals: Vec<f32> = (0..64).map(|i| bevy_lights::char_field(i as f32 / 63.0, 0.37, seed)).collect();
        let mean: f32 = vals.iter().sum::<f32>() / 64.0;
        println!("seed {seed} mean {mean:.3}");
        println!("  {}", vals.iter().map(|v| format!("{v:.2}")).collect::<Vec<_>>().join(" "));
        let all: Vec<f32> = (0..64).flat_map(|i| (0..64).map(move |j| (i, j))).map(|(i, j)| bevy_lights::char_field(i as f32/64.0, j as f32/64.0, seed)).collect();
        let m: f32 = all.iter().sum::<f32>() / all.len() as f32;
        println!("  grid mean {m:.3} max {:.3} min {:.3} frac<.25 {:.3} frac>.75 {:.3}",
            all.iter().cloned().fold(f32::MIN, f32::max),
            all.iter().cloned().fold(f32::MAX, f32::min),
            all.iter().filter(|v| **v < 0.25).count() as f32 / all.len() as f32,
            all.iter().filter(|v| **v > 0.75).count() as f32 / all.len() as f32);
    }
}
