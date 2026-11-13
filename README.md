# Zen Desktop App — Tauri + Rust (scaffold)

**Status:** scaffold ready. Installers can be built the moment the server is
live — but three things must happen first (see "What is needed next"):
a GitHub repo, an updater signing keypair, and the hosted backend URL.

**Dates (decided Nov 13, 2026):**
- **Nov 14, 2026 — pre-load:** people download the Zen app early. The app
  shows a countdown; it unlocks for the Beta on Nov 20.
- **Nov 20, 2026 — Beta:** the app loads the hosted Zen app. Same day as
  the GTA VI Story Mode Marathon (live 5:00 PM AST).
- **Dec 25, 2026 — full release.**

**Decision (Brodie, Nov 12):** Tauri + Rust. **Signing (Brodie, Nov 13):**
no paid code-signing certs — ship unsigned, direct download only.
Mac shows a Gatekeeper warning and Windows shows a SmartScreen warning on
first open; both are still installable.

## What is ready (this folder)

```
desktop-app/
  README.md                     <- this file
  src/                          <- bundled Zen frontend snapshot (6.4 MB)
    index.html                  <- Zen UI (client/dist snapshot, Nov 13)
    assets/                     <- JS/CSS/images (incl. the Zen globe texture)
    preload.html                <- pre-load countdown page (Nov 20, midnight AST)
  src-tauri/
    Cargo.toml                  <- Tauri v2, updater + notification + process plugins
    tauri.conf.json             <- app id, window, bundle targets, updater
    build.rs                    <- tauri_build::build()
    src/main.rs                 <- window, system tray, notifications, auto-updater
    capabilities/default.json   <- Tauri v2 capability grants
    icons/                      <- Zen logo: icon.png (512), 128/32, icon.ico
                                  (icon.icns is generated in CI by `tauri icon`)
  .github/workflows/build.yml   <- builds Windows + macOS installers on release
  scripts/sync-frontend.sh      <- refresh src/ from the Zen artifact's client/dist
  .gitignore
```

- **Window / tray / notifications / updater** are in `src-tauri/src/main.rs`:
  window (1280×800), system tray (Show Zen / Check for Updates / Quit
  Zen), native notifications (update available), and the auto-updater
  (checks on startup and from the tray; downloads, installs, restarts).
- **Installers:** Windows NSIS `.exe` + `.msi`, macOS `.dmg`
  (`bundle.targets` in `tauri.conf.json`). Built by GitHub Actions.
- **Frontend snapshot:** `src/` is a copy of the Zen app's `client/dist`
  (Nov 13, 2026). It is a snapshot — refresh it with
  `bash scripts/sync-frontend.sh` before every release build.
- **Icon:** converted from the Zen logo (`icon.webp`, 256×256) to PNG
  (512/256/128/32) + Windows `.ico`. macOS `.icns` is generated in CI.

## How the pre-load / Beta flow works

The app has two modes, chosen at **build time** by the `ZEN_BACKEND_URL`
env var (baked into the binary by CI):

| Build | `ZEN_BACKEND_URL` | What the app shows |
|---|---|---|
| Pre-load (Nov 14) | *unset* | Bundled `preload.html`: a countdown to the Beta (Nov 20, midnight AST). Works offline — no backend needed. |
| Beta (Nov 20) | `https://<vultr-host>` | The hosted Zen app (full chat, voice, servers). The prebuilt frontend's relative `./actions` calls resolve to the backend origin, so no frontend rebuild is needed. |

The **auto-updater is the unlock**: the Beta release is published to
GitHub Releases, the pre-load app checks on startup, downloads it,
installs it, and restarts into the Beta build (which has the backend URL).
That is how a Nov 14 download becomes the Nov 20 Beta without the user
re-downloading anything.

Why the window loads the backend URL in Beta mode instead of the bundled
`index.html`: the prebuilt Zen frontend posts to relative `./actions`.
Served from the backend origin, those calls reach the backend. Served
from the app bundle (`tauri://localhost`), they would not. Loading the
backend URL is the correct, zero-rebuild path. The bundled `src/` is
still the static source the server serves (see "Backend assessment") and
the fallback if the backend is unreachable.

## What is needed next (in order)

1. **GitHub repo (not created).** Create a repo, push this folder as the
   repo root, and confirm the Actions workflow appears. No repo exists
   yet; nothing has been pushed. (Do not push from this VM without
   Brodie's go-ahead — the repo is his.)
2. **Updater signing keypair (one-time, before the first build).**
   On a machine with the Tauri CLI:
   `tauri signer generate --write-keys` (choose a password). Then:
   - put the **public key** into `src-tauri/tauri.conf.json`
     (`plugins.updater.pubkey`, currently a placeholder) and commit it;
   - add the **private key** + password as repo secrets
     `TAURI_SIGNING_PRIVATE_KEY` and
     `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
   Without these the build fails — the updater only installs signed
   releases. The private key is never committed.
3. **Backend URL (the Vultr server).** Set the repo variable
   `ZEN_BACKEND_URL` (Settings → Secrets and variables → Actions →
   Variables) to the server's URL once it is live, e.g.
   `https://zen.example.com` (the real host/domain is TBD — do not
   hardcode a placeholder domain). Leave it **unset** for the pre-load
   build. The CI workflow reads `vars.ZEN_BACKEND_URL`.
4. **The backend host server on Vultr (the big open task).** The Zen app
   does not run on a plain Ubuntu VPS as-is — see "Backend assessment"
   below. Someone must build the host server (the adapter that serves
   the app + its API) and deploy it before the Beta build can load
   anything. **This is the gating item for Nov 20, not the installers.**
   The server must be live by Nov 20 (also marathon day).
5. **Refresh the frontend snapshot** (`bash scripts/sync-frontend.sh`)
   right before cutting a release, so the installers ship the latest UI.
   Commit the refreshed `src/`.
6. **Cut the release.** Create a GitHub release (tag `vX.Y.Z`). The
   workflow builds Windows + macOS installers and attaches them to it.
   A pre-release works the same way for a test build.
7. **Download page** on `zen.zs-gaming-hub.com` (Windows + Mac buttons,
   direct download, no store). Not built yet — the installers' download
   URLs come from the GitHub release.
8. **Signing: none.** Ship unsigned (decided Nov 13). No Apple Developer
   ($99/year) and no Windows code-signing cert (~$200–400/year).
   First-open warnings (Gatekeeper / SmartScreen) are expected and the
   app still installs.

## Backend assessment — what Zen needs to run on the Vultr server

### Correction to the plan
`desktop-app-plan.md` says "Backend: Supabase". That is **wrong**. The
Zen app's data does **not** live in Supabase. Supabase is used only for
one thing: verifying a Z Hub login (`loginWithZHub` calls Supabase Auth's
password grant and reads the caller's Z Hub profile). Everything else —
accounts, servers, channels, messages, voice, pre-loads — lives in the
app's own backend (below). The desktop plan's Supabase line should be
corrected.

### What the app actually is
Zen is a Muse "TypeScript space" (a full-stack artifact), not a plain
Node app:

| Piece | What it is | Size / count |
|---|---|---|
| Client | React 19 SPA, built by the SDK (`Bun.build`), static output `client/dist/` (6.4 MB: `index.html` + JS/CSS/images) | 6.4 MB |
| Server | TypeScript actions in `server/src/actions.ts` (12,068 lines), compiled by `bun build --target=bun` to `server/dist/actions.js` (1.4 MB). One `Actions` object, ~537 `defineAction` call sites. | 12,068 lines, ~537 actions |
| Schema | Drizzle ORM, `server/src/schema.ts` (1,201 lines), **166 tables** | 166 tables |
| Migrations | Plain SQL, `drizzle/0001–0033` (33 files), journal `drizzle/meta/_journal.json` (SQLite dialect) | 33 migrations |
| Database | **SQLite** file (`app.db`), via `drizzle-orm` + `bun:sqlite`. **Not Postgres, not Supabase.** | 1 file |
| Runtime | Bun 1.3.10 (`packageManager: bun@1.3.10`), TypeScript, `zod`, `drizzle-orm` 0.45.2 | — |

### The client ↔ server contract
The client (`client/src/api.ts`) uses `createActionClient` from
`@hatch/space-sdk/client`, which POSTs `{action, args}` to `./actions`
and returns typed JSON. The SDK's client accepts an `endpoint` override,
so a rebuilt frontend could point at a remote backend — but the shipped
bundle uses the default relative `./actions`. That is why the desktop
app loads the backend URL in Beta mode (see "How it works").

### What is portable vs. what is not
- **Portable as-is:** the client bundle (`client/dist`, static files);
  the database file (`app.db`, SQLite) and the migration SQL
  (`drizzle/0001–0033`); the action *logic* (TypeScript) and the schema.
- **NOT portable as-is — the hard part:** the server actions run on the
  proprietary `@hatch/space-sdk` runtime. The SDK is a `file:` dependency
  (`file:/opt/hatch/skills/spaces/ts-runtime/dist/space-sdk.tgz`) — it is
  **not on npm** and cannot be `npm install`ed on a Vultr VPS. The
  actions depend on runtime services the SDK provides through `Ctx`:
  - `ctx.db` — the SQLite handle (drizzle). Portable in principle
    (Bun ships `bun:sqlite`), but it is wired through the SDK's `Ctx`.
  - `ctx.blobs` — media/blob storage (`getUrl` with expiry, `delete`).
    Used for avatars, message media, and attachments. The artifact
    keeps blobs in its `blobs/` directory. A self-hosted server needs
    disk (or object) storage + signed URLs.
  - `ctx.inference` — server-side AI (`complete(...)`) for translations,
    channel/thread summaries, smart DM replies, and daily digests.
    Needs an LLM API (or those features are stubbed/disabled).
  - Viewer/auth context — the SDK supplies the signed-in viewer. On a
    self-hosted server the Z Hub login (migration 0033: `zhub_links`,
    `zhub_sessions`, 30-day sessions) is the auth path.
  - The action dispatcher itself — the `POST /actions` endpoint that
    validates args (zod), routes to the action, and returns JSON.

### The two options to run it on Vultr
- **Option A (recommended): vendor the SDK + write a thin host server.**
  Copy the SDK tarball into the deployment, write a small Bun HTTP
  server that (a) serves `client/dist` statically, (b) exposes
  `POST /actions` and dispatches to the compiled `server/dist/actions.js`
  with a re-implemented `Ctx` (SQLite via drizzle + `bun:sqlite`, viewer
  from the Z Hub session, blobs on disk, inference proxied to an LLM API
  or stubbed). This keeps all ~537 actions working. It is real
  engineering (the `Ctx` surface is the work), not a lift-and-shift.
- **Option B: refactor the action layer off the SDK.** Replace
  `defineAction`/`Ctx` with plain HTTP handlers + drizzle. Same result,
  more code churn across 12,068 lines. Only worth it if the SDK cannot
  be vendored (licensing / availability outside the Muse image is
  unconfirmed — check before choosing this path).

Either way, **do not assume the app lifts onto the VPS**. The data layer
is portable; the runtime is not. Budget real time for the host server
before Nov 20.

### Runtime dependencies (Vultr server — Ubuntu 24.04, 1 vCPU / 1 GB RAM / 25 GB SSD)
- **Bun 1.3.x** (the runtime; install from bun.sh). Node 24 (for npm/npx
  and the Tauri icon step if done on the server — normally CI-only).
- **Nginx** (reverse proxy + static files for `client/dist`), **Certbot**
  (Let's Encrypt TLS), **PM2** (or systemd) to keep the host server up,
  **UFW** (allow 22/80/443 only).
- **SQLite** — no separate server; it is a file (`app.db`) via Bun's
  built-in `bun:sqlite`. No Postgres needed.
- No GPU / extra RAM needed: inference is an API call (or stubbed), not
  local. 1 GB RAM is tight but workable for Bun + Nginx + SQLite at
  beta scale; watch memory if the host server also serves many
  concurrent voice-signalling requests (signalling is lightweight —
  WebRTC media is peer-to-peer, audio/video never goes through the
  server).

### Migration steps (once the host-server approach is chosen)
1. **Server:** spin up the Vultr instance (Toronto, Ubuntu 24.04),
   `apt update`, install Bun, Node, Nginx, Certbot, PM2, UFW; open
   22/80/443.
2. **Code:** deploy `server/` (actions + schema), `client/dist/`
   (static), `drizzle/` (migrations), and the host server. Use the
   `src/` snapshot in this repo as the static source (it is the same
   `client/dist`).
3. **Data — pick one:**
   - **(a) Copy `app.db`** (recommended for the Beta): copy the live
     artifact `app.db` to the server (back it up first). This preserves
     every account, server, and message — including the official hub
     server pre-loaded for Nov 20 (announcements / general / gaming /
     voice-lounge, locked, unlocks Nov 20 midnight AST) and the Z Hub
     account links. Copy the file; do not run SQL against it by hand.
   - **(b) Fresh migrations:** run `drizzle/0001–0033` against a new
     SQLite file, then run the app's `setupHubPreload` action once to
     create + lock the official hub server. This starts with empty data
     (no existing accounts/messages).
4. **Config (env vars on the server, never in the repo):**
   `SUPABASE_URL=https://mkqrekalzqnswgxvnfjr.supabase.co` and the
   **public anon key** (for Z Hub login verification — the same public
   key The Z Hub site ships; never the service-role key). `ZEN_BACKEND_URL`
   is the public URL of this server (the value baked into the Beta build).
5. **Nginx:** proxy `/` → static `client/dist`; proxy `/actions` → the
   host server (Bun, e.g. `127.0.0.1:3000`). Certbot for TLS.
6. **Start + smoke test:** start the host server under PM2/systemd;
   confirm `GET /` serves the app, `POST /actions` answers, a Z Hub
   login works on a phone (phone is ground truth for sign-in), a message
   sends, and a second device joins voice. Then set the repo variable
   `ZEN_BACKEND_URL` and cut the Beta release.

### External services the app touches
- **Supabase Auth + REST** (`mkqrekalzqnswgxvnfjr.supabase.co`) — Z Hub
  login only (password grant + the caller's own profile row). Public
  anon key only; the password is never stored by Zen.
- **Google public STUN** (`stun:stun.l.google.com:19302` + mirrors) —
  WebRTC NAT traversal only; no call audio/video passes through it.
- **Open-Meteo + dictionaryapi.dev** — client-side, keyless, only when
  the user runs `/weather` / `/define`. Degrade to an honest error when
  unreachable.
- **RSS feeds** — server-side fetch of feed URLs members supply.
- **Server-side inference** — AI summaries/replies/digests (see above;
  needs an LLM API or stubbing on Vultr).

## How to build

**CI (the real path):** push this folder to a GitHub repo, set the
secrets/variable above, create a release — the workflow builds Windows
(`.exe` NSIS + `.msi`) and macOS (`.dmg`) installers and attaches them.
The first CI run is the compile check for the Rust code (it was written
here without a local Rust toolchain — this VM has no `cargo`/`rustc`).

**Local (if you have Rust + the Tauri CLI):**
```bash
# Pre-load build (no backend URL -> countdown page):
cargo tauri build
# Beta build (loads the hosted Zen app):
ZEN_BACKEND_URL=https://<vultr-host> cargo tauri build
```

**Refresh the frontend snapshot before a release:**
```bash
bash scripts/sync-frontend.sh   # copies the latest client/dist into src/
```

## Known limitations / risks (read before cutting a release)
- **The Rust code was not compile-tested here** (no `cargo` on this VM).
  The API surface was verified against the Tauri v2 docs on Nov 13, 2026
  (updater flow `app.updater()?.check()` + `download_and_install`,
  `app.restart()` via ProcessExt, `MenuItem::with_id` + `Menu::with_items`,
  `TrayIconBuilder` with the `tray-icon` feature) — but the first CI run
  is still the real compile check.
- **`icon.icns` is not in the repo** (PIL cannot write `.icns`). CI
  generates it from `icon.png` via `tauri icon`. If that step fails,
  the macOS build will complain about the missing icon.
- **Updater `pubkey` is a placeholder** in `tauri.conf.json`. Replace it
  with the real public key before the first build, or the updater will
  reject the release.
- **The backend host server is the gating item** (see above). Without it
  the Beta build has nothing to load. Do not cut the Beta release until
  the server answers `POST /actions` and a phone login works.
- **The frontend snapshot goes stale.** `src/` is a Nov 13 copy. Always
  run `scripts/sync-frontend.sh` before a release.
- **Pre-load builds have no backend.** If someone opens the pre-load app
  after Nov 20 without updating, it still shows the countdown. The
  auto-updater is the path to the Beta — it only works once the repo,
  signing key, and Beta release exist.
- **Unsigned.** First-open Gatekeeper (Mac) / SmartScreen (Windows)
  warnings are expected (decided Nov 13 — no paid certs).

## Costs (recap)
- Hosting (Vultr, Toronto): $5 USD/month (~$7.13 CAD/month, tax may be
  added). Card on file = parents thing. No account created yet.
- GitHub Actions builds: free at this scale.
- Signing: $0 (unsigned, decided).
- Everything else: free.

## Source map (where things came from)
- Plan: `~/workspace/project-zen/desktop-app-plan.md`
- Zen app source (read-only assessment): `~/workspace/ts-spaces/real-platform/`
  (`client/`, `server/src/actions.ts`, `server/src/schema.ts`,
  `drizzle/0001–0033`, `DATA-PLAN.md`). The artifact itself is only ever
  changed through the artifact builder — nothing in this scaffold edits it.
- Hosting pick: `~/workspace/project-zen/hosting-provider-pick.md`
  (Vultr, Toronto, $5 USD/month).
