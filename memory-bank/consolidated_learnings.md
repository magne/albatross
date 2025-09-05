# Consolidated Learnings

This file contains distilled, actionable insights and patterns extracted from `raw_reflection_log.md`. These represent fundamental learnings that should guide future development.

## Frontend State Management Patterns

### User Context Stability
**Pattern:** Always verify hook implementations call correct endpoints for user-specific data
**Context:** `useUserSelf` hook was calling `listUsers` instead of `/api/users/self`
**Impact:** User context switching after creating new users
**Prevention:** Test user context stability across CRUD operations

### API Integration Best Practices
**Pattern:** Include proper Authorization headers for authenticated endpoints
**Context:** Missing Bearer token in user-specific API calls
**Impact:** Authentication failures and incorrect data retrieval
**Prevention:** Use centralized API client with automatic auth header injection

## Enum Value Synchronization

### Frontend-Backend Enum Mapping
**Pattern:** Verify enum values match between frontend constants and backend protobuf definitions
**Context:** Frontend used 0,1,2 while protobuf expected 1,2,3 for roles
**Impact:** 400 Bad Request errors on user creation
**Prevention:** Create shared constants or automated validation for enum mappings

### Role-Based UI Filtering
**Pattern:** Update all role comparison logic when changing enum values
**Context:** Role filtering in UserCreateForm used old values
**Impact:** Incorrect role availability in UI
**Prevention:** Use centralized role constants and automated tests

## Database Operations

### SQLx Migration Management
**Pattern:** Use `sqlx migrate run --source <path>` for explicit migration source control
**Context:** Database reset script needed proper SQLx integration
**Impact:** Migration failures without explicit source paths
**Prevention:** Document migration directory structure and SQLx command patterns

### Safety-First Database Scripts
**Pattern:** Include confirmation prompts and comprehensive error handling in destructive operations
**Context:** Database reset script for development workflows
**Impact:** Potential data loss without safety measures
**Prevention:** Standardize safety patterns for all database maintenance scripts

## Testing and Validation

### Multi-Path Testing
**Pattern:** Test both bootstrap and authenticated code paths
**Context:** User creation worked for bootstrap but failed for authenticated requests
**Impact:** Incomplete test coverage missed authentication issues
**Prevention:** Create test matrices covering all authentication states

### Integration Test Importance
**Pattern:** Add integration tests for complete user workflows
**Context:** User creation flow had multiple integration points
**Impact:** Issues only discovered during end-to-end testing
**Prevention:** Test complete workflows, not just individual components

## Debugging Methodology

### Systematic Investigation
**Pattern:** Follow structured debugging: check components, verify assumptions, isolate issues
**Context:** User context switching required checking multiple layers
**Impact:** Efficient root cause identification
**Prevention:** Use debugging checklists and log comprehensive investigation steps

### Hypothesis Testing
**Pattern:** Form and test specific hypotheses during debugging
**Context:** Initial API key switching hypothesis was incorrect
**Impact:** Focused investigation on actual root cause
**Prevention:** Document hypotheses and test results for future reference

## Code Quality Improvements

### Import Management
**Pattern:** Verify all required imports are present after code changes
**Context:** Missing `useApiKey` import in updated hook
**Impact:** TypeScript compilation errors
**Prevention:** Run type checking after all code modifications

### Error Message Clarity
**Pattern:** Provide specific error messages for different failure modes
**Context:** Generic error handling masked specific issues
**Impact:** Delayed debugging due to unclear error information
**Prevention:** Implement granular error handling with descriptive messages

## Architectural Insights

### CQRS/ES Data Flow
**Pattern:** Understand complete event flow from command to projection
**Context:** User creation involves multiple system components
**Impact:** Issues can occur at any point in the flow
**Prevention:** Document and test complete event-driven workflows

### Multi-Tenant Considerations
**Pattern:** Verify tenant isolation in all operations
**Context:** User creation requires proper tenant assignment
**Impact:** Data isolation failures
**Prevention:** Test tenant boundaries in all multi-tenant operations

## Development Workflow

### Memory Bank Maintenance
**Pattern:** Update reflection logs immediately after debugging sessions
**Context:** Continuous improvement protocol requirements
**Impact:** Loss of debugging insights and learnings
**Prevention:** Integrate reflection logging into debugging workflow

### Test-Driven Development
**Pattern:** Write tests for all new and modified code
**Context:** Recent fixes lacked comprehensive test coverage
**Impact:** Potential regressions in fixed functionality
**Prevention:** Implement tests before marking fixes complete

## Performance Considerations

### Query Optimization
**Pattern:** Use appropriate endpoints for specific data requirements
**Context:** `/api/users/self` vs `/api/users/list` for current user data
**Impact:** Unnecessary data transfer and processing
**Prevention:** Choose minimal viable queries for each use case

### Caching Strategy
**Pattern:** Implement appropriate cache TTL for different data types
**Context:** User data caching with 5-minute TTL
**Impact:** Balance between data freshness and performance
**Prevention:** Document caching decisions and monitor cache hit rates
