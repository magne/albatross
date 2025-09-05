# Development Scripts

This directory contains utility scripts for development workflow automation.

## update-sqlx-metadata.sh

Updates SQLx offline query metadata for compile-time SQL verification.

### Purpose
- Ensures database infrastructure is running
- Runs database migrations to keep schema up-to-date
- Generates/updates `.sqlx` query metadata files
- Enables compilation without requiring database connection

### Usage

```bash
# Run the script (it will start database if needed)
./scripts/update-sqlx-metadata.sh

# Or with custom DATABASE_URL
DATABASE_URL="postgresql://user:pass@localhost:5432/db" ./scripts/update-sqlx-metadata.sh
```

### What it does

1. **Database Check**: Verifies PostgreSQL container is running
2. **Auto-start**: Starts database infrastructure if not running
3. **Health Check**: Waits for database to be ready
4. **Migrations**: Runs database migrations (if api-gateway supports --migrate-only)
5. **SQLx Prepare**: Generates query metadata with `cargo sqlx prepare --workspace`
6. **Instructions**: Reminds you to commit `.sqlx` directory

### Requirements

- Docker and Docker Compose
- `docker-compose.infra.yml` in project root
- DATABASE_URL environment variable (defaults to development settings)

### After Running

Remember to commit the updated metadata:

```bash
git add .sqlx/
git commit -m "Update SQLx query metadata"
```

### Troubleshooting

- **Permission denied**: Run `chmod +x scripts/update-sqlx-metadata.sh`
- **Database connection failed**: Check Docker containers with `docker compose -f docker-compose.infra.yml ps`
- **No queries found**: Ensure you're running from project root and DATABASE_URL is set
