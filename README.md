# Albatross

Albatross is a web-based, multi-tenant Virtual Airline (VA) Management Platform built with modern technologies including Rust/Axum backend, Event Sourcing/CQRS architecture, React frontend, and real-time WebSocket updates.

## Features

- **Multi-tenant Virtual Airline Management**: Secure isolation between different VA instances
- **User Management**: Registration, authentication, and role-based access control (PlatformAdmin, TenantAdmin, Pilot)
- **Fleet & Route Management**: Aircraft assignments and route planning
- **Real-time Updates**: WebSocket-powered live UI updates
- **API Key Management**: Secure API key generation and revocation
- **Event Sourcing**: Complete audit trail of all system changes
- **Modern UI**: React + Tailwind CSS responsive interface

## Architecture

- **Backend**: Rust/Axum with Event Sourcing and CQRS patterns
- **Frontend**: React + TypeScript + Tailwind CSS
- **Database**: PostgreSQL for event store and read models
- **Message Queue**: RabbitMQ for event distribution
- **Cache/PubSub**: Redis for caching and real-time notifications
- **Deployment**: Docker Compose for local development, Kubernetes for production

## Prerequisites

- **Rust**: Latest stable version (1.70+)
- **Node.js**: 18+ with pnpm
- **Docker & Docker Compose**: For running infrastructure
- **PostgreSQL**: 13+ (via Docker)
- **RabbitMQ**: 3.9+ (via Docker)
- **Redis**: 6+ (via Docker)

## Quick Start

### 1. Clone and Setup

```bash
git clone https://github.com/magne/albatross.git
cd albatross
```

### 2. Start Infrastructure

```bash
# Start PostgreSQL, RabbitMQ, and Redis
docker-compose -f docker-compose.infra.yml up -d
```

### 3. Backend Setup

```bash
# Install Rust dependencies and run migrations
cd apps/api-gateway
cargo build
# Run database migrations (adjust DATABASE_URL in .env)
# The projection worker will handle migrations automatically

# Start the API Gateway
cargo run
```

In another terminal:

```bash
# Start the Projection Worker
cd apps/projection-worker
cargo run
```

### 4. Frontend Setup

```bash
cd apps/web-ui

# Install dependencies
pnpm install

# Start development server
pnpm dev
```

### 5. Access the Application

- **Frontend**: <http://localhost:5173> (Vite dev server)
- **API**: <http://localhost:3000>
- **WebSocket**: ws://localhost:3000/api/ws

### 6. Bootstrap First Admin

1. Open <http://localhost:5173>
2. You'll see the bootstrap form for creating the first PlatformAdmin user
3. Fill in username, email, password
4. Submit to create the admin account
5. The API key will be set automatically for authentication

## Development Setup

### Environment Variables

**Important**: You must create `.env` files in each service directory. The application uses `dotenv` to load these files, and each service requires its own `.env` file in its directory.

Create the following `.env` files:

**`apps/api-gateway/.env`:**

```bash
DATABASE_URL=postgresql://postgres:password@localhost:5432/albatross_dev
RABBITMQ_URL=amqp://guest:guest@localhost:5672
REDIS_URL=redis://localhost:6379
JWT_SECRET=your-secret-key-here
```

**`apps/projection-worker/.env`:**

```bash
DATABASE_URL=postgresql://postgres:password@localhost:5432/albatross_dev
RABBITMQ_URL=amqp://guest:guest@localhost:5672
REDIS_URL=redis://localhost:6379
```

**Note**: If you get errors like "RABBITMQ_URL must be set: NotPresent", it means the `.env` file is missing from that service's directory. Make sure to create the `.env` file in the correct location (`apps/projection-worker/.env` for the projection worker, `apps/api-gateway/.env` for the API gateway).

### Running Individual Services

```bash
# API Gateway
cd apps/api-gateway && cargo run

# Projection Worker
cd apps/projection-worker && cargo run

# Frontend (in another terminal)
cd apps/web-ui && pnpm dev
```

### Database Setup

The projection worker automatically runs migrations on startup. If you encounter issues with migrations, you can manually create the required tables:

```bash
# Check if tables exist
docker exec albatross_postgres psql -U postgres -d albatross_dev -c "\dt"

# If tables are missing, you can create them manually using the SQL in:
# apps/projection-worker/migrations/01__initial_read_models.sql
# apps/projection-worker/migrations/02__add_user_api_keys.sql
```

**Note**: The application expects these tables to exist:
- `tenants` - Stores tenant information
- `users` - Stores user accounts and profiles
- `user_api_keys` - Stores API key information

## Testing

### Backend Tests

```bash
# Run all tests
cargo test --all

# Run specific service tests
cd apps/api-gateway && cargo test
cd apps/projection-worker && cargo test

# Run with coverage (requires cargo-tarpaulin)
cargo tarpaulin --all
```

### Frontend Tests

```bash
cd apps/web-ui

# Run unit tests
pnpm test

# Run E2E tests (requires Playwright setup)
pnpm test:e2e

# Run linting
pnpm lint
```

### Integration Tests

```bash
# Start infrastructure
docker-compose -f docker-compose.infra.yml up -d

# Run integration tests
cargo test --test integration
```

### Manual Testing

1. **Bootstrap Flow**:
   - Start the app without any users
   - Verify the bootstrap form appears
   - Create first admin user
   - Verify API key is set and UI updates

2. **Real-time Updates**:
   - Open multiple browser tabs
   - Create a tenant in one tab
   - Verify the tenant list updates in real-time in other tabs

3. **API Key Management**:
   - Generate a new API key
   - Use it to authenticate API requests
   - Revoke the key and verify access is denied

4. **RBAC Testing**:
   - Create users with different roles
   - Verify role-based access to features
   - Test tenant isolation

## API Documentation

### Authentication

All API requests require an API key in the Authorization header:

```text
Authorization: Bearer your-api-key-here
```

### Key Endpoints

- `POST /api/users/register` - Bootstrap first admin
- `GET /api/tenants/list` - List tenants (RBAC filtered)
- `GET /api/users/list` - List users (RBAC filtered)
- `POST /api/users/{userId}/apikeys` - Generate API key
- `DELETE /api/users/{userId}/apikeys/{keyId}` - Revoke API key
- `GET /api/ws` - WebSocket endpoint for real-time updates

## Deployment

### Docker Compose (Local)

```bash
# Build and run all services
docker-compose up --build
```

### Kubernetes (Production)

```bash
# Apply Helm charts
cd infra/helm
helm install albatross .
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass
6. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.
