use lumen_library::error::LibraryError;
use lumen_library::model::{BlockKind, UnitRef};
use lumen_library::parse::*;

#[test]
fn test_parse_vachanamrut_synthetic() {
    let gu_html = r#"
    <div id="vach_text">
        <h3 class="pra_secno">ગઢડા પ્રથમ ૧</h3>
        <h1 class="title_gu">ગુજરાતી શીર્ષક<sup>૧</sup></h1>
        <div id="audio_wrap"><audio src="test.mp3"></audio></div>
        <cite class="text_gu">મંગળાચરણ શ્લોક પંક્તિ ૧<br>શ્લોક પંક્તિ ૨</cite>
        <p class="text_gu">ગુજરાતી પાઠ પેરાગ્રાફ એક <span class="target">શબ્દ</span><sup>૧</sup> અને આગળનો ભાગ.</p>
        <p class="text_gu">ગુજરાતી પાઠ પેરાગ્રાફ બે.</p>
    </div>
    <div id="tab-footnotes">
        <div class="footnote-item"><p id="fn_1" class="text_gu">૧. પ્રથમ પાદટીપ નોંધ.</p></div>
    </div>
    "#;

    let en_html = r#"
    <div id="vach_text">
        <h3 class="pra_secno">Gadhada I-1</h3>
        <h1 class="title_en">Sample Title English<sup>1</sup></h1>
        <div id="audio_wrap"><audio src="test.mp3"></audio></div>
        <cite class="text_en">Opening verse line one<br>Opening verse line two</cite>
        <p class="text_en">English text paragraph one <span class="target">word one</span><sup>1</sup> and following part.</p>
        <p class="text_en">English text paragraph two.</p>
    </div>
    <div id="tab-footnotes">
        <div class="footnote-item"><p id="fn_1" class="text_en">1. First footnote note.</p></div>
    </div>
    "#;

    let unit_ref = UnitRef::new("vachanamrut", "gadhada-1-1");
    let unit = parse_vachanamrut(gu_html, en_html, &unit_ref).expect("parse vachanamrut");

    assert_eq!(unit.key, "gadhada-1-1");
    assert_eq!(unit.label, "Gadhada I-1");
    assert_eq!(unit.title_gu.as_deref(), Some("ગુજરાતી શીર્ષક"));
    assert_eq!(unit.title_en.as_deref(), Some("Sample Title English"));

    // 1 verse + 2 paragraphs = 3 blocks
    assert_eq!(unit.blocks.len(), 3);
    assert_eq!(unit.blocks[0].kind, BlockKind::Verse);
    assert!(unit.blocks[0].gu.as_ref().unwrap().contains("મંગળાચરણ"));
    assert!(
        unit.blocks[0]
            .en
            .as_ref()
            .unwrap()
            .contains("Opening verse")
    );

    assert_eq!(unit.blocks[1].kind, BlockKind::Paragraph);
    assert!(unit.blocks[1].gu.as_ref().unwrap().contains("ગુજરાતી પાઠ"));
    // Footnote should not be in text
    assert!(!unit.blocks[1].gu.as_ref().unwrap().contains("પ્રથમ પાદટીપ"));
    assert!(
        unit.blocks[1]
            .notes
            .as_ref()
            .unwrap()
            .contains("પ્રથમ પાદટીપ")
    );
    assert!(
        unit.blocks[1]
            .notes
            .as_ref()
            .unwrap()
            .contains("First footnote")
    );

    assert_eq!(unit.blocks[2].kind, BlockKind::Paragraph);
    assert!(unit.blocks[2].gu.as_ref().unwrap().contains("પેરાગ્રાફ બે"));
    assert!(
        unit.blocks[2]
            .en
            .as_ref()
            .unwrap()
            .contains("paragraph two")
    );
}

#[test]
fn test_vachanamrut_count_mismatch_warning() {
    let gu_html = r#"
    <div id="vach_text">
        <h3 class="pra_secno">Gadhada I-1</h3>
        <h1 class="title_gu">શીર્ષક</h1>
        <p class="text_gu">ગુજરાતી ફકરો એક.</p>
        <p class="text_gu">ગુજરાતી ફકરો બે.</p>
    </div>
    "#;

    let en_html = r#"
    <div id="vach_text">
        <h3 class="pra_secno">Gadhada I-1</h3>
        <h1 class="title_en">Title</h1>
        <p class="text_en">English paragraph one only.</p>
    </div>
    "#;

    let unit_ref = UnitRef::new("vachanamrut", "gadhada-1-1");
    let gu_u = parse_vachanamrut_page(gu_html, "gu", &unit_ref).unwrap();
    let en_u = parse_vachanamrut_page(en_html, "en", &unit_ref).unwrap();

    let (merged, warn) = merge_vachanamrut_units(gu_u, en_u);
    assert!(warn.is_some(), "expected mismatch warning");
    assert!(warn.unwrap().contains("paragraph count mismatch"));
    // Keeps both sequences: 2 gu + 1 en = 3 blocks
    assert_eq!(merged.blocks.len(), 3);
    assert!(merged.blocks[0].gu.is_some() && merged.blocks[0].en.is_none());
    assert!(merged.blocks[1].gu.is_some() && merged.blocks[1].en.is_none());
    assert!(merged.blocks[2].gu.is_none() && merged.blocks[2].en.is_some());
}

#[test]
fn test_vachanamrut_missing_layout_error() {
    let invalid_html = "<div>No vach_text container</div>";
    let unit_ref = UnitRef::new("vachanamrut", "gadhada-1-1");
    let res = parse_vachanamrut_page(invalid_html, "gu", &unit_ref);
    match res {
        Err(LibraryError::Layout { book, unit, .. }) => {
            assert_eq!(book, "vachanamrut");
            assert_eq!(unit, "gadhada-1-1");
        }
        _ => panic!("expected LibraryError::Layout"),
    }
}

#[test]
fn test_parse_vato_synthetic() {
    let html = r#"
    <div class="vat_card">
        <div class="vat_wrap" id="vat_gu_1">
            <div class="audio_wrap"><audio class="player"></audio></div>
            <div class="vat"><p class="text_gu">વાત એક ગુજરાતી વિવરણ.</p></div>
            <div class="vatinfo_gu"><i>સાધુ મહિમા</i> / <b>(૧/૧)</b></div>
        </div>
        <div class="vat_wrap" id="vat_en_1">
            <div class="vat"><p class="text_en">Discourse one English explanation.</p></div>
            <div class="vatinfo_en"><i>Sadhu Glory</i> / <b>(1/1)</b></div>
            <div class="footnote"><p class="text_en">Explanation footnote.</p></div>
        </div>
        <div class="vat_wrap" id="vat_trans_1">
            <div class="vat"><p class="text_trans">Vat ek transliteration.</p></div>
        </div>
    </div>
    "#;

    let unit_ref = UnitRef::new("vato", "1-1");
    let unit = parse_vato(html, &unit_ref).expect("parse vato");
    assert_eq!(unit.key, "1-1");
    assert_eq!(unit.label, "(1/1)");
    assert_eq!(unit.blocks.len(), 1);
    let b = &unit.blocks[0];
    assert_eq!(b.kind, BlockKind::Paragraph);
    assert_eq!(b.gu.as_deref(), Some("વાત એક ગુજરાતી વિવરણ."));
    assert_eq!(b.en.as_deref(), Some("Discourse one English explanation."));
    assert_eq!(b.translit.as_deref(), Some("Vat ek transliteration."));
    assert_eq!(b.notes.as_deref(), Some("Explanation footnote."));
}

#[test]
fn test_parse_kirtan_synthetic_stanzas_and_refrains() {
    let html = r#"
    <div id="kirtan_wrap_gu">
        <h1 id="title_gu" class="gu">પદ શીર્ષક એક</h1>
        <div id="text_gu" class="gu">
            <p class="k_line_gu">પ્રથમ પંક્તિ ટેક ભાગ એક,</p>
            <p class="k_line_gu">&emsp;બીજી પંક્તિ ટેક ભાગ બે... ꠶ટેક</p>
            <p class="k_line_gu">અંતરા પંક્તિ એક શબ્દ,</p>
            <p class="k_line_gu">&emsp;અંતરા પંક્તિ બે શબ્દ... ꠶ ૧</p>
        </div>
    </div>
    <div id="kirtan_wrap_en">
        <h1 id="title_en" class="en">Pad Title One</h1>
        <div id="text_en" class="en">
            <p class="k_line_en">First line refrain part one,</p>
            <p class="k_line_en">&emsp;second line refrain part two... tek</p>
            <p class="k_line_en">Stanza line one,</p>
            <p class="k_line_en">&emsp;stanza line two... 1</p>
        </div>
    </div>
    "#;

    let unit_ref = UnitRef::new("kirtan", "1-1");
    let unit = parse_kirtan(html, &unit_ref).expect("parse kirtan");
    assert_eq!(unit.key, "1-1");
    assert_eq!(unit.title_gu.as_deref(), Some("પદ શીર્ષક એક"));
    assert_eq!(unit.title_en.as_deref(), Some("Pad Title One"));

    assert_eq!(unit.blocks.len(), 2);
    // Block 0 is refrain
    assert_eq!(unit.blocks[0].kind, BlockKind::Refrain);
    let refrain_gu = unit.blocks[0].gu.as_ref().unwrap();
    assert!(refrain_gu.contains('\n'), "stanzas must keep line breaks");
    assert!(refrain_gu.contains("꠶ટેક"), "must preserve refrain mark");

    // Block 1 is stanza
    assert_eq!(unit.blocks[1].kind, BlockKind::Stanza);
    let stanza_gu = unit.blocks[1].gu.as_ref().unwrap();
    assert!(stanza_gu.contains('\n'));
    assert!(stanza_gu.contains("꠶ ૧"));
}

#[test]
fn test_parse_diksha_synthetic() {
    let html = r#"
    <div class="shloks_wrap" id="shlok_wrap_1">
        <div class="shlok_num"><div class="circle">1</div></div>
        <div class="sanskrit_html"><p class="shlok_hi">संस्कृत श्लोक एक</p></div>
        <div class="gujarati_sans_html"><p class="shlok_gu">સંસ્કૃત શ્લોક એક ગુજરાતી લિપિ</p></div>
        <div class="sanskrit_trans_html"><p class="shlok_trans">Sanskrit shlok ek translit</p></div>
        <div class="gujarati_html"><p class="meaning_gu">શ્લોક એક નો ગુજરાતી અર્થ.</p></div>
        <div class="english_html"><p class="meaning_en">Shlok one English meaning.</p></div>
    </div>
    "#;

    let unit_ref = UnitRef::new("diksha", "1");
    let unit = parse_diksha(html, &unit_ref).expect("parse diksha");
    assert_eq!(unit.key, "1");
    assert_eq!(unit.label, "Shlok 1");

    // Verse block + meaning block
    assert_eq!(unit.blocks.len(), 2);
    assert_eq!(unit.blocks[0].kind, BlockKind::Shlok);
    assert_eq!(
        unit.blocks[0].gu.as_deref(),
        Some("સંસ્કૃત શ્લોક એક ગુજરાતી લિપિ")
    );
    assert_eq!(
        unit.blocks[0].translit.as_deref(),
        Some("Sanskrit shlok ek translit")
    );

    assert_eq!(unit.blocks[1].kind, BlockKind::Paragraph);
    assert_eq!(unit.blocks[1].gu.as_deref(), Some("શ્લોક એક નો ગુજરાતી અર્થ."));
    assert_eq!(
        unit.blocks[1].en.as_deref(),
        Some("Shlok one English meaning.")
    );
}

#[test]
fn test_parse_kavya_synthetic() {
    let html = r#"
    <h1 class="title">કાવ્ય ગ્રંથ શીર્ષક</h1>
    <h2 class="section">પ્રકારઃ ૧</h2>
    <p class="line kavya-verse"><span class="kavya-verse-text">કાવ્ય પંક્તિ પ્રથમ ભાગ</span> <span class="kavya-verse-number">૧॥</span></p>
    <p class="line_en">Kavya line first part 1</p>
    <p class="meaning_en">Kavya English meaning one.</p>
    "#;

    let unit_ref = UnitRef::new("kavya", "1");
    let unit = parse_kavya(html, &unit_ref).expect("parse kavya");
    assert_eq!(unit.label, "પ્રકારઃ ૧");
    assert_eq!(unit.blocks.len(), 1);
    assert_eq!(unit.blocks[0].kind, BlockKind::Verse);
    assert!(unit.blocks[0].gu.as_ref().unwrap().contains("કાવ્ય પંક્તિ"));
    assert!(
        unit.blocks[0]
            .translit
            .as_ref()
            .unwrap()
            .contains("Kavya line")
    );
    assert!(
        unit.blocks[0]
            .en
            .as_ref()
            .unwrap()
            .contains("English meaning")
    );
}

#[test]
fn test_parse_chintamani_synthetic() {
    let html = r#"
    <h1 class="title_uni">ચિંતામણિ પ્રકરણ શીર્ષક</h1>
    <h1 class="title_trans">Chintamani Prakaran Title</h1>
    <p class="line_uni verse-line"><span class="verse-text">ચિંતામણિ પંક્તિ એક</span> <span class="verse-number">૧</span></p>
    <p class="line_trans verse-line"><span class="verse-text">Chintamani line one</span> <span class="verse-number">1</span></p>
    "#;

    let unit_ref = UnitRef::new("chintamani", "1");
    let unit = parse_chintamani(html, &unit_ref).expect("parse chintamani");
    assert_eq!(unit.blocks.len(), 1);
    assert_eq!(unit.blocks[0].kind, BlockKind::Verse);
    assert!(unit.blocks[0].gu.as_ref().unwrap().contains("ચિંતામણિ પંક્તિ"));
    assert!(
        unit.blocks[0]
            .translit
            .as_ref()
            .unwrap()
            .contains("Chintamani line")
    );
}

#[test]
fn test_parse_harililamrut_synthetic() {
    let html = r#"
    <h1 class="title_gu">કળશ ૧ વિશ્રામ ૧</h1>
    <p class="line hla-verse"><span class="hla-verse-text">હરિલીલામૃત દોહરો પંક્તિ એક</span> <span class="hla-verse-number">૧</span></p>
    "#;

    let unit_ref = UnitRef::new("harililamrut", "1-1");
    let unit = parse_harililamrut(html, &unit_ref).expect("parse harililamrut");
    assert_eq!(unit.blocks.len(), 1);
    assert_eq!(unit.blocks[0].kind, BlockKind::Verse);
    assert!(unit.blocks[0].gu.as_ref().unwrap().contains("હરિલીલામૃત"));
}

#[test]
fn test_parse_yogi_gita_synthetic() {
    let html = r#"
    <h1 class="sec_title">યોગી ગીતા સૂનૃત</h1>
    <div class="gu_text"><p class="gu_center">યોગીજી મહારાજ બોલ્યા વચન એક.</p></div>
    <div class="en_text"><p class="en_center">Yogiji Maharaj spoke discourse one.</p></div>
    "#;

    let unit_ref = UnitRef::new("yogi-gita", "sunrut");
    let unit = parse_yogi_gita(html, &unit_ref).expect("parse yogi gita");
    assert_eq!(unit.label, "યોગી ગીતા સૂનૃત");
    assert_eq!(unit.blocks.len(), 1);
    assert_eq!(unit.blocks[0].kind, BlockKind::Paragraph);
    assert!(
        unit.blocks[0]
            .gu
            .as_ref()
            .unwrap()
            .contains("યોગીજી મહારાજ")
    );
    assert!(
        unit.blocks[0]
            .en
            .as_ref()
            .unwrap()
            .contains("Yogiji Maharaj")
    );
}

#[test]
fn test_parse_karika_synthetic() {
    let html = r#"
    <div id="text_body">
        <p class="hi_title">कारिका शीर्षक</p>
        <p class="hi_center">अक्षरब्रह्म गुण सम्पन्नं पुरुषोत्तम रूपिणम् ॥ १॥</p>
    </div>
    "#;

    let unit_ref = UnitRef::new("karika", "mangal");
    let unit = parse_karika(html, &unit_ref).expect("parse karika");
    assert_eq!(unit.blocks.len(), 1);
    assert_eq!(unit.blocks[0].kind, BlockKind::Shlok);
    assert!(
        unit.blocks[0]
            .translit
            .as_ref()
            .unwrap()
            .contains("अक्षरब्रह्म")
    );
}

#[test]
fn test_parse_aksharamrutam_synthetic() {
    let html = r#"
    <p class="vat_text_gu" id="gu_pra_8_vat_1">અક્ષરામૃતમ્ વાત પાઠ એક વિવરણ. <b>(૧)</b></p>
    "#;

    let unit_ref = UnitRef::new("aksharamrutam", "1");
    let unit = parse_aksharamrutam(html, &unit_ref).expect("parse aksharamrutam");
    assert_eq!(unit.blocks.len(), 1);
    assert_eq!(unit.blocks[0].kind, BlockKind::Paragraph);
    assert!(unit.blocks[0].gu.as_ref().unwrap().contains("અક્ષરામૃતમ્"));
}

#[test]
fn test_parse_pbp_synthetic() {
    let html = r#"
    <h1 class="chp_title">૧. પ્રકરણ શીર્ષક</h1>
    <p class="gu_indent">પુરુષોત્તમ બોલ્યા પ્રીતે ફકરો એક.</p>
    "#;

    let unit_ref = UnitRef::new("pbp", "1");
    let unit = parse_pbp(html, &unit_ref).expect("parse pbp");
    assert_eq!(unit.blocks.len(), 1);
    assert_eq!(unit.blocks[0].kind, BlockKind::Paragraph);
    assert!(
        unit.blocks[0]
            .gu
            .as_ref()
            .unwrap()
            .contains("પુરુષોત્તમ બોલ્યા")
    );
}

#[test]
fn test_parse_asv_synthetic() {
    let html = r#"
    <h1 class="chp_title">અક્ષરાનંદ સ્વામી વાત એક</h1>
    <div class="vat_wrap"><p class="gu_indent">અક્ષરાનંદ સ્વામીની વાતો પ્રસંગ એક.</p></div>
    "#;

    let unit_ref = UnitRef::new("asv", "1");
    let unit = parse_asv(html, &unit_ref).expect("parse asv");
    assert_eq!(unit.blocks.len(), 1);
    assert_eq!(unit.blocks[0].kind, BlockKind::Paragraph);
    assert!(
        unit.blocks[0]
            .gu
            .as_ref()
            .unwrap()
            .contains("અક્ષરાનંદ સ્વામીની વાતો")
    );
}

#[test]
fn test_parse_gita_synthetic() {
    let html = r#"
    <h3 class="chapter_title">અધ્યાય ૧</h3>
    <p class="shloka_no">Shlok 1.1</p>
    <p class="sanskrit">धर्मक्षेत्रे कुरुक्षेत्रे</p>
    <p class="transliteration">dharmakshetre kurukshetre</p>
    <p class="translation">In the field of righteousness</p>
    "#;

    let unit_ref = UnitRef::new("gita", "1");
    let unit = parse_gita(html, &unit_ref).expect("parse gita");
    assert_eq!(unit.blocks.len(), 1);
    assert_eq!(unit.blocks[0].kind, BlockKind::Shlok);
    assert_eq!(unit.blocks[0].translit.as_deref(), Some("धर्मक्षेत्रे कुरुक्षेत्रे"));
    assert_eq!(
        unit.blocks[0].en.as_deref(),
        Some("In the field of righteousness")
    );
}

#[test]
fn test_parse_bhagvat_synthetic() {
    let html = r#"
    <h2 class="adhyay">અધ્યાય ૧</h2>
    <p class="hi_left">जन्माद्यस्य यतोऽन्वयादितरतश्चार्थेष्वभिज्ञः स्वराट्</p>
    "#;

    let unit_ref = UnitRef::new("bhagvat", "1-1");
    let unit = parse_bhagvat(html, &unit_ref).expect("parse bhagvat");
    assert_eq!(unit.blocks.len(), 1);
    assert_eq!(unit.blocks[0].kind, BlockKind::Shlok);
    assert!(
        unit.blocks[0]
            .translit
            .as_ref()
            .unwrap()
            .contains("जन्माद्यस्य")
    );
}

#[test]
fn test_parse_shloks_synthetic() {
    let html = r#"
    <p class="shloka_gu">ગુણાતીતોક્ષરં બ્રહ્મ</p>
    <p class="shloka_sa">गुणातीतोक्षरं ब्रह्म</p>
    <p class="meaning_gu">ગુણાતીતાનંદ સ્વામી મૂળ અક્ષર છે.</p>
    <p class="meaning_en">Gunatitanand Swami is Mul Akshar.</p>
    "#;

    let unit_ref = UnitRef::new("shloks", "1");
    let unit = parse_shloks(html, &unit_ref).expect("parse shloks");
    assert_eq!(unit.blocks.len(), 2);
    assert_eq!(unit.blocks[0].kind, BlockKind::Shlok);
    assert_eq!(unit.blocks[1].kind, BlockKind::Paragraph);
    assert!(unit.blocks[0].gu.as_ref().unwrap().contains("ગુણાતીતોક્ષરં"));
    assert!(unit.blocks[1].en.as_ref().unwrap().contains("Gunatitanand"));
}
