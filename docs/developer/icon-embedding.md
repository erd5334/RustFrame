# Icon Embedding

RustFrame relies on Tauri bundling to embed icons into app bundles and installers.

## Configuration
- Icons are listed in tauri.conf.json under bundle.icon.
- build.rs calls tauri_build::build(), which performs bundling.

## Notes
- The icons/ directory is required for building.
- End users do not need icons/ at runtime because the icons are embedded in the bundle/executable.
