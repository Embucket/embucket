# embucket-seed

Implements seeding data for embucket database. Supports 3 levels of seeding: Minimal, Typical, Extreme.

## Usage

```bash
embucket-seed --server-address '127.0.0.1:3000' --auth-user embucket --auth-password embucket --seed-variant typical

# or using cargo run
cargo run -p embucket-seed -- --server-address '127.0.0.1:3000' --auth-user embucket --auth-password embucket --seed-variant typical
```

## Updating seed templates

When updating seed templates in yaml files, embucket-seed need to be rebuilt to apply changes.