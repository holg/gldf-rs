use std::fs::File;
use gldf_rs::hyg::HygCatalogue;

fn main() {
    let file = File::open("data/hygdata_v42.csv").expect("HYG data not found - run from project root");
    let catalogue = HygCatalogue::from_csv(file).expect("Failed to parse");
    
    println!("=== HYG Star Catalogue ===");
    println!("Total stars: {}", catalogue.stars.len());
    println!("Naked eye (mag<6.5): {}", catalogue.naked_eye_stars().count());
    println!("With proper names: {}", catalogue.named_stars().count());
    
    println!("\n=== Brightest 15 stars ===");
    for star in catalogue.brightest(15) {
        let [r, g, b] = star.color_rgb();
        println!("{:12} mag={:6.2} T={:5.0}K RGB=({:.2},{:.2},{:.2}) {:8}", 
            star.display_name(), star.mag, star.temperature(), r, g, b, star.spect);
    }
    
    println!("\n=== Sirius ===");
    if let Some(s) = catalogue.find_by_name("Sirius") {
        println!("RA={:.4}° Dec={:.4}° dist={:.1}pc", s.ra, s.dec, s.dist);
    }
}
