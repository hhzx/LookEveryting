# Bundled fonts

Place a CJK font here for portable builds (optional if building on Windows with system fonts):

- `NotoSansSC-Regular.otf` (recommended, [OFL license](https://fonts.google.com/noto/specimen/Noto+Sans+SC))
- or `NotoSansSC-Regular.ttf`

The build script copies the first available font from:

1. `assets/fonts/`
2. `%WINDIR%\Fonts\msyh.ttc`
3. `%WINDIR%\Fonts\simhei.ttf`

into `dist/LookEveryting/fonts/` at package time.
