## Running the app

Graphql backend for sstackrr game. 

DATABASE_URL env var should be set: `DATABASE_URL=postgres://sstackrr:sstackrr@localhost/sstackrr`

Before use, run migrations: `diesel migration run`

To run, do `cargo run`

GraphQL playground, at the moment of writing this: http://sstackrr-backend.apps.loskutoff.com