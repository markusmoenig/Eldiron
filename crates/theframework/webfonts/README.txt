Browser fallback font source

Unmodified Noto Sans CJK Regular collection, release Sans2.004.
https://github.com/notofonts/noto-cjk/tree/Sans2.004
https://raw.githubusercontent.com/notofonts/noto-cjk/Sans2.004/Sans/OTC/NotoSansCJK-Regular.ttc
License: SIL Open Font License 1.1; see NotoSansCJK-LICENSE.txt and font metadata.
Collection SHA-256: b76b0433203017ca80401b2ee0dd69350349871c4b19d504c34dbdd80541690a
License SHA-256: 6a73f9541c2de74158c0e7cf6b0a58ef774f5a780bf191f2d7ec9cc53efe2bf2

The first 9 MiB is stored here as NotoSansCJK-Regular.ttc.part1. The rest is
in the eldiron-cjk-font-data crate as NotoSansCJK-Regular.ttc.part2.
TheFramework joins the parts at compile time. Their concatenation is the
original collection and matches the collection SHA-256 above.

The collection retains all ten regional/monospace faces. Browser presentation
registers these once per text owner from a shared immutable CPU source and uses
script fallback after the existing UI/code fonts. Host language selects the
default Han face; kana, Hangul and Bopomofo retain their regional fallback faces.
Native production builds retain system discovery and do not embed this file.
Text-only native tests include it to verify fallback without system fonts.

Unmodified Noto Sans Arabic Regular hinted TTF, release NotoSansArabic-v2.013.
https://github.com/notofonts/arabic/releases/tag/NotoSansArabic-v2.013
Source commit: 1b2b7e5c6ce3ab4d50681c854892325530084c35
https://github.com/notofonts/arabic/releases/download/NotoSansArabic-v2.013/NotoSansArabic-v2.013.zip
Archive SHA-256: 1301aceaea84c501cf2e6dcfb3182e2328c8eae5725817fcb239672bda7154f1
Extracted member: NotoSansArabic/hinted/ttf/NotoSansArabic-Regular.ttf
License: SIL Open Font License 1.1; see NotoSansArabic-LICENSE.txt.
NotoSansArabic-Regular.ttf SHA-256: bdff3e5659d67e67def05b33f749683b9376ae819d65d3dd62ac4640b3aaef48
NotoSansArabic-LICENSE.txt SHA-256: a7a5a25eb188bf1cd96982030d53e23c33485c69b1044a562254226857ee13af
Browser Arabic-script fallback uses the same shared immutable source policy.
It covers real Arabic shaping in authored content without installed system fonts;
Latin UI/code primaries and native system-font discovery remain unchanged.
