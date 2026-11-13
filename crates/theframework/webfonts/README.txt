Browser fallback font source

Unmodified Noto Sans CJK Regular collection, release Sans2.004.
https://github.com/notofonts/noto-cjk/tree/Sans2.004
https://raw.githubusercontent.com/notofonts/noto-cjk/Sans2.004/Sans/OTC/NotoSansCJK-Regular.ttc
License: SIL Open Font License 1.1; see NotoSansCJK-LICENSE.txt and font metadata.
Collection SHA-256: b76b0433203017ca80401b2ee0dd69350349871c4b19d504c34dbdd80541690a
License SHA-256: 6a73f9541c2de74158c0e7cf6b0a58ef774f5a780bf191f2d7ec9cc53efe2bf2

The collection retains all ten regional/monospace faces. Browser presentation
registers these once per text owner from a shared immutable CPU source and uses
script fallback after the existing UI/code fonts. Host language selects the
default Han face; kana, Hangul and Bopomofo retain their regional fallback faces.
Native production builds retain system discovery and do not embed this file.
Text-only native tests include it to verify fallback without system fonts.
