# papers-review

A desktop app to review and edit papers: a project folder with the
manuscript, its versions and the material that comes with it, the PDF shown
as Preview shows it, and a review in Markdown that floats over it when
called.

Everything is plain files on your computer (synced by whatever service you
use): the PDFs, a `review.md` (or as many Markdown files as you like) and a
small `review.toml`. Any other app opens them.

Website and downloads: <https://mathesislab.github.io/papers-review/>.

Part of the papers family:
[papers-cli](https://github.com/perrette/papers),
[Papers Desktop](https://github.com/mathesislab/papers-desktop),
[papers-rs](https://github.com/mathesislab/papers-rs).

**State**: the first steps. A window per project, or one switched between projects: its files, the files open in
tabs (a page each, as in Papers Write), the PDF in PDFKit (page, zoom, find), the colour bar; a start window.
The plans are in [docs/design.md](docs/design.md), the mock-ups in
[docs/mockups/review.html](docs/mockups/review.html).

## Develop

```bash
npm install
npm run tauri dev
```

Rust (stable) and Node.js. macOS first: the PDF view is PDFKit
([tauri-plugin-pdfkit](https://github.com/mathesislab/tauri-plugin-pdfkit)).

## License

MIT.
