# Third-Party Notices

Z_COM includes and links to third-party open-source software. Those components
remain subject to their respective licenses; the Apache License 2.0 for Z_COM
does not replace or modify those licenses.

Exact resolved versions are recorded in `package-lock.json` and
`src-tauri/Cargo.lock`. Source code and license texts for Rust packages are
available from [crates.io](https://crates.io/) and their linked repositories;
source code and license texts for JavaScript packages are available from
[npm](https://www.npmjs.com/) and their linked repositories.

## Principal Runtime Components

| Component | License |
| --- | --- |
| Tauri and Tauri plugins | Apache-2.0 OR MIT |
| Svelte and SvelteKit | MIT |
| Lucide Svelte | ISC |
| probe-rs | Apache-2.0 OR MIT |
| serialport | MPL-2.0 |
| reqwest | Apache-2.0 OR MIT |
| serde and serde_json | Apache-2.0 OR MIT |
| chrono | Apache-2.0 OR MIT |
| parking_lot | Apache-2.0 OR MIT |
| libloading | ISC |
| if-addrs | MIT OR BSD-3-Clause |

The resolved dependency graph also contains software under permissive
licenses including Apache-2.0, MIT, MIT-0, 0BSD, BSD-2-Clause, BSD-3-Clause,
ISC, Zlib, BSL-1.0, CC0-1.0, Unlicense, Unicode-3.0,
CDLA-Permissive-2.0 and Apache-2.0 WITH LLVM-exception.

Some resolved Rust components, including `serialport`, `cssparser`,
`cssparser-macros`, `dtoa-short`, `option-ext` and `selectors`, are available
under MPL-2.0. Z_COM does not modify or relicense those components. Their
corresponding Source form can be obtained using the exact versions in
`src-tauri/Cargo.lock` from crates.io and the upstream repositories identified
in each package's Cargo metadata.

Dependencies that offer multiple licenses are used under a permissive option
where available. In particular, `unescaper` is used under MIT rather than
GPL-3.0-only, and `r-efi` is used under MIT or Apache-2.0 rather than LGPL.

SEGGER J-Link libraries are optional proprietary runtime dependencies supplied
and installed separately by the user. They are not included in Z_COM source or
binary distributions and remain subject to SEGGER's own license terms.
