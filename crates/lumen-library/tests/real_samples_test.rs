use lumen_library::model::UnitRef;
use lumen_library::parse::*;
use std::fs;
use std::path::Path;

#[test]
#[ignore]
fn test_all_saved_real_research_samples() {
    let base_dir =
        Path::new("/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/research/anirdesh");
    if !base_dir.exists() {
        eprintln!("Research directory not found, skipping real sample checks");
        return;
    }

    // 1. Vachanamrut GU & EN
    let vach_gu_path = base_dir.join("vachanamrut_gu_1.html");
    if vach_gu_path.exists() {
        let content = fs::read_to_string(&vach_gu_path).expect("read vachanamrut_gu_1");
        let unit_ref = UnitRef::new("vachanamrut", "1");
        let unit = parse_vachanamrut_page(&content, "gu", &unit_ref).expect("parse vachanamrut gu");
        assert!(!unit.blocks.is_empty(), "vachanamrut gu blocks empty");
    }

    let vach_en_path = base_dir.join("vachanamrut_en_1.html");
    if vach_en_path.exists() {
        let content = fs::read_to_string(&vach_en_path).expect("read vachanamrut_en_1");
        let unit_ref = UnitRef::new("vachanamrut", "1");
        let unit = parse_vachanamrut_page(&content, "en", &unit_ref).expect("parse vachanamrut en");
        assert!(!unit.blocks.is_empty(), "vachanamrut en blocks empty");
    }

    // 2. Kirtan
    for kirtan_file in &["kirtan_p1_1.html", "kirtan_p2_1.html"] {
        let path = base_dir.join(kirtan_file);
        if path.exists() {
            let content = fs::read_to_string(&path).expect("read kirtan");
            let unit_ref = UnitRef::new("kirtan", "1-1");
            let unit = parse_kirtan(&content, &unit_ref).expect("parse kirtan");
            assert!(
                !unit.blocks.is_empty(),
                "kirtan blocks empty in {}",
                kirtan_file
            );
        }
    }

    // 3. Swamini Vato
    for vato_file in &["vato_gu_1_10.html", "vato_en_1_10.html", "vato_p1_50.html"] {
        let path = base_dir.join(vato_file);
        if path.exists() {
            let content = fs::read_to_string(&path).expect("read vato");
            let unit_ref = UnitRef::new("vato", "1-1");
            let unit = parse_vato(&content, &unit_ref).expect("parse vato");
            assert!(
                !unit.blocks.is_empty(),
                "vato blocks empty in {}",
                vato_file
            );
        }
    }

    // 4. Satsang Diksha
    let diksha_path = base_dir.join("diksha_1_5.html");
    if diksha_path.exists() {
        let content = fs::read_to_string(&diksha_path).expect("read diksha");
        let unit_ref = UnitRef::new("diksha", "1");
        let unit = parse_diksha(&content, &unit_ref).expect("parse diksha");
        assert!(!unit.blocks.is_empty(), "diksha blocks empty");
    }

    // 5. Nishkulanand Kavya
    for kavya_file in &["kavya_prakash_1.html", "kavya_chosath_padi_1.html"] {
        let path = base_dir.join(kavya_file);
        if path.exists() {
            let content = fs::read_to_string(&path).expect("read kavya");
            let unit_ref = UnitRef::new("kavya", "1");
            let unit = parse_kavya(&content, &unit_ref).expect("parse kavya");
            assert!(
                !unit.blocks.is_empty(),
                "kavya blocks empty in {}",
                kavya_file
            );
        }
    }

    // 6. Bhaktachintamani
    let chintamani_path = base_dir.join("chintamani_1.html");
    if chintamani_path.exists() {
        let content = fs::read_to_string(&chintamani_path).expect("read chintamani");
        let unit_ref = UnitRef::new("chintamani", "1");
        let unit = parse_chintamani(&content, &unit_ref).expect("parse chintamani");
        assert!(!unit.blocks.is_empty(), "chintamani blocks empty");
    }

    // 7. Harililamrut
    let harililamrut_path = base_dir.join("harililamrut_k1_v1.html");
    if harililamrut_path.exists() {
        let content = fs::read_to_string(&harililamrut_path).expect("read harililamrut");
        let unit_ref = UnitRef::new("harililamrut", "1-1");
        let unit = parse_harililamrut(&content, &unit_ref).expect("parse harililamrut");
        assert!(!unit.blocks.is_empty(), "harililamrut blocks empty");
    }

    // 8. Yogi Gita
    let yogi_gita_path = base_dir.join("yogi_gita_sunrut.html");
    if yogi_gita_path.exists() {
        let content = fs::read_to_string(&yogi_gita_path).expect("read yogi gita");
        let unit_ref = UnitRef::new("yogi-gita", "sunrut");
        let unit = parse_yogi_gita(&content, &unit_ref).expect("parse yogi gita");
        assert!(!unit.blocks.is_empty(), "yogi gita blocks empty");
    }

    // 9. Karika
    let karika_path = base_dir.join("karika_mangal.html");
    if karika_path.exists() {
        let content = fs::read_to_string(&karika_path).expect("read karika");
        let unit_ref = UnitRef::new("karika", "mangal");
        let unit = parse_karika(&content, &unit_ref).expect("parse karika");
        assert!(!unit.blocks.is_empty(), "karika blocks empty");
    }

    // 10. Aksharamrutam
    let aksharamrutam_path = base_dir.join("aksharamrutam_p8.html");
    if aksharamrutam_path.exists() {
        let content = fs::read_to_string(&aksharamrutam_path).expect("read aksharamrutam");
        let unit_ref = UnitRef::new("aksharamrutam", "1");
        let unit = parse_aksharamrutam(&content, &unit_ref).expect("parse aksharamrutam");
        assert!(!unit.blocks.is_empty(), "aksharamrutam blocks empty");
    }

    // 11. Purushottam Bolya Prite
    let pbp_path = base_dir.join("pbp_chp_1.html");
    if pbp_path.exists() {
        let content = fs::read_to_string(&pbp_path).expect("read pbp");
        let unit_ref = UnitRef::new("pbp", "1");
        let unit = parse_pbp(&content, &unit_ref).expect("parse pbp");
        assert!(!unit.blocks.is_empty(), "pbp blocks empty");
    }

    // 12. Sadguru Aksharanand Swamini Vato
    let asv_path = base_dir.join("asv_chp_1.html");
    if asv_path.exists() {
        let content = fs::read_to_string(&asv_path).expect("read asv");
        let unit_ref = UnitRef::new("asv", "1");
        let unit = parse_asv(&content, &unit_ref).expect("parse asv");
        assert!(!unit.blocks.is_empty(), "asv blocks empty");
    }

    // 13. Bhagvad Gita
    let gita_path = base_dir.join("gita_chp_1.html");
    if gita_path.exists() {
        let content = fs::read_to_string(&gita_path).expect("read gita");
        let unit_ref = UnitRef::new("gita", "1");
        let unit = parse_gita(&content, &unit_ref).expect("parse gita");
        assert!(!unit.blocks.is_empty(), "gita blocks empty");
    }

    // 14. Bhagavata Purana
    let bhagvat_path = base_dir.join("bhagvat_s1_a1.html");
    if bhagvat_path.exists() {
        let content = fs::read_to_string(&bhagvat_path).expect("read bhagvat");
        let unit_ref = UnitRef::new("bhagvat", "1-1");
        let unit = parse_bhagvat(&content, &unit_ref).expect("parse bhagvat");
        assert!(!unit.blocks.is_empty(), "bhagvat blocks empty");
    }

    // 15. Sanskar Kunj Shloks
    let shloks_path = base_dir.join("kirtan_shloks.html");
    if shloks_path.exists() {
        let content = fs::read_to_string(&shloks_path).expect("read shloks");
        let unit_ref = UnitRef::new("shloks", "collection");
        let unit = parse_shloks(&content, &unit_ref).expect("parse shloks");
        assert!(!unit.blocks.is_empty(), "shloks blocks empty");
    }
}
