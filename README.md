# Oshirase

A utility for keeping up with the latest anime and manga. Inspired by [Taiga](https://github.com/erengy/taiga) and [Tachiyomi](https://github.com/tachiyomiorg/tachiyomi).

This is primarily a project to learn new technologies.

## Roadmap

### In Progress

* `api` - A REST API written in Go that aggregates data from anime/manga lists and sources with recent episode/chapter release and schedule information. Data is mainly sourced from the [AniList API](https://anilist.gitbook.io/anilist-apiv2-docs/), with other sources like the [MangaDex API](https://api.mangadex.org/docs/) being used to supplement this data. Will be the backend for the future `ui`. Opting to regenerate data per request instead of saving to database because a notification system really should be up to date. Could use caching where applicable.

### Planned

* `worker` - A background worker system to handle miscellaneous jobs that shouldn't run in the `api` or `ui`, i.e. generating alt_titles in the background to speed up aggregation in the `api`.
* `ui` - A frontend that will display the data aggregated by the data pipeline. Currently considering Next.js.
* `analytics` - A project to learn machine learning. Currently considering Python. Goal is to predict the score for a new anime, given a user's previous anime list scores.
