//! Proof: parse a real LDT, paint through the SAME anyrender Scene API a
//! Blitz `Widget::paint` uses, rasterize with vello_cpu, write a PNG.
//! If this picture looks right, the Blitz widget renders right.

use anyrender::ImageRenderer as _;
use anyrender_vello_cpu::VelloCpuImageRenderer;
use eulumdat::diagram::PolarDiagram;
use eulumdat::Eulumdat;
use photometric_widgets::polar::{paint_polar_styled, PolarStyle};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).map(String::as_str).unwrap_or("assets/sample.ldt");
    let out = args.get(2).map(String::as_str).unwrap_or("polar.png");

    let ldt = Eulumdat::from_file(path)?;
    println!(
        "parsed: {} | {} (symmetry: {:?})",
        ldt.identification.trim(), ldt.luminaire_name, ldt.symmetry
    );
    let diagram = PolarDiagram::from_eulumdat(&ldt);
    println!(
        "scale_max = {} cd/klm, C0-C180 pts = {}, C90-C270 pts = {}",
        diagram.scale.scale_max,
        diagram.c0_c180_curve.points.len(),
        diagram.c90_c270_curve.points.len()
    );

    const W: u32 = 640;
    const H: u32 = 640;
    let mut renderer = VelloCpuImageRenderer::new(W, H);
    let mut buf: Vec<u8> = vec![0u8; (W * H * 4) as usize];
    renderer.render(
        |scene| {
            // white bg for the PNG (in Blitz, CSS provides the background)
            let mut style = PolarStyle::default();
            style.background = Some(peniko::Color::WHITE);
            paint_polar_styled(scene, &diagram, W as f64, H as f64, 1.0, &style);
        },
        &mut buf,
    );

    // buf is RGBA8; encode PNG
    let file = std::fs::File::create(out)?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), W, H);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(&buf)?;
    println!("wrote {out}");
    Ok(())
}
