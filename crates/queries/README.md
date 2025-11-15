# queries

Package is responsible for the abstraction and interaction with the underlying queries storage system. Defines data models and traits for queries storage operations.

### Using Postgres based Queries Storage with Diesel ORM

Refer here to install diesel cli:
https://diesel.rs/guides/getting-started#installing-diesel-cli

Find Diesel config in `diesel.toml` file. 

```bash
echo DATABASE_URL=postgresql://dev@localhost:5432/queries >> .env
```

To run migrations as a 'dev' user on developement server:

```bash
# run migrations (for first time it creates database tables)
diesel migration run --config-file config/diesel.toml

# get diesel schema (for development)
diesel print-schema --config-file config/diesel.toml
```

### Optional development setup
``` bash
docker run -d \
    --name postgres-container \
    -e POSTGRES_USER=postgres \
    -e POSTGRES_PASSWORD=embucket \
    -e POSTGRES_DB=postgres \
    -p 5432:5432 \
    postgres
```

``` bash
echo "CREATE USER dev WITH PASSWORD 'dev';
ALTER USER dev CREATEDB;
CREATE DATABASE queries;
CREATE DATABASE dev; -- fallback database
GRANT ALL PRIVILEGES ON DATABASE dev TO dev;" > config/postgres-dev.sql
```

``` bash
# connect as admin
export PGPASSWORD=embucket
psql -h localhost -U postgres -f config/postgres-dev.sql
```