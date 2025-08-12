use axum::http::StatusCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthRole {
    PlatformAdmin,
    TenantAdmin,
    Pilot,
}

pub fn parse_role(s: &str) -> Option<AuthRole> {
    match s {
        "PlatformAdmin" | "ROLE_PLATFORM_ADMIN" => Some(AuthRole::PlatformAdmin),
        "TenantAdmin" | "ROLE_TENANT_ADMIN" => Some(AuthRole::TenantAdmin),
        "Pilot" | "ROLE_PILOT" => Some(AuthRole::Pilot),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub enum Requirement {
    PlatformAdminOnly,
    SelfOrTenantAdmin {
        target_user_id: String,
        target_tenant_id: Option<String>,
    },
}

pub fn authorize(
    ctx_user_id: &str,
    ctx_tenant_id: &Option<String>,
    ctx_role: AuthRole,
    req: Requirement,
) -> Result<(), StatusCode> {
    match req {
        Requirement::PlatformAdminOnly => {
            if ctx_role == AuthRole::PlatformAdmin {
                Ok(())
            } else {
                Err(StatusCode::FORBIDDEN)
            }
        }
        Requirement::SelfOrTenantAdmin {
            target_user_id,
            target_tenant_id,
        } => {
            if ctx_role == AuthRole::PlatformAdmin {
                return Ok(());
            }
            if target_user_id == ctx_user_id {
                return Ok(());
            }
            if ctx_role == AuthRole::TenantAdmin
                && ctx_tenant_id.is_some()
                && ctx_tenant_id == &target_tenant_id
            {
                return Ok(());
            }
            Err(StatusCode::FORBIDDEN)
        }
    }
}
