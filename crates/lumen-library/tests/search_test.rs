use lumen_library::model::{Block, BlockKind, Book, Unit};
use lumen_library::search::SearchIndex;
use std::time::{Duration, Instant};

fn sample_books() -> Vec<Book> {
    vec![
        Book {
            id: "vachanamrut".into(),
            title_gu: "વચનામૃત".into(),
            title_en: "Vachanamrut".into(),
            units: vec![
                Unit {
                    key: "gadhada-1-1".into(),
                    label: "Gadhada I-1".into(),
                    title_gu: Some("અખંડ વૃત્તિનું".into()),
                    title_en: Some("Continuously Engaging Mind".into()),
                    blocks: vec![Block {
                        gu: Some("ભગવાનના સ્વરૂપમાં મનની અખંડ વૃત્તિ રાખવી.".into()),
                        en: Some("One should continuously engage one's mind on God.".into()),
                        translit: None,
                        notes: None,
                        kind: BlockKind::Paragraph,
                    }],
                },
                Unit {
                    key: "gadhada-1-2".into(),
                    label: "Gadhada I-2".into(),
                    title_gu: Some("સંત સેવવાનું".into()),
                    title_en: Some("Serving the Sant".into()),
                    blocks: vec![Block {
                        gu: Some("સંતની સેવા કરવી એ મોટું સાધન છે.".into()),
                        en: Some("Serving the Sant is a great spiritual endeavor.".into()),
                        translit: None,
                        notes: None,
                        kind: BlockKind::Paragraph,
                    }],
                },
            ],
            schema: 1,
        },
        Book {
            id: "kirtan".into(),
            title_gu: "કીર્તન મુક્તાવલી".into(),
            title_en: "Kirtan Muktavali".into(),
            units: vec![Unit {
                key: "1-1".into(),
                label: "1-1".into(),
                title_gu: Some("માધોજી મેરે તુમ હી એક".into()),
                title_en: Some("Madhoji Mere Tum Hi Ek".into()),
                blocks: vec![Block {
                    gu: Some("માધોજી મેરે તુમ હી એક, પાંઉ ધરનકો ઠેકાનો... ꠶ટેક".into()),
                    en: Some("Madhoji mere tum hi ek, pau dharanko thekano...".into()),
                    translit: None,
                    notes: None,
                    kind: BlockKind::Stanza,
                }],
            }],
            schema: 1,
        },
    ]
}

#[test]
fn test_search_ranking_label_over_title_over_text() {
    let books = sample_books();
    let index = SearchIndex::new(&books);

    // 1. Label match ranks first
    let hits = index.search("gadhada i-1", 10);
    assert!(!hits.is_empty());
    assert_eq!(hits[0].unit_key, "gadhada-1-1");

    // 2. Abbreviation label match "ga 1-1"
    let hits_abbrev = index.search("ga 1-1", 10);
    assert!(!hits_abbrev.is_empty());
    assert_eq!(hits_abbrev[0].unit_key, "gadhada-1-1");

    // 3. Case-insensitive English search
    let hits_ci = index.search("cOnTiNuOuSlY", 10);
    assert!(!hits_ci.is_empty());
    assert_eq!(hits_ci[0].unit_key, "gadhada-1-1");

    // 4. Gujarati NFC search
    let hits_gu = index.search("અખંડ", 10);
    assert!(!hits_gu.is_empty());
    // Title hit comes before text hit
    assert_eq!(hits_gu[0].unit_key, "gadhada-1-1");
}

#[test]
fn test_search_benchmark_2491_kirtans() {
    // Construct synthetic 2,491 kirtan units
    let mut units = Vec::with_capacity(2491);
    for i in 1..=2491 {
        let part = if i <= 1300 { 1 } else { 2 };
        let no = if i <= 1300 { i } else { i - 1300 };
        units.push(Unit {
            key: format!("{part}-{no}"),
            label: format!("{part}-{no}"),
            title_gu: Some(format!("કીર્તન શીર્ષક પદ {i}")),
            title_en: Some(format!("Kirtan Title Pad {i}")),
            blocks: vec![
                Block {
                    gu: Some(format!("પંક્તિ એક ટેક ભાગ {i}\nપંક્તિ બે ટેક ભાગ {i}")),
                    en: Some(format!(
                        "Line one refrain part {i}\nLine two refrain part {i}"
                    )),
                    translit: None,
                    notes: None,
                    kind: BlockKind::Stanza,
                },
                Block {
                    gu: Some(format!("અંતરા પંક્તિ એક {i}\nઅંતરા પંક્તિ બે {i}")),
                    en: Some(format!("Stanza line one {i}\nStanza line two {i}")),
                    translit: None,
                    notes: None,
                    kind: BlockKind::Stanza,
                },
            ],
        });
    }

    let kirtan_book = Book {
        id: "kirtan".into(),
        title_gu: "કીર્તન મુક્તાવલી".into(),
        title_en: "Kirtan Muktavali".into(),
        units,
        schema: 1,
    };

    let start_build = Instant::now();
    let index = SearchIndex::new(&[kirtan_book]);
    let build_time = start_build.elapsed();
    eprintln!("Built index for 2,491 units in {:?}", build_time);

    // Measure search queries
    let queries = ["kirtan", "pad 500", "refrain", "પંક્તિ", "1-750"];
    for q in queries {
        let start = Instant::now();
        let hits = index.search(q, 20);
        let elapsed = start.elapsed();
        eprintln!("Query '{q}' took {:?} ({} hits)", elapsed, hits.len());
        assert!(!hits.is_empty(), "expected hits for query '{q}'");
        assert!(
            elapsed < Duration::from_millis(50),
            "search took {:?}, expected < 50ms",
            elapsed
        );
    }
}
