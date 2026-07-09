use std::fs;

fn main() {
    // 读取缓存封面
    let cache_path = "target/debug/covers/v2_beebf95a478a4524.jpg";
    if let Ok(data) = fs::read(cache_path) {
        if let Ok(img) = image::load_from_memory(&data) {
            println!("Cache: {}x{} {:?}", img.width(), img.height(), img.color());
            let rgb = img.to_rgba8();
            // 采样几个点（使用 get_pixel）
            let w = rgb.width();
            let h = rgb.height();
            let samples = [(w/4, h/4), (w/2, h/2), (3*w/4, 3*h/4)];
            for &(x, y) in &samples {
                let pixel = rgb.get_pixel(x, y);
                println!("  ({},{}): RGBA({},{},{},{})", x, y,
                    pixel[0], pixel[1], pixel[2], pixel[3]);
            }
        }
    }
}
