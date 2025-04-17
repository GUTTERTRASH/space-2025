use bevy::prelude::*;
use rand::Rng;

pub fn generate_targets(len: usize) -> Box<[(Vec3, Color, String)]> {
    let mut rng = rand::thread_rng();

    let mut vec = Vec::with_capacity(len);
    for i in 0..len {
        let position = Vec3::new(
            rng.gen_range(-20.0..20.0),
            rng.gen_range(-20.0..20.0),
            rng.gen_range(-20.0..-1.0),
        );
        let color = Color::srgb_u8(rng.r#gen(), rng.r#gen(), rng.r#gen());
        let name = format!("Target #{}", i);

        vec.push((position, color, name));
    }

    return vec.into_boxed_slice();
}
