# queries

Package is responsible for the abstraction and interaction with the underlying queries storage system. Defines data models and traits for queries storage operations.

## Development setup
``` bash
docker run -d \
    --name postgres-container \
    -e POSTGRES_USER=postgres \
    -e POSTGRES_PASSWORD=embucket \
    -e POSTGRES_DB=postgres \
    -p 5432:5432 \
    postgres
```

### Create dev user
``` bash
# connect as admin
export PGPASSWORD=embucket
echo "CREATE USER dev WITH PASSWORD 'dev'; ALTER USER dev CREATEDB;"  | psql -h localhost -U postgres
```

### Create database as dev user

``` bash
echo "CREATE DATABASE dev;" | psql -h localhost -U dev -d postgres -W
```

## Build prerequisites and Diesel setup

### Build prerequisites

Yep, it has external dependency on libpq,  which is a postgres client library.
```bash
apt install -y libpq-dev
```

### Generate Diesel schema using Diesel migrations on dev database

Refer here how to install diesel cli:
https://diesel.rs/guides/getting-started#installing-diesel-cli

Diesel config is in repo root in `config/diesel.toml` file. 

Before running diesel cli set DATABASE_URL env var or create .env file:
```bash
echo DATABASE_URL=postgresql://dev@localhost:5432/dev >> .env
```

Run migrations to re-generate diesel schema:

```bash
# run migrations (for first time it creates database tables)
diesel migration run --config-file config/diesel.toml

# get diesel schema (for development)
diesel print-schema --config-file config/diesel.toml
```

### Development tricks
Attempted migration will not re-generate initial diesel schema, if table already exists.
Drop tables to trigger initial setup:
```
drop table public.__diesel_schema_migrations;
drop table public.queries;
```