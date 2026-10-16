# R2: anirdesh.com book structure research (read-only web)

Goal: a precise map so a later task can write a Rust downloader/parser for every book on https://www.anirdesh.com, keeping Gujarati and English text. The user chose in-app download for personal use; no text is bundled or committed. Output `docs/research/anirdesh-structure.md` in your worktree (commit it). Save sample HTML pages under `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/research/anirdesh/` (never in git).

Rules: use `curl -sL --max-time 20 -A "LumenPresenter-research/0.1"`; at most 1 request per second (`sleep 1` between); at most 150 requests total; respect robots.txt (Disallow /img /scripts /admin). Do not submit forms or log in. Do not paste long passages of scripture text into the doc — short (≤ 15 words) snippets only to identify fields.

For EACH book linked from the homepage nav and the site's book sections (Vachanamrut, Swamini Vato, Kirtan Muktavali, Satsang Diksha, Nishkulanand Kavya, Bhaktachintamani, Harililamrut, Yogi Gita, and any other scripture books you find such as bhagvat/, aksharamrutam/, karika/, gita/, ebooks/), record:

1. Index URL and how the list of units is discovered (e.g. `vachanamrut/index.php?format=en&vachno=N`), total unit count, and hierarchy (sections → units → paragraphs/verses/stanzas). Give the exact numbering scheme and human labels (e.g. "Gadhada I-1").
2. Language selection: which `format=` (or other) values exist; whether Gujarati and English are on the same page or separate; encoding (UTF-8?).
3. HTML structure for the text body: the stable element/class/id selectors for title, unit label, paragraph/verse/stanza blocks, footnotes, and things to exclude (nav, ads, footers, audio players, commentary vs original text). Note any JavaScript-loaded content (if text is not in the static HTML, say how it loads).
4. Natural slide unit for presenting: paragraph, verse/shlok, kirtan stanza, vat (Swamini Vato), etc.
5. Edge cases: missing English, mixed scripts, tables, inline numbering, `<br>` line structure in kirtans.
6. Approximate total pages needed to download the whole book in both languages.

End with a per-book table: book id, index URL, unit URL template, unit count, languages, slide unit, selectors, pages to download, and parser difficulty (low/med/high). Final reply: 10 lines max.
