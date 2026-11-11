<p align="center">
  <a href="https://rbxport.com">
    <img src="docs/assets/brand/combined/rbxport-logo-color-white.png" alt="rbxport" width="360">
  </a>
</p>

# rbxport

rbxport is a music library management app inspired by rekordbox. It keeps its
feature set deliberately small, aiming for a faster, simpler user experience.

Its scope is limited to library management, USB exporting, and PRO DJ LINK.

## Download this fork (Apple Silicon)

Download the latest [Apple Silicon DMG](https://github.com/einaralex/rbxporteng/releases/latest/download/rbxport-apple-silicon.dmg) from the [fork's GitHub Releases](https://github.com/einaralex/rbxporteng/releases/latest). Open it and drag rbxport to Applications. Every merge to this fork's `main` builds a new DMG and makes it the latest GitHub release. The app links to that release for updates; it does not install them automatically.

The DMG is currently unsigned and unnotarized. If macOS blocks its first launch, open System Settings → Privacy & Security and choose **Open Anyway** after trying to open it. [Apple explains this step](https://support.apple.com/en-ie/102445). Engine DJ must be installed to use Engine USB preparation; other rbxport features do not require it.

## Engine DJ preparation in this fork

Enable **Preferences → USB Export → Prepare USB for Engine DJ after export**.
After a successful, verified Rekordbox export, this fork runs
[`rekordbox-to-engine`](https://github.com/einaralex/rb-engine) on that USB. This applies
to Sync Manager, playlist/folder export, and track export. Automatic eject
waits for Engine preparation to succeed. The option is off by default.

Install Engine DJ on macOS. The app includes the Rust preparation code and
its SQLCipher/SQLite dependencies in the macOS build. Leave the external
helper field blank to use that built-in integration. No separate helper or SQL
command-line tools are required. The field can still point to a compatible
external helper executable for development.

The preparation workflow requires a modern OneLibrary export. A repeat export
can refresh an Engine Library created by this helper if its main database has
not changed. Unknown or modified Engine libraries are left alone unless you
confirm replacement in rbxport; Engine-side edits are not merged. The external helper override
still requires a USB without an Engine Library. The app checks for Engine DJ
and whether the library can be refreshed before it writes the Rekordbox export.
If analysis fails, the previous Engine Library directory remains in place. The
Rekordbox export has already completed, so audio removed by that export may no
longer be available to the previous Engine library. The app reports the error
and prevents automatic eject. Preparation is shown as a
separate busy stage; cancellation is unavailable during that stage.

The integration tests use disposable directories. The ignored
`bundled_engine_prepares_a_disposable_rekordbox_export` test also runs the
installed OfflineAnalyzer on generated click tracks across three exports: first,
added-track, and removed-track. It passed locally with
Engine DJ Desktop 5.1.0.2acfe893c4. These checks do not establish playback
compatibility on Engine DJ hardware.

## Project vision

The core vision of rbxport is **fewer features**.

Because rbxport is open source, you are free to fork it and build a version of
rekordbox customized by the community. The core project does not aim to be that.
A project that keeps adding features eventually becomes what rekordbox already is: full-featured
DJ software. rbxport stays small on purpose. Any contribution that adds a feature
is weighed carefully against this vision, and a good feature can still be
declined.

Two commitments define what "small" must not cost:

- **Compatibility.** rbxport aims to be fully compatible with the current
  version of rekordbox and all the hardware that rekordbox supports.
- **Performance.** rbxport maintains its speed through a performance budget,
  defined in `perf-budgets.json` and enforced by `pnpm budget` and CI. See
  [Development conventions](docs/development/conventions.md).

If you want to propose a feature, read
[Feature proposals](CONTRIBUTING.md#feature-proposals) in the contributing guide
first.

## Tech stack

| Layer | Technology |
| --- | --- |
| Desktop app | Tauri 2. |
| Frontend | React and TypeScript, with TanStack Virtual for track browsing. |
| Backend | Rust, with independent `rbl-*` crates for library, audio, export, and LINK logic. |
| Database | SQLCipher for rekordbox libraries and OneLibrary USB exports. |
| Build tooling | Vite, pnpm, and Cargo. |
| Testing | Vitest, Playwright, and Rust tests with temporary library fixtures. |

See [Architecture](docs/development/architecture.md) for the repository layout
and how the frontend, desktop shell, and Rust crates fit together.

## Start here

For a first look at the interface, use the browser with a mock library:

```sh
pnpm install
pnpm dev:web
```

This requires Node 24 and pnpm 10.17.1. It does not open an installed rekordbox
library or require a Rust build. For desktop prerequisites and safe library
setup, read [Getting started](docs/development/getting-started.md).

## Developer reading path

Read these in order to understand the project and make your first change:

1. [Getting started](docs/development/getting-started.md): prerequisites, run modes, and library safety.
2. [Architecture](docs/development/architecture.md): repository map, crate responsibilities, IPC, and edit flow.
3. [Development conventions](docs/development/conventions.md): code style, performance, translation, and evidence rules.
4. [Testing](docs/development/testing.md): checks, test setup, and what each result proves.
5. [Contributing](CONTRIBUTING.md): issues, branches, implementation, validation, and review.
6. [Debugging](docs/development/debugging.md): logs, diagnostics, environment variables, and cleanup.
7. [Releases](docs/development/releases.md): versions, CI, packaging, and publication.

The [documentation index](docs/README.md) also links to user guides and detailed
analysis, USB-format, and hardware references.

## Common commands

| Command | Purpose |
| --- | --- |
| `pnpm dev:web` | Browser UI with the mock backend. |
| `pnpm dev` | Desktop app; can open your installed library. Read the safety guide first. |
| `pnpm lint` | Frontend lint rules. |
| `pnpm build` | TypeScript check and production frontend bundle. |
| `pnpm test` | Vitest tests. |
| `pnpm e2e` | Playwright browser tests against the mock backend. |
| `RB_LITE_TEST=1 cargo test --workspace` | Rust tests with installed-library writes refused. |

## License and trademarks

rbxport is GPL-2.0-or-later. See [LICENSE](LICENSE) and
[Licensing](LICENSING.md) for bundled components and distribution terms.
Third-party product names belong to their respective owners. rbxport is an
independent project and is not affiliated with, endorsed by, or sponsored by
their owners.

## Disclaimer

rbxport is provided "as is", without warranty to the extent permitted by
applicable law. Use it at your own risk and keep backups of your music library
and USB drives. Unless required by
applicable law or agreed to in writing, the authors and contributors are not
liable for damages arising from using or being unable to use rbxport,
including lost or corrupted data, equipment damage, or financial losses.
See [LICENSE](LICENSE) for the full warranty and liability terms.

rbxport is an independent project and is not affiliated with, endorsed by,
or sponsored by AlphaTheta or Pioneer DJ. References to rekordbox,
PRO DJ LINK, and other products describe compatibility only. All trademarks
and product names belong to their respective owners.

## Special thanks

evanpurkhiser, Maddix, Morgan Page, nichi, profbx, Sean Tyas, shiz, syl, trancejesus, xorbxbx, and
AlphaTheta.
