#!/bin/bash

# Script to update SQLx offline query metadata
# This script ensures the database is running and updates the .sqlx directory

set -e  # Exit on any error

echo "🔄 Updating SQLx offline query metadata..."

# Database configuration
DATABASE_URL="${DATABASE_URL:-postgresql://postgres:password@localhost:5432/albatross_dev}"

# Function to check if database is running
check_database() {
    echo "🔍 Checking if database is running..."
    if ! docker compose -f docker-compose.infra.yml ps | grep -q "albatross_postgres.*running"; then
        echo "📦 Starting database infrastructure..."
        docker compose -f docker-compose.infra.yml up -d

        # Wait for database to be ready
        echo "⏳ Waiting for database to be ready..."
        for i in {1..30}; do
            if docker compose -f docker-compose.infra.yml exec -T postgres pg_isready -U postgres -d albatross_dev >/dev/null 2>&1; then
                echo "✅ Database is ready!"
                break
            fi
            echo "Waiting... ($i/30)"
            sleep 2
        done

        if [ $i -eq 30 ]; then
            echo "❌ Database failed to start within timeout"
            exit 1
        fi
    else
        echo "✅ Database is already running"
    fi
}

# Function to run migrations (in case schema has changed)
run_migrations() {
    echo "🔧 Running database migrations..."
    export DATABASE_URL="$DATABASE_URL"
    cd apps/api-gateway
    cargo run --bin api-gateway -- --migrate-only 2>/dev/null || true
    cd ../..
}

# Function to prepare SQLx metadata
prepare_sqlx() {
    echo "📝 Preparing SQLx query metadata..."
    export DATABASE_URL="$DATABASE_URL"
    cargo sqlx prepare --workspace

    echo "✅ SQLx metadata updated successfully!"
    echo "📁 Check the .sqlx directory for updated query files"
}

# Main execution
main() {
    check_database
    run_migrations
    prepare_sqlx

    echo ""
    echo "🎉 SQLx metadata update complete!"
    echo "💡 Remember to commit the .sqlx directory to version control:"
    echo "   git add .sqlx/"
    echo "   git commit -m 'Update SQLx query metadata'"
}

# Run main function
main "$@"
