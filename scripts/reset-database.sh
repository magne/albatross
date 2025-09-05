#!/bin/bash

# Script to reset the database using SQLx
# This script safely resets the database by dropping all data and recreating the schema

set -e  # Exit on any error

# Configuration
DATABASE_URL="${DATABASE_URL:-postgresql://postgres:password@localhost:5432/albatross_dev}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Function to check if database is running
check_database() {
    log_info "Checking database connectivity..."
    if ! docker compose -f "$PROJECT_ROOT/docker-compose.infra.yml" ps | grep -q "albatross_postgres.*running"; then
        log_info "Starting database infrastructure..."
        docker compose -f "$PROJECT_ROOT/docker-compose.infra.yml" up -d

        log_info "Waiting for database to be ready..."
        for i in {1..30}; do
            if docker compose -f "$PROJECT_ROOT/docker-compose.infra.yml" exec -T postgres pg_isready -U postgres -d albatross_dev >/dev/null 2>&1; then
                log_success "Database is ready!"
                return 0
            fi
            echo "Waiting... ($i/30)"
            sleep 2
        done

        log_error "Database failed to start within timeout"
        return 1
    else
        log_success "Database is already running"
    fi
}

# Function to confirm destructive operation
confirm_reset() {
    if [[ "$1" == "--force" ]]; then
        log_warning "Skipping confirmation due to --force flag"
        return 0
    fi

    log_warning "This will DELETE ALL DATA in the database!"
    echo "Database: $DATABASE_URL"
    echo ""
    read -p "Are you sure you want to continue? (type 'yes' to confirm): " -r
    echo ""

    if [[ ! $REPLY =~ ^yes$ ]]; then
        log_info "Operation cancelled by user"
        exit 0
    fi
}

# Function to reset database
reset_database() {
    log_info "Resetting database..."

    # Connect to database and drop all tables
    log_info "Dropping all tables..."
    docker compose -f "$PROJECT_ROOT/docker-compose.infra.yml" exec -T postgres psql -U postgres -d albatross_dev -c "
        DO \$\$ DECLARE
            r RECORD;
        BEGIN
            FOR r IN (SELECT tablename FROM pg_tables WHERE schemaname = 'public') LOOP
                EXECUTE 'DROP TABLE IF EXISTS ' || quote_ident(r.tablename) || ' CASCADE';
            END LOOP;
        END \$\$;
    " >/dev/null 2>&1

    log_success "All tables dropped"
}

# Function to run migrations
run_migrations() {
    log_info "Running database migrations..."

    cd "$PROJECT_ROOT/apps/api-gateway"
    export DATABASE_URL="$DATABASE_URL"

    if cargo run --bin api-gateway -- --migrate-only >/dev/null 2>&1; then
        log_success "Migrations completed successfully"
    else
        log_error "Migration failed"
        return 1
    fi

    cd "$PROJECT_ROOT"
}

# Function to update SQLx metadata
update_sqlx_metadata() {
    log_info "Updating SQLx query metadata..."

    export DATABASE_URL="$DATABASE_URL"

    if cargo sqlx prepare --workspace >/dev/null 2>&1; then
        log_success "SQLx metadata updated"
    else
        log_error "SQLx metadata update failed"
        return 1
    fi
}

# Function to verify database schema
verify_schema() {
    log_info "Verifying database schema..."

    local table_count
    table_count=$(docker compose -f "$PROJECT_ROOT/docker-compose.infra.yml" exec -T postgres psql -U postgres -d albatross_dev -t -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public';" 2>/dev/null | tr -d ' ')

    if [[ "$table_count" -gt 0 ]]; then
        log_success "Database schema verified ($table_count tables created)"
    else
        log_error "No tables found in database"
        return 1
    fi
}

# Function to show usage
usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Reset the database using SQLx"
    echo ""
    echo "Options:"
    echo "  --force      Skip confirmation prompts"
    echo "  --help       Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  DATABASE_URL    Database connection URL (default: postgresql://postgres:password@localhost:5432/albatross_dev)"
    echo ""
    echo "Examples:"
    echo "  $0                    # Interactive mode with confirmation"
    echo "  $0 --force           # Skip confirmation prompts"
}

# Main execution
main() {
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --force)
                FORCE_RESET=true
                shift
                ;;
            --help)
                usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done

    echo "🔄 Database Reset Script"
    echo "========================"
    echo ""

    # Check database connectivity
    if ! check_database; then
        exit 1
    fi

    # Confirm destructive operation
    confirm_reset "$FORCE_RESET"

    # Reset database
    if ! reset_database; then
        exit 1
    fi

    # Run migrations
    if ! run_migrations; then
        exit 1
    fi

    # Update SQLx metadata
    if ! update_sqlx_metadata; then
        exit 1
    fi

    # Verify schema
    if ! verify_schema; then
        exit 1
    fi

    echo ""
    log_success "Database reset completed successfully!"
    echo ""
    log_info "Next steps:"
    echo "  1. Start your application services"
    echo "  2. Create initial admin user if needed"
    echo "  3. Test your application functionality"
    echo ""
    log_info "Remember to commit the updated .sqlx directory:"
    echo "  git add .sqlx/"
    echo "  git commit -m 'Update SQLx query metadata'"
}

# Run main function
main "$@"
