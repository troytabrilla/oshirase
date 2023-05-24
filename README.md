# Oshirase

A utility for keeping up with the latest anime and manga. Inspired by [Taiga](https://github.com/erengy/taiga) and [Tachiyomi](https://github.com/tachiyomiorg/tachiyomi).

This is primarily a project to learn new technologies.

## Roadmap

### In Progress

* `aggregator` - A data pipeline written in Rust. Aggregates data from anime/manga lists and sources with recent episode/chapter release information. Data is mainly sourced from the [AniList API](https://anilist.gitbook.io/anilist-apiv2-docs/), with other sources like the [MangaDex API](https://api.mangadex.org/docs/) being used to supplement this data.
* `api` - A REST API written in Go that returns the same data as above. Basically `aggregator`, but without persistence or workers; all calculations are done on the server. For comparing/contrasting with the job-based aggregator approach.

### Planned

* `ui` - A frontend that will display the data aggregated by the data pipeline. Currently considering Next.js.
* `analytics` - A project to learn machine learning. Currently considering Python. Goal is to predict the score for a new anime, given a user's previous anime list scores.
