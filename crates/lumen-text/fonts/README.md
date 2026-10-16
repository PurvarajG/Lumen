# Bundled fonts

## Poppins

The six static Poppins faces and `Poppins-OFL.txt` come from the official
`google/fonts` repository, `ofl/poppins/`, at commit
[`1d0e46c7933245c8180fe83e47917961c913bbdb`](https://github.com/google/fonts/tree/1d0e46c7933245c8180fe83e47917961c913bbdb/ofl/poppins).
They are licensed under the SIL Open Font License 1.1. Each upstream file is
downloaded from `https://raw.githubusercontent.com/google/fonts/1d0e46c7933245c8180fe83e47917961c913bbdb/ofl/poppins/`.

## Noto Sans Gujarati

At the same google/fonts commit, `ofl/notosansgujarati/` contains a variable
font rather than separate static Regular and Bold files. The bundled static
faces were instantiated from
[`NotoSansGujarati[wdth,wght].ttf`](https://raw.githubusercontent.com/google/fonts/1d0e46c7933245c8180fe83e47917961c913bbdb/ofl/notosansgujarati/NotoSansGujarati%5Bwdth%2Cwght%5D.ttf)
at `wdth=100`, `wght=400` and `wght=700` using FontTools 4.66.0. The source
file SHA-256 is `9901d8552f1dd5d2c50dbd4caa6f6e174e74e8264f06594ab259ae6e7b1ac428`.
The instances and `NotoSansGujarati-OFL.txt` are licensed under SIL OFL 1.1.

## File SHA-256

| File | SHA-256 |
|---|---|
| `NotoSansGujarati-Bold.ttf` | `8325d19bd4bca11cd5a9f1e9deb9ae53060aa57381319d0251280b51fc8a6066` |
| `NotoSansGujarati-OFL.txt` | `c0b88977aa18b5e4fd05d646d560da89fde61b6581fa4507cb00dd90bd1bf7d4` |
| `NotoSansGujarati-Regular.ttf` | `2f15f536539a44a3d6f0a21ad4c453f6e513a52c39ead02ec18d3cb01b808bca` |
| `Poppins-Bold.ttf` | `983676516167748b74de6f4771fb384c664fd913acb8b471122ecacf5da5ea6c` |
| `Poppins-BoldItalic.ttf` | `3572ac8116a0ac7317d342262b29937bcbaf94d8f03f90df6fe666fa7e2fb43a` |
| `Poppins-Italic.ttf` | `4fa76ae75b40f926420514044722cb97f32186cafd3b38263cc34dad7174d46d` |
| `Poppins-Medium.ttf` | `90373e7d838d32468438fc3e152dca0bdb12edcab99ea639f158790b1ba1fd05` |
| `Poppins-OFL.txt` | `6be04893d770899a015649c7aa3b582f871b272f8747a92b78b17c3e5c8b2573` |
| `Poppins-Regular.ttf` | `7e65201e9b79159e2300267cc885e16c8dcef2424cdfa09a29bfb0980a94a7ba` |
| `Poppins-SemiBold.ttf` | `d3bf1bdaf0550e83da9ac0b1d1d9fe6db086835a83aa28578e609a394b9a0286` |

ChitraUni is commercial and is not bundled. At runtime, only filenames
containing `chitra` in `~/Library/Fonts` and `/Library/Fonts` are considered;
files over 8 MiB and faces whose font family name is not `ChitraUni` are
ignored. Without that font, Gujarati uses the bundled Noto Sans Gujarati faces.
