# Raw Reflection Log

This file contains detailed, timestamped entries from debugging sessions and problem-solving activities. Entries are consolidated into `consolidated_learnings.md` periodically.

## [2025-01-XX] User Context Switching Bug Investigation

**Task:** Investigate and fix user context switching after user creation

**Problem Analysis:**
- User reported that after creating a new user "pa", dashboard greets as "pa" instead of current user "magne"
- Initial hypothesis: Automatic API key switching after user creation
- Investigation revealed: `useUserSelf` hook was incorrectly implemented

**Root Cause Found:**
- `useUserSelf` hook was calling `api.listUsers(1, 0).then((res) => res.data[0])`
- This gets the first user from the users list, not the current authenticated user
- When new user "pa" was created, it became the first user in the list ordering
- Dashboard displayed `res.data[0]` (newly created user) instead of current user

**Debugging Steps:**
1. Checked UserCreateForm - no API key switching logic found
2. Checked ApiKeyContext - no automatic switching logic
3. Checked backend register_user - no auto API key generation
4. Found `useUserSelf` hook implementation issue
5. Verified `/api/users/self` endpoint exists and works correctly

**Solution Implemented:**
- Updated `useUserSelf` to properly call `/api/users/self` endpoint
- Added proper Authorization header with current API key
- Added missing `useApiKey` import
- Fixed user context to remain stable across user creation operations

**Testing:**
- Verified bootstrap user creation works
- Verified authenticated user creation works
- Confirmed dashboard shows correct user after creating new users
- All existing functionality preserved

**Learnings:**
- Always verify hook implementations call correct endpoints
- User context stability is critical for multi-user applications
- Frontend state management can be affected by incorrect API calls
- Proper Authorization headers are essential for user-specific endpoints

**Improvements Identified:**
- Add integration tests for user creation flow
- Add unit tests for `useUserSelf` hook
- Consider adding user context validation in ApiKeyContext
- Document user context management patterns

---

## [2025-01-XX] Role Value Mapping Investigation

**Task:** Fix 400 Bad Request errors when creating users

**Problem Analysis:**
- User creation failing with 400 errors
- Tested with `initial_role: 0` (expecting PlatformAdmin)
- Bootstrap creation worked, authenticated creation failed

**Root Cause Found:**
- Frontend using role values 0, 1, 2
- Protobuf expecting role values 1, 2, 3
- Mismatch causing validation failures in backend

**Debugging Steps:**
1. Checked protobuf role enum values
2. Verified frontend role constants
3. Tested with corrected values (1, 2, 3)
4. Confirmed backend validation logic

**Solution Implemented:**
- Updated UserCreateForm role values to match protobuf:
  - `ROLE_PLATFORM_ADMIN` = 1 (was 0)
  - `ROLE_TENANT_ADMIN` = 2 (was 1)
  - `ROLE_PILOT` = 3 (was 2)
- Updated all related logic (defaults, filtering, validation)
- Fixed role comparison logic in components

**Testing:**
- Verified PlatformAdmin creation works (bootstrap)
- Verified TenantAdmin creation works (authenticated)
- Confirmed role filtering works in UI
- All role-based permissions functioning correctly

**Learnings:**
- Always verify enum values between frontend and backend
- Protobuf enum numbering can cause subtle bugs
- Role-based UI filtering depends on correct value mapping
- Test both bootstrap and authenticated code paths

**Improvements Identified:**
- Add type safety for role values
- Create shared constants file for role mappings
- Add validation tests for role value ranges
- Document role enum mapping requirements

---

## [2025-01-XX] Database Reset Script Development

**Task:** Create SQLx-based database reset script

**Problem Analysis:**
- Need safe way to reset database for development/testing
- Existing scripts used cargo run, needed pure SQLx approach
- Required proper error handling and safety checks

**Solution Implemented:**
- Created `scripts/reset-database.sh` with SQLx integration
- Uses `sqlx migrate run --source` for proper migrations
- Includes safety confirmations and comprehensive logging
- Handles Docker infrastructure startup if needed
- Updates SQLx metadata after migrations

**Testing:**
- Verified database reset works correctly
- Confirmed migrations run in proper order
- Tested SQLx metadata updates
- Validated schema integrity checks

**Learnings:**
- SQLx migrate commands need explicit source paths
- Docker exec requires proper quoting for SQL commands
- Safety confirmations prevent accidental data loss
- Comprehensive logging improves debugging

**Improvements Identified:**
- Add dry-run option for migration preview
- Include data seeding options
- Add selective table reset capabilities
- Create integration tests for the script
