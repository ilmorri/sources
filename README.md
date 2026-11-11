# ilmorri Sources

Personal fork of [Aidoku-Community/sources](https://github.com/Aidoku-Community/sources), trimmed to **English + Japanese**: every `en.*` and `ja.*` source, plus the `multi.*` sources with their language lists cut to en/ja. Source IDs are unchanged, so library entries and reading progress carry over.

Add to Aidoku (Settings → Source Lists): `https://ilmorri.github.io/sources/index.min.json`

Then remove the community list, so its updates don't replace your fixes.

## Fix a broken source

```sh
cd sources/en.<name>
# edit src/lib.rs, or templates/<template>/src/ if the bug is in the shared theme
cargo test                       # live-site tests (#[aidoku_test]); needs aidoku-test-runner
aidoku package && aidoku verify package.aix
aidoku serve package.aix         # optional: test on the phone before pushing
aidoku logcat                    # live println!/errors from the phone while you use the source
```

Then **bump `info.version` in `res/source.json`**. The app only updates when the number goes up, so always make it higher than upstream's. Commit and push to `main`. CI builds every source, publishes the list to GitHub Pages, and the phone offers the update.

If you change a template, bump the version of **every source that uses it** (`grep -l 'templates/<name>"' sources/*/Cargo.toml`).

CI reliability: if one source fails to build, it is left out of the list, everything else still deploys, and the run is marked failed with the broken source listed.

### Phone can't reach `aidoku serve` (WSL)

WSL2's NAT hides the server from the LAN. Pick one:
- `C:\Users\<you>\.wslconfig` → `[wsl2]` / `networkingMode=mirrored`, then `wsl --shutdown`
- Admin PowerShell: `netsh interface portproxy add v4tov4 listenport=8080 listenaddress=0.0.0.0 connectport=8080 connectaddress=<output of: wsl hostname -I>` plus `New-NetFirewallRule -DisplayName aidoku -Direction Inbound -LocalPort 8080 -Protocol TCP -Action Allow`
- Or skip serving: `cp package.aix /mnt/c/Users/<you>/Desktop/` and open it on the phone via Files/AirDrop

## Sync with upstream

```sh
git fetch upstream && git merge upstream/main
```

Expected conflicts:
- upstream changed a source you deleted (non en/ja) → `git rm -r <path>`
- upstream changed a `multi.*` `source.json` → keep upstream's new `version` + 1 and your trimmed `languages`
- a source you fixed → merge by hand, or take upstream's version if they already fixed it

## What usually breaks

Most sources are thin configs over a shared template. Site redesigns break the template's CSS selectors, so check these first. A source overrides one in its `params()`, e.g. `search_manga_selector` in `sources/en.toonily/src/lib.rs`.

| Template (sources) | Selectors that break most | Where |
|---|---|---|
| mangathemesia (9) | `chapter_list_selector`, `series_cover_selector`, `series_title_selector` | `templates/mangathemesia/src/lib.rs` |
| madara (6) | `search_manga_selector`, `chapter_selector` (`li.wp-manga-chapter`), `details_*`, `chapter_protector_selector` (encrypted pages) | `templates/madara/src/lib.rs` |
| liliana (5), mangabox (4), iken (4) | list / chapter / page selectors | `templates/<name>/src/imp.rs` |

Other common failures:
- **Domain change**: update `BASE_URL` in `src/lib.rs` and `info.url` in `res/source.json`
- **Cloudflare added**: requests return 403/503, and logcat shows HTML instead of data. Open the source's site from the app once so Aidoku's WebView can solve the challenge.
- **API sources** (mangadex, mangaplus, webtoon, …): JSON field renames in `models.rs`

## License

Apache-2.0 or MIT, same as upstream.
