# PrintCaddy

PrintCaddy is a browser CAD app for 3D printing. This repository is the Rust and WebAssembly rewrite. The current TypeScript app stays live at [rrkennedy69.github.io](https://rrkennedy69.github.io/) until this one reaches parity.

The preview of this rewrite is [rrkennedy69.github.io/printcaddy-rs/](https://rrkennedy69.github.io/printcaddy-rs/).

## What runs today

A 3D viewport with orbit, pan, and zoom on mouse and touch, a 256 mm build plate grid, box and cylinder primitives in the default slate `#8A98A6`, selection and moving on the plate, and STL import and export.

The feature inventory and the milestone plan are in [docs/FEATURES.md](docs/FEATURES.md) and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Build

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
cargo test -p printcaddy-core
cd crates/printcaddy-app
trunk serve
```

GitHub Pages is built with `trunk build --release --public-url /printcaddy-rs/`.

The workflow publishes on every push to `main`. GitHub's workflow token is not allowed to create the Pages site, so the first publish needs one setting: repository **Settings → Pages → Build and deployment → Source → GitHub Actions**. After that, the same workflow deploys on its own. The preview is [rrkennedy69.github.io/printcaddy-rs/](https://rrkennedy69.github.io/printcaddy-rs/).
