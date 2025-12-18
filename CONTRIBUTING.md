# Contributing to SQLx-Data

Thank you for your interest in contributing to SQLx-Data! This guide will help you set up the development environment and understand our workflow.

## Development Setup

### Prerequisites

- Rust toolchain (latest stable)
- Docker and Docker Compose
- Git

### Database Setup

SQLx-Data supports multiple databases. Use Docker Compose to run the databases locally:

```bash
# Start all databases
docker-compose up -d

# Or start specific databases
docker-compose up -d sqlite-db
docker-compose up -d mysql-db
docker-compose up -d postgres-db
```

### Environment Configuration

1. **Create `.env` file** in the project root:

```bash
cp .env.example .env
```

2. **Configure database URLs** in `.env`. Uncomment the database you want to test:

```bash
# SQLite (always available)
DATABASE_URL="sqlite:test_data/test.db"

# MySQL (uncomment when testing MySQL features)
# DATABASE_URL="mysql://user:password@localhost:3306/test_db"

# PostgreSQL (uncomment when testing PostgreSQL features)
# DATABASE_URL="postgresql://user:password@localhost:5432/test_db"
```

### Feature-Based Development

**Important**: SQLx-Data uses **feature flags for database support**. You must test **one database at a time** to avoid feature conflicts.

#### Testing SQLite

1. **Update `Cargo.toml`** - ensure SQLite integration is uncommented:

```toml
[workspace]
members = [
    "sqlx-data",
    "sqlx-data-macros",
    "sqlx-data-params",
    "sqlx-data-parser",
    "sqlx-data-integration",
    "test-integration-sqlite",      # <- Keep this uncommented
    # "test-integration-mysql",
    # "test-integration-postgresql",
]
```

2. **Update `.env`** - use SQLite DATABASE_URL:

```bash
DATABASE_URL="sqlite:test_data/test.db"
```

3. **Run commands**:

```bash
# Core library with features
cargo check --features "sqlite,json"
cargo test --features "sqlite,json"
cargo clippy --features "sqlite,json"

# Integration tests (no features needed)
cargo test --package test-integration-sqlite
```

#### Testing MySQL

1. **Update `Cargo.toml`** - uncomment MySQL integration:

```toml
[workspace]
members = [
    "sqlx-data",
    "sqlx-data-macros",
    "sqlx-data-params",
    "sqlx-data-parser",
    "sqlx-data-integration",
    # "test-integration-sqlite",
    "test-integration-mysql",       # <- Uncomment this line
    # "test-integration-postgresql",
]
```

2. **Update `.env`** - use MySQL DATABASE_URL:

```bash
DATABASE_URL="mysql://user:password@localhost:3306/test_db"
```

3. **Run commands**:

```bash
# Core library with features
cargo check --features "mysql,json"
cargo test --features "mysql,json"
cargo clippy --features "mysql,json"

# Integration tests (no features needed)
cargo test --package test-integration-mysql
```

#### Testing PostgreSQL

1. **Update `Cargo.toml`** - uncomment PostgreSQL integration:

```toml
[workspace]
members = [
    "sqlx-data",
    "sqlx-data-macros",
    "sqlx-data-params",
    "sqlx-data-parser",
    "sqlx-data-integration",
    # "test-integration-sqlite",
    # "test-integration-mysql",
    "test-integration-postgresql",  # <- Uncomment this line
]
```

2. **Update `.env`** - use PostgreSQL DATABASE_URL:

```bash
DATABASE_URL="postgresql://user:password@localhost:5432/test_db"
```

3. **Run commands**:

```bash
# Core library with features
cargo check --features "postgres,json"
cargo test --features "postgres,json"
cargo clippy --features "postgres,json"

# Integration tests (no features needed)
cargo test --package test-integration-postgresql
```

## Development Workflow

### Core Library Commands

For the core library, **always use features**:

```bash
# For SQLite development
cargo check --features "sqlite,json"
cargo test --features "sqlite,json"
cargo clippy --features "sqlite,json"

# For MySQL development
cargo check --features "mysql,json"
cargo test --features "mysql,json"
cargo clippy --features "mysql,json"

# For PostgreSQL development
cargo check --features "postgres,json"
cargo test --features "postgres,json"
cargo clippy --features "postgres,json"
```

### Integration Tests

Integration tests **do not use features**:

```bash
# SQLite integration tests
cargo test --package test-integration-sqlite

# MySQL integration tests
cargo test --package test-integration-mysql

# PostgreSQL integration tests
cargo test --package test-integration-postgresql
```

### Individual Package Tests

```bash
# Core packages with features
cargo test --package sqlx-data --features "sqlite,json"
cargo test --package sqlx-data-macros --features "sqlite,json"
cargo test --package sqlx-data-params --features "sqlite,json"
cargo test --package sqlx-data-parser --features "sqlite,json"

# Integration packages without features
cargo test --package test-integration-sqlite
cargo test --package test-integration-mysql
cargo test --package test-integration-postgresql
```

## Project Structure

```
sqlx-data/
├── sqlx-data/                 # Core library (needs features)
├── sqlx-data-macros/          # Procedural macros (needs features)
├── sqlx-data-params/          # Parameter builders (needs features)
├── sqlx-data-parser/          # SQL parsing utilities (needs features)
├── sqlx-data-integration/     # Integration helpers (needs features)
├── test-integration-sqlite/   # SQLite integration tests (no features)
├── test-integration-mysql/    # MySQL integration tests (no features)
├── test-integration-postgresql/ # PostgreSQL integration tests (no features)
├── book/                      # Documentation (mdbook)
├── docker-compose.yml         # Database containers
└── .env                       # Database configuration
```

## Feature Development

### Adding New Features

1. **Start with core implementation** in `sqlx-data/` (with features)
2. **Add macro support** in `sqlx-data-macros/` (with features)
3. **Write integration tests** in appropriate `test-integration-*/` package (no features)
4. **Update documentation** in `book/src/`

### Database-Specific Features

When adding database-specific functionality:

1. **Implement in core** with feature flags
2. **Add integration tests** in the specific test package
3. **Document differences** in the book

### Testing Strategy

- **Unit tests**: Test individual functions with features
- **Integration tests**: Test complete workflows with real databases (no features)
- **Doc tests**: Ensure documentation examples work with features

## Common Issues

### "Cannot find database" errors

Make sure:
1. Docker containers are running
2. Correct DATABASE_URL in `.env`
3. Only one integration test package uncommented in `Cargo.toml`

### Feature conflicts

SQLx features conflict when multiple databases are enabled. Always:
1. Comment out unused integration packages in `Cargo.toml`
2. Use only one DATABASE_URL at a time
3. Use matching features for core library commands

### Compilation errors

Remember:
- **Core packages**: Use `--features "database,json"`
- **Integration packages**: Use no features

## Pull Request Guidelines

1. **One feature per PR**
2. **Include tests** for new functionality
3. **Update documentation** if needed
4. **Follow existing code style**
5. **Ensure all tests pass** with proper setup

### Before Submitting

```bash
# For SQLite development
cargo check --features "sqlite,json"
cargo test --features "sqlite,json"
cargo clippy --features "sqlite,json"
cargo test --package test-integration-sqlite

# Format code
cargo fmt

# Verify documentation builds
cd book && mdbook build
```

### Example: Complete SQLite Development Cycle

```bash
# 1. Setup
docker-compose up -d sqlite-db
# Uncomment test-integration-sqlite in Cargo.toml
# Set DATABASE_URL="sqlite:test_data/test.db" in .env

# 2. Core library development (with features)
cargo check --features "sqlite,json"
cargo test --features "sqlite,json"
cargo clippy --features "sqlite,json"

# 3. Integration tests (no features)
cargo test --package test-integration-sqlite

# 4. Format and final check
cargo fmt
cargo check --features "sqlite,json"
```

### Example: Complete MySQL Development Cycle

```bash
# 1. Setup
docker-compose up -d mysql-db
# Comment out other integration tests, uncomment test-integration-mysql in Cargo.toml
# Set DATABASE_URL="mysql://user:password@localhost:3306/test_db" in .env

# 2. Core library development (with features)
cargo check --features "mysql,json"
cargo test --features "mysql,json"
cargo clippy --features "mysql,json"

# 3. Integration tests (no features)
cargo test --package test-integration-mysql

# 4. Format and final check
cargo fmt
cargo check --features "mysql,json"
```

## Getting Help

- **Issues**: Create a GitHub issue for bugs or feature requests
- **Discussions**: Use GitHub Discussions for questions
- **Documentation**: Check the book at `book/src/`

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT).

Thank you for contributing to SQLx-Data! 🦀✨