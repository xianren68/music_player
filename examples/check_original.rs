use std::fs;

fn main() {
    // 直接读取岸边客的 FLAC 文件 - 保存原始 PNG 以便对比
    let path = r"C:\Kugou\王玉玺 - 岸边客.flac";
    println!("Opening: {}", path);

    use lofty::probe::Probe;
    use lofty::file::TaggedFileExt;
    if let Ok(f) = Probe::open(path).unwrap().guess_file_type().unwrap().read() {
        if let Some(tag) = f.first_tag() {
            for pic in tag.pictures() {
                println!("Picture: {:?}, size: {}", pic.pic_type(), pic.data().len());

                // 加载并保存为 PNG
                if let Ok(img) = image::load_from_memory(pic.data()) {
                    println!("Original: {}x{} {:?}", img.width(), img.height(), img.color());

                    // 保存原始封面为 PNG（无损）
                    let _ = img.save("target/debug/original_cover.png");
                    println!("Saved original to target/debug/original_cover.png");

                    // 也保存缓存版本为 PNG
                    let cache_path = r"C:\project\gpui-demo\target\debug\covers\v2_c0f46d6d0f619fd0.jpg";
                    if let Ok(cache_data) = fs::read(cache_path) {
                        if let Ok(cache_img) = image::load_from_memory(&cache_data) {
                            let _ = cache_img.save("target/debug/cache_cover.png");
                            println!("Saved cache to target/debug/cache_cover.png");
                        }
                    }
                }
                break;
            }
        }
    }

    // 检查 JPEG 文件的 markers（查找 ICC profile）
    println!("\n--- JPEG analysis ---");
    let cache_path = r"C:\project\gpui-demo\target\debug\covers\v2_c0f46d6d0f619fd0.jpg";
    if let Ok(data) = fs::read(cache_path) {
        println!("JPEG size: {} bytes", data.len());
        // 查找 APP2 marker (FFEC) = ICC profile
        for i in 0..data.len().saturating_sub(1) {
            if data[i] == 0xFF && data[i+1] == 0xEC {
                println!("Found ICC profile marker at offset {}", i);
                break;
            }
            if data[i] == 0xFF && data[i+1] == 0xE0 {
                println!("Found JFIF (APP0) at offset {}", i);
            }
        }
    }

    // 检查原始 FLAC 中封面的格式
    println!("\n--- Original cover format ---");
    if let Ok(f) = Probe::open(path).unwrap().guess_file_type().unwrap().read() {
        if let Some(tag) = f.first_tag() {
            for pic in tag.pictures() {
                let data = pic.data();
                println!("Cover data size: {} bytes", data.len());
                println!("First 20 bytes: {:02X?}", &data[..20.min(data.len())]);

                // 检测格式：PNG (89 50 4E 47), JPEG (FF D8 FF)
                if data.len() >= 4 {
                    match &data[..4] {
                        [0x89, 0x50, 0x4E, 0x47] => println!("Format: PNG"),
                        [0xFF, 0xD8, 0xFF, ..] => println!("Format: JPEG"),
                        _ => println!("Format: Unknown"),
                    }
                }
                break;
            }
        }
    }
}
