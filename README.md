# Oshirase

A utility for keeping up with the latest anime and manga.

This is primarily a project to learn new technologies.

## Roadmap

### In Progress

* aggregator - A date pipeline written in Rust (and eventually Go, for comparison). Aggregates data from anime/manga lists and sources with recent episode/chapter release information. Data is mainly sourced from the [AniList API](https://anilist.gitbook.io/anilist-apiv2-docs/), with other sources like the [MangaDex API](https://api.mangadex.org/docs/) being used to supplement this data.

### Planned

* ui - A frontend that will display the data aggregated by the data pipeline. Currently considering Tauri or Next.js.
* analytics - A project to learn machine learning. Currently considering Julia or Python. Goal is to predict the score for a new anime, given a user's previous anime list scores.
