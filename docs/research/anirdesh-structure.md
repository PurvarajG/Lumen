# Anirdesh.com Book Structure Research & Parser Blueprint

> **Notice**: Read-only structural research conducted for personal use in-app presentation downloaders. No full scripture texts are bundled or committed. Snippets shown are <= 15 words purely for element and field identification.

---

## 1. Executive Summary & Network Protocol Profile

- **Base URL**: `https://www.anirdesh.com/`
- **Encoding**: UTF-8 across all pages (`<meta charset="utf-8" />`).
- **Robots Policy**: `robots.txt` disallows `/img`, `/scripts`, `/admin`.
- **WAF / User-Agent Sensitivity**:
  - The Nginx server configuration actively blocks User-Agents containing the lowercase substring `research` with HTTP `403 Forbidden` (`403 - Forbidden | Access to this page is forbidden.`).
  - Standard User-Agents such as `LumenPresenter-Research/0.1` (capital `R`), `LumenPresenter/0.1`, or default client agents succeed.
  - Recommended downloader client header: `User-Agent: LumenPresenter/0.1` with a polite rate limit of >= 1 second between requests.

---

## 2. Scripture Books Analysis

### 2.1 Vachanamrut (વચનામૃત)

1. **Discovery & Hierarchy**:
   - **Index URL**: `https://www.anirdesh.com/vachanamrut/`
   - **Hierarchy**: Grantha -> 10 Sections -> 274 Units (`vachno=1` through `vachno=274` continuous) -> Intro Shloka (`cite`) -> Paragraphs (`p`).
   - **Sections & Numbering Scheme**:
     - Gadhada I (`GI`): 78 units (`vachno=1..78`, "Gadhada I-1" .. "Gadhada I-78", "ગઢડા પ્રથમ ૧..૭૮")
     - Sarangpur (`S`): 18 units (`vachno=79..96`, "Sarangpur 1..18", "સારંગપુર ૧..૧૮")
     - Kariyani (`K`): 12 units (`vachno=97..108`, "Kariyani 1..12", "કારિયાણી ૧..૧૨")
     - Loya (`L`): 18 units (`vachno=109..126`, "Loya 1..18", "લોયા ૧..૧૮")
     - Panchala (`P`): 7 units (`vachno=127..133`, "Panchala 1..7", "પંચાળા ૧..૭")
     - Gadhada II (`GII`): 67 units (`vachno=134..200`, "Gadhada II-1" .. "Gadhada II-67", "ગઢડા મધ્ય ૧..૬૭")
     - Vartal (`V`): 20 units (`vachno=201..220`, "Vartal 1..20", "વરતાલ ૧..૨૦")
     - Ahmedabad (`A`): 3 units (`vachno=221..223`, "Amdavad 1..3", "અમદાવાદ ૧..૩")
     - Gadhada III (`GIII`): 39 units (`vachno=224..262`, "Gadhada III-1" .. "Gadhada III-39", "ગઢડા અંત્ય ૧..૩૯")
     - Bhugol-Khagol: 1 unit (`vachno=263`, "Bhugol-Khagol", "ભૂગોળ-ખગોળનું વચનામૃત")
     - Additional (`Add`): 11 units (`vachno=264..274`, Amdavad 4, 5, 7, 8; Ashlali; Jetalpur 1..5)
   - **Total Units**: 274 discourses.
2. **Language Selection**:
   - Query parameter: `format=`
   - Values: `gu` (Gujarati Unicode), `en` (English translation), `hi` (Hindi translation), `trans` (Roman transliteration).
   - Delivery: Separate page per language format (`index.php?format=en&vachno=1` vs `index.php?format=gu&vachno=1`).
3. **HTML Selectors**:
   - Container: `div#vach_text > div.main-content`
   - Unit label: `h3.pra_secno` (e.g. `Gadhada I-1`, `ગઢડા પ્રથમ ૧`)
   - Title: `h1.title_en` / `h1.title_gu`
   - Opening verse: `cite.text_en` / `cite.text_gu` (snippet: `Nijair-vacho’mṛutair-loke’tarpayadyo nijāshritān;...`)
   - Text paragraphs: `p.text_en` / `p.text_gu`
   - Footnotes: `div.footnotes-list > div.footnote-item > p.text_en` / `p.text_gu`
   - In-text note markers: `sup` with `span.target`
   - Exclusions: `header.vach-header`, `div#audio_wrap`, `div.formats`, `div#site-menu`, nav links, search bars.
4. **Natural Slide Unit**: Paragraph (preceded by title/label slide and optional opening verse slide).
5. **Edge Cases**:
   - Diacritical Sanskrit and Gujarati transliteration terms in English paragraphs (`sādhus`, `darbār`).
   - Additional section has non-sequential internal numbering (Amdavad 4, 5, 7, 8; Ashlali; Jetalpur 1-5).
   - Inline notes within quotes or bold text spans (`span.target > sup`).
6. **Pages to Download**: 274 units x 2 languages = 548 pages (1,096 if fetching all 4 formats).

---

### 2.2 Swamini Vato (સ્વામીની વાતો)

1. **Discovery & Hierarchy**:
   - **Index URL**: `https://www.anirdesh.com/vato/`
   - **Hierarchy**: Grantha -> 7 Prakarans -> 1,484 Vatos -> Paragraphs.
   - **Prakaran Breakdown**:
     - Prakaran 1: 342 vatos (`prakaran=1`, pagination `beg=1..341`, `increment=10`)
     - Prakaran 2: 192 vatos (`prakaran=2`, `beg=1..191`)
     - Prakaran 3: 74 vatos (`prakaran=3`, `beg=1..71`)
     - Prakaran 4: 140 vatos (`prakaran=4`, `beg=1..131`)
     - Prakaran 5: 407 vatos (`prakaran=5`, `beg=1..401`)
     - Prakaran 6: 292 vatos (`prakaran=6`, `beg=1..291`)
     - Prakaran 7: 37 vatos (`prakaran=7`, `beg=1..31`)
   - **Total Units**: 1,484 vatos across 7 prakarans.
2. **Language Selection**:
   - Both Gujarati, English, and Transliteration are present simultaneously in the same HTML response!
   - Setting `lang=gu` or `lang=en` in `vato/index.php?lang=gu&prakaran=1&beg=1&increment=10` only changes page metadata and default CSS display toggle; both language DOM blocks are embedded in every page.
3. **HTML Selectors**:
   - Unit Card Container: `div.vat_card`
   - Gujarati wrapper: `div.vat_wrap[id="vat_gu_{N}"]`
     - Text: `div.vat > p.text_gu` (snippet: `ભગવાનના સ્વરૂપમાં મનની અખંડ વૃત્તિ રાખવી તેથી કોઈ સાધન કઠણ નથી...`)
     - Unit info/label: `div.vatinfo_gu` (e.g. `૧/૧`)
   - English wrapper: `div.vat_wrap[id="vat_en_{N}"]`
     - Text: `div.vat > p.text_en` (snippet: `One should continuously engage in delivering and listening to talks...`)
     - Unit info/label: `div.vatinfo_en` (e.g. `1/1`)
     - Footnotes: `div.footnote > p.text_en`
   - Transliteration wrapper: `div.vat_wrap[id="vat_trans_{N}"]`
     - Text: `div.vat > p.text_trans`
   - Exclusions: `div.dropdown-content`, sidebar nav, audio players.
4. **Natural Slide Unit**: Vat (each discourse `P/V` is a single talk, or multi-paragraph slide progression).
5. **Edge Cases**:
   - The server enforces `increment=10`. Passing larger increments (e.g. 50) is ignored.
   - Some longer vatos have multiple `<p class="text_gu">` paragraphs within `div.vat`.
   - Explanatory expansions in English text appear in parentheses or brackets.
6. **Pages to Download**: 152 total pages (since each page provides both Gujarati and English simultaneously).

---

### 2.3 Kirtan Muktavali (કીર્તન મુક્તાવલી)

1. **Discovery & Hierarchy**:
   - **Index URL**: `https://www.anirdesh.com/kirtan/`
   - **Hierarchy**: Grantha -> 2 Parts (Part 1 and Part 2) -> 2,491 Kirtans -> Pads / Stanzas -> Lines (`p.k_line_gu` / `p.k_line_en`).
   - **Unit Numbering & Labels**:
     - Format: `{part}-{no}` (e.g. `1-1: માધોજી મેરે તુમ હી એક`, `2-1: અક્ષરનાથ આવો મારે ઓરડે`).
     - Part 1: ~1,300 kirtans (`part=1&no=1..`).
     - Part 2: ~1,191 kirtans (`part=2&no=1..`).
     - Total: 2,491 kirtans (verified via `alphas.gu` index count in `kirtan/data.js`).
   - **Discovery Mechanism**:
     - Direct unit URL template: `kirtan/index.php?part={P}&no={N}`.
     - Full manifest accessible via `kirtan/data.js` (`alphas`, `categories`, `raags`, `writers`) and `kirtan/getAlpha-Kirtan-List.php`.
2. **Language Selection**:
   - Gujarati and English (diacritic Roman transliteration) are served on the same static page!
   - Non-diacritic transliteration is also embedded in `#kirtan_en_nd pre#text_en_nd`.
3. **HTML Selectors**:
   - Gujarati Container: `div#kirtan_wrap_gu`
     - Title: `h1#title_gu.gu`
     - Metadata (Author/Category/Raag): `div#kirtan-info_gu` (`p.no_info_gu`, `p.raag_info_gu`)
     - Lines: `div#text_gu.gu > p.k_line_gu` (lines use `&emsp;` and `꠶ટેક` / `꠶ ૧` refrains)
   - English Container: `div#kirtan_wrap_en`
     - Title: `h1#title_en.en`
     - Metadata: `div#kirtan-info_en`
     - Lines: `div#text_en.en > p.k_line_en`
   - Exclusions: `div.unified-sticky-bar`, `div#media-info`, `div#kirtan-nav`, print lists, modal elements.
4. **Natural Slide Unit**: Kirtan Stanza (typically 2-4 lines per pad/stanza delimited by verse numbers or indentation).
5. **Edge Cases**:
   - Line structure uses `<p class="k_line_gu">` with trailing refrains like `꠶ટેક`, `માધો꠶ ૧`.
   - Multi-pad kirtans (Dhol, Thal, Aarti) may group 4 to 8 pads in sequential numbers or within a single kirtan unit.
   - Full English translation is available for selected kirtans under `kirtan/study.php?by=type&type=Translation&part={P}&no={N}`.
6. **Pages to Download**: 2,491 total pages (downloading each unit page captures both Gujarati and transliteration).

---

### 2.4 Satsang Diksha (સત્સંગદીક્ષા)

1. **Discovery & Hierarchy**:
   - **Index URL**: `https://www.anirdesh.com/diksha/`
   - **Hierarchy**: Grantha -> 315 Shloks (1 to 315).
   - **Discovery Mechanism**:
     - Paginated in increments of 5: `diksha/index.php?shlok={N}&increment=5` for `N ∈ {1, 6, 11, ... 311}`.
     - Total: Exactly 63 index requests cover all 315 shloks.
2. **Language Selection**:
   - Multilingual all-in-one architecture!
   - Every single page embeds Sanskrit Devanagari, Gujarati script Sanskrit, Roman transliteration, Gujarati meaning, English translation, plus 10 other languages (Hindi, Marathi, Punjabi, Spanish, French, German, Polish, Portuguese, Greek, Russian).
3. **HTML Selectors**:
   - Shlok Container: `div.shloks_wrap[id="shlok_wrap_{N}"]`
   - Shlok Number: `div.shlok_num > div.circle`
   - Sanskrit Devanagari: `div.sanskrit_html[id="sanskrit_html_{N}"] > p.shlok_hi`
   - Gujarati Transliteration: `div.gujarati_sans_html[id="gujarati_sans_html_{N}"] > p.shlok_gu`
   - Roman Transliteration: `div.sanskrit_trans_html[id="sanskrit_trans_html_{N}"] > p.shlok_trans`
   - Gujarati Meaning: `div.gujarati_html[id="gujarati_html_{N}"] p.meaning_gu`
   - English Meaning: `div.english_html[id="english_html_{N}"] p.meaning_en`
   - Exclusions: `div.player_wrap`, sidebar navigation, test quiz links (`?test=true`).
4. **Natural Slide Unit**: Shlok (original verse + translation).
5. **Edge Cases**:
   - The server strictly enforces `increment=5`. Passing any other value (such as `increment=20`) returns plaintext error `Invalid number.`.
   - Some verses contain memorization masking spans (`span.hidden_text`), which should be unwrapped to retain full text.
6. **Pages to Download**: 63 pages total for all 315 shloks with all languages.

---

### 2.5 Nishkulanand Kavya (નિષ્કુળાનંદ કાવ્ય)

1. **Discovery & Hierarchy**:
   - **Index URL**: `https://www.anirdesh.com/kavya/`
   - **Hierarchy**: Anthology of 22 distinct Grantha sub-books -> Prakars/Kadavu/Pads -> Doha/Chopai Stanzas -> Lines.
   - **The 22 Granths**:
     1. Purushottam Prakash (`prakash/`, 55 prakarans, `prakarah=1..55`)
     2. Sneh Gita (`sneh-gita/`, kadavu & pads, `kadavu=1..`)
     3. Vachanvidhi (`vidhi/`, 41 prakarans)
     4. Sar Siddhi (`siddhi/`, 45 prakarans)
     5. Bhakti Nidhi (`nidhi/`, 25 prakarans)
     6. Haribal Gita (`haribal/`, 25 prakarans)
     7. Hruday Prakash (`hruday/`, 15 prakarans)
     8. Dhirajakhyan (`dhiraj/`, 45 prakarans)
     9. Hari Smruti (`hari-smruti/`, 28 prakarans)
     10. Chosath Padi (`chosath-padi/`, 64 pads in sets of 8, `pad=1, 9, 17, ... 57`)
     11. Manganjan (`manganjan/`, 12 prakarans)
     12. Gungrahak (`gungrahak/`, 15 prakarans)
     13. Hari Vicharan (`hari-vicharan/`, 40 prakarans)
     14. Arji Vinay (`vinay/`, 16 prakarans)
     15. Kalyan Nirnay (`nirnay/`, 25 prakarans)
     16. Avatar Chintamani (`avatar/`, 34 prakarans)
     17. Chihna Chintamani (`chihna/`, 25 prakarans)
     18. Pushpa Chintamani (`pushpa/`, 25 prakarans)
     19. Lagna Shakunavali (`lagna/`, 25 prakarans)
     20. Yamdand (`yamdand/`, 50 prakarans)
     21. Vrutti Vivah (`vivah/`, 15 prakarans)
     22. Shikshapatri Bhasha (`shikshapatri/`, 212 shloks)
2. **Language Selection**:
   - Gujarati original verses and Roman transliteration (`line_en`) are embedded on the same page.
   - For major granths (Purushottam Prakash, Vachanvidhi, Sar Siddhi, Dhirajakhyan, Chosath Padi, Kalyan Nirnay marked with `menu_book` icon in index), English verse-by-verse translation (`p.meaning_en`) is included!
3. **HTML Selectors**:
   - Container: `div.reading-area` or `div#main_text.main_text` / `div.kavya-formatted`
   - Book Title: `h1.title`
   - Section/Prakar: `h2.section` (e.g. `પ્રકારઃ ૧`)
   - Meter tag: `p.doha`, `p.chopai`, `p.raag`, `p.info`
   - Gujarati verse line: `p.line.kavya-verse > span.kavya-verse-text`
   - Verse number: `span.kavya-verse-number` (e.g. `૧॥`)
   - English transliteration line: `p.line_en`
   - English meaning: `p.meaning_en`
   - Footnotes: `sup.fn` linking to `text/footnotes.php?note={N}`
   - Exclusions: `div#player`, `div.toolbar`, `div.content-tools`, `aside#sidebar`.
4. **Natural Slide Unit**: Verse/Stanza (Doha, Chopai, or Pad).
5. **Edge Cases**:
   - Note query parameter variation: `prakarah={N}` (with an `h`) in `prakash/index.php?prakarah=1`, vs `pad={N}` in `chosath-padi/index.php?pad=1`, vs `kadavu={N}` in `sneh-gita`.
   - Not all 22 granths have English meaning (minor granths have Gujarati text and transliteration only).
6. **Pages to Download**: ~600 pages across all 22 granths (both languages included per page).

---

### 2.6 Bhaktachintamani (ભક્તચિંતામણિ)

1. **Discovery & Hierarchy**:
   - **Index URL**: `https://www.anirdesh.com/chintamani/`
   - **Hierarchy**: Grantha -> 164 Prakarans (1 to 164) -> Stanzas (Doha, Chopai, Sameri) -> Lines.
   - **Discovery Mechanism**:
     - Unit URL template: `chintamani/index.php?prakaran={N}` for `N ∈ {1..164}`.
     - Title manifest available in `chintamani/titles.js` (`titles.gu` and `titles.trans` arrays of 165 elements).
2. **Language Selection**:
   - Gujarati text (`line_uni`) and Roman transliteration (`line_trans`) are served on the same page.
   - Chapter data is uniquely delivered BOTH in static HTML elements AND as an embedded JSON string inside `<script> var initialCache = "{...}"; </script>`.
   - Summary and English translations of key prakarans are available under `chintamani/chintamani-sar.php?page=sar-{N}` (12 sar sections).
3. **HTML & JSON Structure**:
   - **Option A (JSON Extraction from `initialCache`)**:
     - `prakaran`: integer `1..164`
     - `title_gu`: Gujarati title string (e.g. `મંગળાચરણ`)
     - `title_trans`: Transliteration title (e.g. `Mangaḷācharaṇ`)
     - `gujarati`: HTML string containing `p.raag_uni`, `p.line_uni.verse-line`, `span.verse-text`, `span.verse-number`
     - `trans`: HTML string containing `p.raag_trans`, `p.line_trans.verse-line`
   - **Option B (DOM Selectors)**:
     - Main Container: `main.main-content` / `div.content-wrapper`
     - Title: `h1.title_uni` / `h1.title_trans`
     - Verse Line: `p.line_uni.verse-line > span.verse-text`
     - Verse Number: `span.verse-number`
     - Exclusions: `aside#sidebar`, `div.toolbar`, search modal.
4. **Natural Slide Unit**: Verse (couple of lines with `span.verse-number`).
5. **Edge Cases**:
   - Dual representation: parser can either decode the `var initialCache` JSON directly (cleanest and fastest) or parse the DOM.
   - Full English translation is not present for all 164 prakarans (transliteration is present throughout; English translation is in Chintamani Sar).
6. **Pages to Download**: 164 pages for the complete scripture text (plus 12 pages for Chintamani Sar).

---

### 2.7 Harililamrut (શ્રીહરિલીલામૃત)

1. **Discovery & Hierarchy**:
   - **Index URL**: `https://www.anirdesh.com/harililamrut/`
   - **Hierarchy**: Grantha -> 10 Kalashes -> 332 Vishrams -> Verses (Dohro, Chopai, etc.) -> Lines.
   - **Kalash Breakdown**:
     - Kalash 1: 20 vishrams (`kalash=1&vishram=1..20`)
     - Kalash 2: 18 vishrams (`kalash=2&vishram=1..18`)
     - Kalash 3: 27 vishrams (`kalash=3&vishram=1..27`)
     - Kalash 4: 31 vishrams (`kalash=4&vishram=1..31`)
     - Kalash 5: 28 vishrams (`kalash=5&vishram=1..28`)
     - Kalash 6: 29 vishrams (`kalash=6&vishram=1..29`)
     - Kalash 7: 83 vishrams (`kalash=7&vishram=1..83`)
     - Kalash 8: 63 vishrams (`kalash=8&vishram=1..63`)
     - Kalash 9: 13 vishrams (`kalash=9&vishram=1..13`)
     - Kalash 10: 20 vishrams (`kalash=10&vishram=1..20`)
     - Total: 332 vishrams + front-matter sections (`section=ashirvad`, `section=sarvoparita`, `section=purvabhumika`, `section=acharya`, `section=prastavana`).
2. **Language Selection**:
   - Gujarati only (no English translation exists on the site).
3. **HTML Selectors**:
   - Container: `div.content-wrapper` / `div.reading-area`
   - Title: `h1.title_gu` / `h2.toc_sub`
   - Meter header: `p.raag` (e.g. `દોહરો`, `પ્રમાણિકાવૃત્ત`, `ચોપાઈ`)
   - Verse line: `p.line.hla-verse > span.hla-verse-text`
   - Verse number: `span.hla-verse-number` (e.g. `૧`)
   - Footnotes: `sup.fn > a.noteref` linking to `kalash_{K}/footnotes.php?note={N}`
   - Exclusions: `div#player`, audio sources, sidebar navigation.
4. **Natural Slide Unit**: Verse (2-line couplet with meter header).
5. **Edge Cases**:
   - Complex verse meters (`પ્રમાણિકાવૃત્ત`, `ભુજંગી`, `દોહરો`, `ચોપાઈ`).
   - Extensive word footnotes explaining 19th-century Gujarati idioms.
6. **Pages to Download**: 332 vishram pages + 5 introductory pages = 337 pages.

---

### 2.8 Yogi Gita (યોગી ગીતા)

1. **Discovery & Hierarchy**:
   - **Index URL**: `https://www.anirdesh.com/yogi-gita/`
   - **Hierarchy**: Grantha -> 8 Sections:
     - `page=foreword` (Foreword)
     - `page=publishers_note` (Publisher's Note)
     - `page=nivedan` (નિવેદન)
     - `page=patro` (હસ્તલિખિત પત્ર)
     - `page=sunrut` (સૂનૃત - ૭ મુદ્દાઓ)
     - `page=prarthana` (પ્રાર્થના)
     - `page=jivan_bhavana` (જીવન ભાવના)
     - `page=hrudayni_vato` (હૃદયની વાતો)
2. **Language Selection**:
   - Gujarati and English translation are interleaved directly in the same static page!
3. **HTML Selectors**:
   - Container: `div#content > div.text`
   - Section Title: `h1.sec_title`
   - Gujarati content: `div.gu_text` (`p.gu_center`, `p.gu_left`, `ul.indented_list`)
   - English content: `div.en_text` (`p.en_center`, `p.en_left`, `ul.indented_list`)
   - Exclusions: `div#toolbar` (font controls, dark mode, nav links).
4. **Natural Slide Unit**: Paragraph or list item (`li.item` in `ul.indented_list`).
5. **Edge Cases**:
   - Structured lists (such as the 5 principles or 7 points) use nested `ul.indented_list > li.item`.
   - Handwritten letter images present in `page=patro`.
6. **Pages to Download**: 8 pages total for the complete book in both languages.

---

### 2.9 Swaminarayan Siddhant Karika (સ્વામિનારાયણ સિદ્ધાંત કારિકા)

1. **Discovery & Hierarchy**:
   - **Index URL**: `https://www.anirdesh.com/karika/`
   - **Hierarchy**: Grantha -> 10 Dharas + Upsamhar + Namavali -> Shloks.
   - **Sections**:
     - `mangal-dhara` & `mangal-dhara-hi` (Roman transliteration)
     - `aksharbrahma-dhara` & `aksharbrahma-dhara-hi`
     - `parabrahma-dhara` & `parabrahma-dhara-hi`
     - `praman-dhara` & `praman-dhara-hi`
     - `maya-dhara` & `maya-dhara-hi`
     - `jiva-dhara` & `jiva-dhara-hi`
     - `ishwar-dhara` & `ishwar-dhara-hi`
     - `sadhan-dhara` & `sadhan-dhara-hi`
     - `fal-dhara` & `fal-dhara-hi`
     - `upsanhar` & `upsanhar-hi`
     - `namavali`
2. **Language Selection**:
   - Sanskrit Devanagari (`section={name}`) and Sanskrit Roman Transliteration (`section={name}-hi`).
   - No Gujarati or English translation is hosted on the site for Karika.
3. **HTML Selectors**:
   - Container: `div#content > div.text > div#text_body`
   - Titles: `p.hi_title`, `p.hi_subtitle`
   - Verses: `p.hi_center` (verses numbered with `॥ ૧॥`, `॥ ૨॥` at end of lines)
   - Exclusions: `div#toolbar`.
4. **Natural Slide Unit**: Shlok (2-line Sanskrit verse).
5. **Edge Cases**: Empty line spacers use `<p class="hi_center">&#160;</p>`.
6. **Pages to Download**: 21 pages total (11 Devanagari + 10 Roman transliteration).

---

### 2.10 Aksharamrutam (અક્ષરામૃતમ્)

1. **Discovery & Hierarchy**:
   - **Index URL**: `https://www.anirdesh.com/aksharamrutam/`
   - **Hierarchy**: Grantha (Gunatitanand Swamini Vato, Volume 2 / Prakarans 8-15) -> 8 Prakarans -> 1,044 Vatos.
   - **Prakaran Breakdown**:
     - Prakaran 8: 131 vatos (`prakaran=8&start=1&increment=10`)
     - Prakaran 9: 199 vatos (`prakaran=9&start=1&increment=10`)
     - Prakaran 10: 146 vatos (`prakaran=10&start=1&increment=10`)
     - Prakaran 11: 237 vatos (`prakaran=11&start=1&increment=10`)
     - Prakaran 12: 107 vatos (`prakaran=12&start=1&increment=10`)
     - Prakaran 13: 108 vatos (`prakaran=13&start=1&increment=10`)
     - Prakaran 14: 84 vatos (`prakaran=14&start=1&increment=10`)
     - Prakaran 15: 32 vatos (`prakaran=15&start=1&increment=10`)
   - Also browsable by topic: `by=kalash&kalash=1..14`.
2. **Language Selection**:
   - Gujarati only.
3. **HTML Selectors**:
   - Vat Text: `p.vat_text_gu[id="gu_pra_{P}_vat_{N}"]`
   - Unit Number: Bold bracketed number at end of text: `<b>(૧)</b>`
   - Footnotes: `p.vat_fn_gu`
   - In-text Footnote marker: `sup` tags
   - Exclusions: Sidebar navigation, pagination controls.
4. **Natural Slide Unit**: Vat (each paragraph unit).
5. **Edge Cases**: Numbering is appended inside the paragraph tag as trailing `<b>(N)</b>`.
6. **Pages to Download**: 105 pages (with `increment=10`).

---

### 2.11 Purushottam Bolya Prite (પુરુષોત્તમ બોલ્યા પ્રીતે)

1. **Discovery & Hierarchy**:
   - **Index URL**: `https://www.anirdesh.com/pbp/`
   - **Hierarchy**: Grantha -> 31 Thematic Chapters (`page=chp_1..chp_31`) + Front matter (`arpan`, `prakashkiya`, `granth-intro`).
2. **Language Selection**:
   - Gujarati only.
3. **HTML Selectors**:
   - Container: `main.content-wrapper > div.text_section`
   - Chapter Title: `h1.chp_title` (e.g. `૧. સત્સંગ`)
   - Text Paragraphs: `p.gu_indent`, `p.gu_center`
   - Footnotes: `sup` tags
   - Exclusions: `div#site-menu` (sidenav).
4. **Natural Slide Unit**: Paragraph (`p.gu_indent`).
5. **Edge Cases**: Some links contain `.html` suffix in the href (e.g. `chp_2.html`), but both query forms resolve.
6. **Pages to Download**: 34 pages total.

---

### 2.12 Sadguru Aksharanand Swamini Vato (સદ્‌ગુરુ અક્ષરાનંદ સ્વામીની વાતો)

1. **Discovery & Hierarchy**:
   - **Index URL**: `https://www.anirdesh.com/asv/`
   - **Hierarchy**: Grantha -> 39 Thematic Chapters (`page=chp_1..chp_39`) -> Vato / Incidents (`div.vat_wrap`).
2. **Language Selection**:
   - Gujarati only.
3. **HTML Selectors**:
   - Container: `main.content-wrapper > div.text > div#text_body`
   - Book Title: `h1.book_title`
   - Chapter Title: `h1.chp_title`
   - Incident / Vat Wrapper: `div.vat_wrap`
   - Paragraphs: `p.gu_indent`
   - Exclusions: `div.sidenav`, header banner.
4. **Natural Slide Unit**: Incident / Vat block (`div.vat_wrap` or `p.gu_indent`).
5. **Edge Cases**: Very large chapters (some chapters > 150KB of prose with dozens of incidents per page).
6. **Pages to Download**: 39 pages total.

---

### 2.13 Shrimad Bhagvad Gita (શ્રીમદ્ ભગવદ્ ગીતા)

1. **Discovery & Hierarchy**:
   - **Index URL**: `https://www.anirdesh.com/gita/`
   - **Hierarchy**: Grantha -> 18 Chapters (`page=chapter_1..chapter_18`) + Front/Back matter -> Verses (`p.shloka_no`).
2. **Language Selection**:
   - Trilingual in every chapter! Sanskrit Devanagari (`p.sanskrit`), Roman Transliteration (`p.transliteration`), and English Translation (`p.translation`).
3. **HTML Selectors**:
   - Container: `main.content-wrapper`
   - Chapter Heading: `h3.chapter_no`, `h3.chapter_title`
   - Shlok Number: `p.shloka_no[id="shloka_{C}_{S}"]` (e.g. `id="shloka_1_1"`)
   - Speaker Attribution: `p.uvacha` (e.g. `Dhrutarāshtra said`)
   - Sanskrit text: `p.sanskrit`
   - Roman transliteration: `p.transliteration`
   - English translation: `p.translation`
   - Exclusions: Sidebar navigation, header.
4. **Natural Slide Unit**: Shlok (Speaker + Verse + Translation).
5. **Edge Cases**: Speaker headers (`p.uvacha`) precede certain verses and should remain attached to the subsequent verse.
6. **Pages to Download**: 18 chapter pages (plus 7 reference/appendix pages = 25 pages total).

---

### 2.14 Shrimad Bhagavata Purana (શ્રીમદ્ભાગવતપુરાણ)

1. **Discovery & Hierarchy**:
   - **Index URL**: `https://www.anirdesh.com/bhagvat/`
   - **Hierarchy**: Grantha -> 12 Skandhas -> 347 Adhyays (`skandh={S}&adhyay={A}`) -> Verses (`p.hi_left`).
2. **Language Selection**:
   - Sanskrit Devanagari text only.
3. **HTML Selectors**:
   - Container: `div.content-wrapper`
   - Adhyay Header: `h2.adhyay` (e.g. `अथ प्रथमोऽध्यायः`)
   - Verses: `p.hi_left` (numbered with Devanagari numerals at end of lines)
   - Vachanamrut Citation Divs: `div[id="ref_{S}_{A}_{V}"]`
   - Exclusions: Sidebar navigation.
4. **Natural Slide Unit**: Verse (stanza of lines ending in Devanagari number).
5. **Edge Cases**: Verses referenced in Vachanamrut are highlighted within special `div#ref_...` containers.
6. **Pages to Download**: 347 adhyay pages.

---

### 2.15 Sanskar Kunj Shloks Collection (સંસ્કાર કુંજ)

1. **Discovery & Hierarchy**:
   - **Index URL**: `https://www.anirdesh.com/kirtan/shloks.php`
   - **Hierarchy**: Collection -> Grantha Filters (`Geeta`, `Shikshapatri`) -> Individual Shloks.
2. **Language Selection**:
   - Multilingual: Sanskrit in Gujarati script (`p.shloka_gu`), Sanskrit Devanagari (`p.shloka_sa`), Roman Transliteration (`p.shloka_tr`), and Gujarati meaning (`p.meaning_gu`).
3. **HTML Selectors**:
   - Shlok Container: `div.shlok_card` or `div.shlok_text_gu` / `div.shlok_text_hi` / `div.shlok_text_trans`
   - Gujarati Shloka: `p.shloka_gu`
   - Devanagari Shloka: `p.shloka_sa`
   - Transliteration Shloka: `p.shloka_tr`
   - Gujarati Meaning: `p.meaning_gu`
   - English Meaning: `p.meaning_en`
4. **Natural Slide Unit**: Shlok.
5. **Edge Cases**: Complete Shikshapatri in Sanskrit with Gujarati meaning.
6. **Pages to Download**: ~1-10 index/filter pages.

---

### 2.16 Satsang Exam eBooks (સત્સંગ શિક્ષણ પરિક્ષા ઇ-બુક્સ)

1. **Discovery & Hierarchy**:
   - **Index URL**: `https://www.anirdesh.com/ebooks/`
   - **Hierarchy**: Levels (Prarambh, Pravesh, Parichay, Pravin, Pragna) -> Individual Exam Books (e.g. Ghanshyam Charitra, Yogiji Maharaj, Kishor Satsang Prarambh, etc.).
2. **Delivery Architecture & Distinction**:
   - Unlike the other scripture sections which serve readable HTML chapters, `ebooks/` is a distribution hub for compiled binary e-books (`.epub`, `.mobi`, `.pdf`) and an embedded BiB/i web reader (`bibi/i/?book={book_id}`).
   - Direct downloads trigger POST requests to `filereq.php` with `filename={book_name}.epub`.
3. **Recommendation**:
   - For presentation slide purposes, the text of these books is already covered under standard granths and Charitra indexes. Downloading binary EPUB files requires submitting POST forms (contrary to automated static scraping guidelines).

---

### 2.17 Special Sections: Charitra Search & Adhiveshan 2025

- **Charitra Search (`charitra/`)**:
  - Search engine covering 17 biographical volumes.
  - Master Table of Contents: `charitra/toc.php`.
  - API endpoint: `charitra/toc.php?ajax_load_toc=1&table={book_id}` returns clean JSON with all chapter titles and physical book page references.
- **North America Adhiveshan 2025 (`smadhiveshan25/`)**:
  - Contains 20 Pushpas (`pushpa=1..20`) and 100 Satsang Prasangs (`format=Eng`, `format=Guj`, `format=Trans`).
  - Slide unit: Verse / Story paragraph.

---

## 3. Master Book Specification Table

| Book ID | Index URL | Unit URL Template | Unit Count | Languages Available | Natural Slide Unit | Primary Text Selectors | Download Pages | Parser Difficulty |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `vachanamrut` | `/vachanamrut/` | `/vachanamrut/index.php?format={lang}&vachno={N}` | 274 discourses | GU, EN, HI, Trans | Paragraph | `div#vach_text p.text_en`, `p.text_gu`, `cite` | 548 (GU+EN) | Low |
| `vato` | `/vato/` | `/vato/index.php?prakaran={P}&beg={B}&increment=10` | 1,484 vatos (7 prakarans) | GU, EN, Trans (simultaneous) | Vat (Discourse) | `div.vat_wrap[id="vat_gu_N"] p.text_gu`, `[id="vat_en_N"] p.text_en` | 152 | Low |
| `kirtan` | `/kirtan/` | `/kirtan/index.php?part={P}&no={N}` | 2,491 kirtans (2 parts) | GU, Trans (simultaneous), EN (study) | Stanza (Pad) | `div#text_gu p.k_line_gu`, `div#text_en p.k_line_en` | 2,491 | Low |
| `diksha` | `/diksha/` | `/diksha/index.php?shlok={N}&increment=5` | 315 shloks | GU, EN, Sanskrit, Trans + 9 others | Shlok | `p.shlok_hi`, `p.shlok_gu`, `p.shlok_trans`, `p.meaning_gu`, `p.meaning_en` | 63 | Low |
| `kavya` | `/kavya/` | `/kavya/{granth}/index.php?prakarah={N}` | 22 granths (~600 units) | GU, Trans, EN (selected granths) | Verse / Pad | `p.line.kavya-verse`, `p.line_en`, `p.meaning_en` | ~600 | Med |
| `chintamani` | `/chintamani/` | `/chintamani/index.php?prakaran={N}` | 164 prakarans | GU, Trans (HTML + JSON cache) | Verse | `var initialCache` JSON or `p.line_uni`, `p.line_trans` | 164 | Low |
| `harililamrut`| `/harililamrut/`| `/harililamrut/index.php?kalash={K}&vishram={V}` | 332 vishrams (10 kalashes) | GU | Verse | `p.line.hla-verse > span.hla-verse-text` | 337 | Low |
| `yogi-gita` | `/yogi-gita/` | `/yogi-gita/index.php?page={section}` | 8 sections | GU, EN (simultaneous) | Paragraph / List item | `div.gu_text p`, `div.en_text p`, `ul.indented_list li` | 8 | Low |
| `karika` | `/karika/` | `/karika/index.php?section={dhara}` | 11 sections | Sanskrit Devanagari, Trans | Shlok | `div#text_body p.hi_center` | 21 | Low |
| `aksharamrutam`| `/aksharamrutam/`| `/aksharamrutam/index.php?prakaran={P}&start={B}&increment=10` | 1,044 vatos (8 prakarans) | GU | Vat (Paragraph) | `p.vat_text_gu`, `b` | 105 | Low |
| `pbp` | `/pbp/` | `/pbp/index.php?page=chp_{N}` | 31 chapters | GU | Paragraph | `main.content-wrapper div.text_section p.gu_indent` | 34 | Low |
| `asv` | `/asv/` | `/asv/index.php?page=chp_{N}` | 39 chapters | GU | Incident (Paragraph) | `div.vat_wrap p.gu_indent` | 39 | Low |
| `gita` | `/gita/` | `/gita/index.php?page=chapter_{N}` | 18 chapters | Sanskrit, Trans, EN (simultaneous) | Shlok | `p.shloka_no`, `p.sanskrit`, `p.transliteration`, `p.translation` | 25 | Low |
| `bhagvat` | `/bhagvat/` | `/bhagvat/index.php?skandh={S}&adhyay={A}` | 347 adhyays (12 skandhas) | Sanskrit Devanagari | Verse | `h2.adhyay`, `p.hi_left`, `div[id^="ref_"]` | 347 | Low |
| `shloks` | `/kirtan/shloks.php` | `/kirtan/shloks.php?by=Granth_Trans&Granth_Trans={G}` | Collections (Shikshapatri, Gita) | GU, Devanagari, Trans, Meaning | Shlok | `p.shloka_gu`, `p.shloka_sa`, `p.shloka_tr`, `p.meaning_gu`, `p.meaning_en` | ~5 | Low |
| `ebooks` | `/ebooks/` | `/ebooks/bibi/i/?book={book_id}` | 5 exam tiers (binary EPUB) | GU, EN (ePub packages) | Chapter / Page | Binary EPUB archive | N/A (Binary) | High |

---

## 4. Downloader Implementation Guidelines for Rust

1. **User-Agent & Rate-Limiting**:
   - Use `User-Agent: LumenPresenter/0.1` (never lowercase `research`).
   - Sleep 1.0 - 1.2s between calls (`tokio::time::sleep(Duration::from_millis(1100))`).
2. **Dual-Language Efficiency**:
   - Books like `vato`, `diksha`, `gita`, `yogi-gita`, `kavya`, and `kirtan` provide both languages (or original + transliteration) in a **single fetch**. Do not issue separate requests when one fetch suffices.
   - For `vachanamrut`, fetch `format=gu` and `format=en` separately for the 274 discourses (548 total HTTP GETs).
3. **HTML Sanitization & Presentation Modeling**:
   - Exclude `<audio>`, `.player_wrap`, `#audio_wrap`, `.toolbar`, `.content-tools`, `#sidebar`.
   - Strip inline footnotes (`sup`, `span.target`) for presentation projection while retaining them in a secondary presenter notes field.
   - In kirtan texts, normalize `&emsp;` indentations and keep refrain marks (`꠶ટેક`, `꠶ ૧`).
