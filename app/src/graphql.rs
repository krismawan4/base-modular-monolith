use std::sync::Arc;
use async_graphql::{Context, EmptySubscription, InputObject, Object, Schema, SimpleObject};
use axum::{
    extract::Extension,
    http::{header, HeaderMap},
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use users::DynUsersClient;

// ---------------------------------------------------------------------------
// 1. GraphQL Types & Inputs
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct SystemInfo {
    pub architecture: String,
    pub users_mode: String,
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self {
            architecture: "Modular Monolith".to_string(),
            users_mode: "In-Process (Modular Monolith)".to_string(),
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
pub struct SystemStatusType {
    pub service_name:     String,
    pub version:          String,
    pub architecture:     String,
    pub users_mode:       String,
    pub graphql_endpoint: String,
    pub message:          String,
}

#[derive(Clone)]
pub struct Token(pub String);

#[derive(SimpleObject, Clone, Debug)]
pub struct UserType {
    pub id:         String,
    pub name:       String,
    pub email:      String,
    pub role:       String,
    pub created_at: String,
}

impl From<users::UserDto> for UserType {
    fn from(dto: users::UserDto) -> Self {
        Self {
            id:         dto.id,
            name:       dto.name,
            email:      dto.email,
            role:       dto.role,
            created_at: dto.created_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
pub struct AuthPayloadType {
    pub token:      String,
    pub token_type: String,
    pub user:       UserType,
}

#[derive(InputObject)]
pub struct LoginGqlInput {
    pub email:    String,
    pub password: String,
}

#[derive(InputObject)]
pub struct CreateUserGqlInput {
    pub name:     String,
    pub email:    String,
    pub password: Option<String>,
    pub role:     Option<String>,
}

#[derive(InputObject)]
pub struct UpdateUserGqlInput {
    pub name:     Option<String>,
    pub email:    Option<String>,
    pub password: Option<String>,
    pub role:     Option<String>,
}

// ---------------------------------------------------------------------------
// 2. Query Root
// ---------------------------------------------------------------------------

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Endpoint testing sederhana
    async fn hello(&self) -> &str {
        "Hello from Axum GraphQL!"
    }

    /// Mengambil status sistem backend dan mode arsitektur secara dinamis
    async fn system_status(&self, ctx: &Context<'_>) -> async_graphql::Result<SystemStatusType> {
        let info = ctx.data::<Arc<SystemInfo>>()?;
        let message = format!("{} • {}", info.architecture, info.users_mode);
        Ok(SystemStatusType {
            service_name:     "ModularBase API".to_string(),
            version:          env!("CARGO_PKG_VERSION").to_string(),
            architecture:     info.architecture.clone(),
            users_mode:       info.users_mode.clone(),
            graphql_endpoint: "/graphql".to_string(),
            message,
        })
    }

    /// Mengambil profile user yang sedang login (memvalidasi JWT Bearer dari context)
    async fn me(&self, ctx: &Context<'_>) -> async_graphql::Result<Option<UserType>> {
        let users_client = ctx.data::<DynUsersClient>()?;
        let jwt_secret = ctx.data::<Arc<String>>()?;

        if let Ok(Token(token)) = ctx.data::<Token>() {
            if let Ok(claims) = auth::decode_jwt(token, jwt_secret) {
                if let Ok(dto) = users_client.get_user_by_id(&claims.sub).await {
                    return Ok(Some(UserType::from(dto)));
                }
            }
        }

        Ok(None)
    }

    /// Mengambil semua user yang terdaftar
    async fn users(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<UserType>> {
        let users_client = ctx.data::<DynUsersClient>()?;
        let list = users_client
            .list_users()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(list.into_iter().map(UserType::from).collect())
    }

    /// Mengambil data user berdasarkan ID
    async fn user_by_id(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::Result<Option<UserType>> {
        let users_client = ctx.data::<DynUsersClient>()?;
        match users_client.get_user_by_id(&id).await {
            Ok(dto) => Ok(Some(UserType::from(dto))),
            Err(users::UserError::NotFound) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(e.to_string())),
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Mutation Root
// ---------------------------------------------------------------------------

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Login user dan dapatkan token JWT Bearer
    async fn login(
        &self,
        ctx: &Context<'_>,
        input: LoginGqlInput,
    ) -> async_graphql::Result<AuthPayloadType> {
        let auth_client = ctx.data::<auth::DynAuthClient>()?;
        let jwt_secret = ctx.data::<Arc<String>>()?;

        let user = auth_client
            .authenticate_user(&input.email, &input.password)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let token = auth::create_jwt(&user.id, &user.email, jwt_secret)
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(AuthPayloadType {
            token,
            token_type: "Bearer".to_string(),
            user: UserType {
                id:         user.id,
                name:       user.name,
                email:      user.email,
                role:       user.role.name,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        })
    }

    /// Membuat user baru
    async fn create_user(
        &self,
        ctx: &Context<'_>,
        input: CreateUserGqlInput,
    ) -> async_graphql::Result<UserType> {
        let users_client = ctx.data::<DynUsersClient>()?;
        let created = users_client
            .create_user(users::CreateUserInput {
                name:     input.name,
                email:    input.email,
                password: input.password,
                role:     input.role,
            })
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(UserType::from(created))
    }

    /// Mengubah data user berdasarkan ID
    async fn update_user(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateUserGqlInput,
    ) -> async_graphql::Result<UserType> {
        let users_client = ctx.data::<DynUsersClient>()?;
        let updated = users_client
            .update_user(
                &id,
                users::UpdateUserInput {
                    name:     input.name,
                    email:    input.email,
                    password: input.password,
                    role:     input.role,
                },
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(UserType::from(updated))
    }

    /// Menghapus user berdasarkan ID
    async fn delete_user(&self, ctx: &Context<'_>, id: String) -> async_graphql::Result<bool> {
        let users_client = ctx.data::<DynUsersClient>()?;
        users_client
            .delete_user(&id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(true)
    }
}

// ---------------------------------------------------------------------------
// 4. Schema & Router
// ---------------------------------------------------------------------------

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build_schema(
    users_client: DynUsersClient,
    auth_client:  auth::DynAuthClient,
    jwt_secret:   Arc<String>,
    system_info:  Arc<SystemInfo>,
) -> AppSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(users_client)
        .data(auth_client)
        .data(jwt_secret)
        .data(system_info)
        .finish()
}

pub async fn graphql_playground() -> impl IntoResponse {
    Html(
        async_graphql::http::GraphiQLSource::build()
            .endpoint("/graphql")
            .finish(),
    )
}

pub async fn graphql_handler(
    Extension(schema): Extension<Arc<AppSchema>>,
    headers: HeaderMap,
    Json(mut req): Json<async_graphql::Request>,
) -> Json<async_graphql::Response> {
    if let Some(auth_val) = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        if let Some(token) = auth_val.strip_prefix("Bearer ") {
            req = req.data(Token(token.trim().to_string()));
        }
    }

    Json(schema.execute(req).await)
}

pub fn graphql_routes(
    users_client: DynUsersClient,
    auth_client:  auth::DynAuthClient,
    jwt_secret:   Arc<String>,
    system_info:  Arc<SystemInfo>,
) -> Router {
    let schema = Arc::new(build_schema(users_client, auth_client, jwt_secret, system_info));
    Router::new()
        .route("/graphql", get(graphql_playground).post(graphql_handler))
        .layer(Extension(schema))
}
