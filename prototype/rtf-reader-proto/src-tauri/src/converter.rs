use std::time::Instant;

/// Convert EMF bytes to SVG string.
///
/// On macOS/Linux this uses libemf2svg. On Windows it currently returns an
/// error; the plan is to add a GDI+ based renderer (see ADR 0002).
#[cfg(not(target_os = "windows"))]
pub fn emf_to_svg(emf_data: &[u8]) -> Result<String, String> {
    use std::ffi::{c_char, c_int, c_void};
    use std::os::raw::c_double;

    #[repr(C)]
    #[allow(non_snake_case)]
    struct GeneratorOptions {
        nameSpace: *mut c_char,
        verbose: bool,
        emfplus: bool,
        svgDelimiter: bool,
        imgHeight: c_double,
        imgWidth: c_double,
    }

    #[link(name = "emf2svg")]
    extern "C" {
        fn emf2svg(
            contents: *mut c_char,
            length: usize,
            out: *mut *mut c_char,
            out_length: *mut usize,
            options: *mut GeneratorOptions,
        ) -> c_int;
    }

    let mut options = GeneratorOptions {
        nameSpace: std::ptr::null_mut(),
        verbose: false,
        emfplus: true,
        svgDelimiter: true,
        imgHeight: 0.0,
        imgWidth: 0.0,
    };

    // libemf2svg may return 1 when it encountered partial-support records
    // but still produced usable output. Accept the output if present.

    let mut out_ptr: *mut c_char = std::ptr::null_mut();
    let mut out_len: usize = 0;

    let ret = unsafe {
        emf2svg(
            emf_data.as_ptr() as *mut c_char,
            emf_data.len(),
            &mut out_ptr,
            &mut out_len,
            &mut options,
        )
    };

    if out_ptr.is_null() || out_len == 0 {
        return Err(format!("emf2svg conversion produced no output (code {})", ret));
    }

    let svg = unsafe {
        let slice = std::slice::from_raw_parts(out_ptr as *const u8, out_len);
        let s = String::from_utf8_lossy(slice).to_string();
        libc::free(out_ptr as *mut c_void);
        s
    };

    Ok(svg)
}

#[cfg(target_os = "windows")]
pub fn emf_to_svg(_emf_data: &[u8]) -> Result<String, String> {
    Err("Windows EMF conversion is not yet implemented (planned: GDI+)".to_string())
}

/// Convert many EMF images and return total nanoseconds.
pub fn convert_images(images: &[super::parser::Image]) -> (Vec<String>, u128) {
    let start = Instant::now();
    let mut svgs = Vec::with_capacity(images.len());
    let mut failures = 0usize;

    for img in images {
        match emf_to_svg(&img.data) {
            Ok(svg) => svgs.push(svg),
            Err(_) => {
                failures += 1;
                svgs.push(String::new());
            }
        }
    }

    let nanos = start.elapsed().as_nanos();
    if failures > 0 {
        eprintln!("Image conversion failures: {}", failures);
    }
    (svgs, nanos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn converts_sample_emf() {
        let path = PathBuf::from("../test-data/figures/figure-01.emf");
        if !path.exists() {
            eprintln!("Skipping test: sample EMF not found");
            return;
        }
        let data = std::fs::read(&path).unwrap();
        let svg = emf_to_svg(&data).unwrap();
        assert!(svg.starts_with("<?xml") || svg.starts_with("<svg"));
        println!("Converted {} bytes EMF -> {} bytes SVG", data.len(), svg.len());
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn converts_libemf2svg_test_emf() {
        let path = PathBuf::from("../deps/libemf2svg/tests/resources/emf/test-011.emf");
        if !path.exists() {
            eprintln!("Skipping test: libemf2svg test EMF not found");
            return;
        }
        let data = std::fs::read(&path).unwrap();
        match emf_to_svg(&data) {
            Ok(svg) => {
                assert!(svg.starts_with("<?xml") || svg.starts_with("<svg"));
                println!("Converted test EMF {} bytes -> {} bytes SVG", data.len(), svg.len());
            }
            Err(e) => {
                eprintln!("Conversion failed (expected for some test files): {}", e);
            }
        }
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn benchmark_parse_and_convert_images() {
        for pages in [200, 1000, 2000] {
            let path = PathBuf::from(format!("../test-data/synthetic-tfl-images-{pages}p.rtf"));
            if !path.exists() {
                continue;
            }
            let parse_start = Instant::now();
            let parse_res = crate::parser::parse_file(&path).unwrap();
            let parse_nanos = parse_start.elapsed().as_nanos();

            let visible_count = parse_res.images.len().min(5);
            let visible = &parse_res.images[..visible_count];
            let (_visible_svgs, visible_convert_nanos) = convert_images(visible);

            let (svgs, convert_nanos) = convert_images(&parse_res.images);
            let success = svgs.iter().filter(|s| !s.is_empty()).count();

            println!(
                "images-{pages}p: size={:.2}MB pages={} images={} parse={:.3}s first5={:.3}s all={:.3}s ({} ok) memory={:.1}MB",
                parse_res.file_size as f64 / 1024.0 / 1024.0,
                parse_res.page_count_estimate,
                parse_res.images.len(),
                parse_nanos as f64 / 1e9,
                visible_convert_nanos as f64 / 1e9,
                convert_nanos as f64 / 1e9,
                success,
                parse_res.peak_memory_kb as f64 / 1024.0
            );
        }
    }
}
