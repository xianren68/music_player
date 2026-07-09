use std::fs;

fn main() {
    // 检查 PNG 缓存的颜色
    let cache_path = "target/debug/covers/v2_c0f46d6d0f619fd0.png";
    if let Ok(data) = fs::read(cache_path) {
        if let Ok(img) = image::load_from_memory(&data) {
            println!("PNG Cache: {}x{} {:?}", img.width(), img.height(), img.color());
            let rgba = img.to_rgba8();
            let w = rgba.width();
            let h = rgba.height();
            let samples = [(w/4, h/4), (w/2, h/2), (3*w/4, 3*h/4)];
            for &(x, y) in &samples {
                let pixel = rgba.get_pixel(x, y);
                println!("  ({},{}): RGBA({},{},{},{})", x, y,
                    pixel[0], pixel[1], pixel[2], pixel[3]);
            }
        }
    }
}
