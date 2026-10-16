use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookInfo {
    pub id: &'static str,
    pub title_gu: &'static str,
    pub title_en: &'static str,
    pub units: usize,
    pub pages: usize,
}

pub const CATALOG: &[BookInfo] = &[
    BookInfo {
        id: "vachanamrut",
        title_gu: "વચનામૃત",
        title_en: "Vachanamrut",
        units: 274,
        pages: 548,
    },
    BookInfo {
        id: "vato",
        title_gu: "સ્વામીની વાતો",
        title_en: "Swamini Vato",
        units: 1484,
        pages: 152,
    },
    BookInfo {
        id: "kirtan",
        title_gu: "કીર્તન મુક્તાવલી",
        title_en: "Kirtan Muktavali",
        units: 2491,
        pages: 2491,
    },
    BookInfo {
        id: "diksha",
        title_gu: "સત્સંગદીક્ષા",
        title_en: "Satsang Diksha",
        units: 315,
        pages: 63,
    },
    BookInfo {
        id: "kavya",
        title_gu: "નિષ્કુળાનંદ કાવ્ય",
        title_en: "Nishkulanand Kavya",
        units: 600,
        pages: 600,
    },
    BookInfo {
        id: "chintamani",
        title_gu: "ભક્તચિંતામણિ",
        title_en: "Bhaktachintamani",
        units: 164,
        pages: 164,
    },
    BookInfo {
        id: "harililamrut",
        title_gu: "શ્રીહરિલીલામૃત",
        title_en: "Harililamrut",
        units: 332,
        pages: 337,
    },
    BookInfo {
        id: "yogi-gita",
        title_gu: "યોગી ગીતા",
        title_en: "Yogi Gita",
        units: 8,
        pages: 8,
    },
    BookInfo {
        id: "karika",
        title_gu: "સ્વામિનારાયણ સિદ્ધાંત કારિકા",
        title_en: "Swaminarayan Siddhant Karika",
        units: 11,
        pages: 21,
    },
    BookInfo {
        id: "aksharamrutam",
        title_gu: "અક્ષરામૃતમ્",
        title_en: "Aksharamrutam",
        units: 1044,
        pages: 105,
    },
    BookInfo {
        id: "pbp",
        title_gu: "પુરુષોત્તમ બોલ્યા પ્રીતે",
        title_en: "Purushottam Bolya Prite",
        units: 31,
        pages: 34,
    },
    BookInfo {
        id: "asv",
        title_gu: "સદ્‌ગુરુ અક્ષરાનંદ સ્વામીની વાતો",
        title_en: "Sadguru Aksharanand Swamini Vato",
        units: 39,
        pages: 39,
    },
    BookInfo {
        id: "gita",
        title_gu: "શ્રીમદ્ ભગવદ્ ગીતા",
        title_en: "Shrimad Bhagvad Gita",
        units: 18,
        pages: 25,
    },
    BookInfo {
        id: "bhagvat",
        title_gu: "શ્રીમદ્ભાગવતપુરાણ",
        title_en: "Shrimad Bhagavata Purana",
        units: 347,
        pages: 347,
    },
    BookInfo {
        id: "shloks",
        title_gu: "સંસ્કાર કુંજ",
        title_en: "Sanskar Kunj",
        units: 2,
        pages: 5,
    },
];

pub fn catalog() -> &'static [BookInfo] {
    CATALOG
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Book {
    pub id: String,
    pub title_gu: String,
    pub title_en: String,
    pub units: Vec<Unit>,
    pub schema: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Unit {
    pub key: String,   /* e.g. "gadhada-1-1" */
    pub label: String, /* "Gadhada I-1" */
    pub title_gu: Option<String>,
    pub title_en: Option<String>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Block {
    pub gu: Option<String>,
    pub en: Option<String>,
    pub translit: Option<String>,
    pub notes: Option<String>, /* footnotes */
    pub kind: BlockKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph,
    Verse,
    Stanza,
    Refrain,
    Shlok,
    Heading,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnitRef {
    pub book: String,
    pub key: String,
}

impl UnitRef {
    pub fn new(book: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            book: book.into(),
            key: key.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookSummary {
    pub id: String,
    pub title_gu: String,
    pub title_en: String,
    pub units: usize,
    pub bytes: u64,
}

pub fn to_nfc(s: &str) -> String {
    s.nfc().collect()
}
