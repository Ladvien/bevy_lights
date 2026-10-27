fn main() {
    let exp = 2f32.powi(-6) / 1.2;
    for t in [950.0f32, 1000.0, 1050.0, 1100.0, 1150.0, 1200.0, 1250.0, 1300.0] {
        let c = bevy_lights::blackbody_colour(t, 1.0);
        let lum = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        println!("T={t:6.0} colour={c:?} lum={lum:.3}");
    }
    let _ = exp;
}
